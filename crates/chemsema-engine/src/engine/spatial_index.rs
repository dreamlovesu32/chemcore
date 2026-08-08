use super::*;
use serde::{Deserialize, Serialize};

const CELL_SIZE_PT: f64 = 96.0;
const MAX_ENUMERATED_CELLS: u64 = 4096;

#[derive(Debug, Clone, Default)]
pub(super) struct SceneSpatialIndex {
    revision: u64,
    cells: BTreeMap<(i32, i32), BTreeSet<String>>,
    global: BTreeSet<String>,
    bounds: BTreeMap<String, [f64; 4]>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialQueryResult {
    pub revision: u64,
    pub bounds: [f64; 4],
    pub entity_ids: Vec<String>,
}

impl SceneSpatialIndex {
    fn build(revision: u64, document: &ChemSemaDocument, primitives: &[RenderPrimitive]) -> Self {
        let mut bounds = BTreeMap::<String, [f64; 4]>::new();
        for primitive in primitives {
            let Some(id) = primitive.object_id() else {
                continue;
            };
            let Some(next) = crate::render_primitive_bounds(primitive) else {
                continue;
            };
            bounds
                .entry(id.to_string())
                .and_modify(|current| *current = union_bounds(*current, next))
                .or_insert(next);
        }
        for object in document.scene_objects() {
            if let Some(next) = super::select::object_selection_bounds_for_render(document, object)
            {
                bounds
                    .entry(object.id.clone())
                    .and_modify(|current| *current = union_bounds(*current, next))
                    .or_insert(next);
            }
        }
        let mut cells = BTreeMap::<(i32, i32), BTreeSet<String>>::new();
        let mut global = BTreeSet::new();
        for (id, bounds) in &bounds {
            if let Some(object_cells) = cells_for_bounds(*bounds) {
                for cell in object_cells {
                    cells.entry(cell).or_default().insert(id.clone());
                }
            } else {
                // A single extremely large entity must not expand the grid
                // without bound. Keep it in a small always-checked set.
                global.insert(id.clone());
            }
        }
        Self {
            revision,
            cells,
            global,
            bounds,
        }
    }

    fn query(&self, bounds: [f64; 4]) -> Vec<String> {
        if !bounds.iter().all(|value| value.is_finite()) {
            return Vec::new();
        }
        let mut candidates = self.global.clone();
        if let Some(query_cells) = cells_for_bounds(bounds) {
            for cell in query_cells {
                if let Some(ids) = self.cells.get(&cell) {
                    candidates.extend(ids.iter().cloned());
                }
            }
        } else {
            // Huge queries are cheaper and safer as an exact scan than as a
            // potentially billions-of-cells allocation.
            candidates.extend(self.bounds.keys().cloned());
        }
        candidates
            .into_iter()
            .filter(|id| {
                self.bounds
                    .get(id)
                    .is_some_and(|candidate| intersects(*candidate, bounds))
            })
            .collect()
    }
}

impl Engine {
    pub fn spatial_query(&self, bounds: [f64; 4]) -> SpatialQueryResult {
        let needs_rebuild = self
            .spatial_index
            .borrow()
            .as_ref()
            .is_none_or(|index| index.revision != self.revision);
        if needs_rebuild {
            let primitives = self.render_list();
            *self.spatial_index.borrow_mut() = Some(SceneSpatialIndex::build(
                self.revision,
                &self.state.document,
                &primitives,
            ));
        }
        let entity_ids = self
            .spatial_index
            .borrow()
            .as_ref()
            .map(|index| index.query(bounds))
            .unwrap_or_default();
        SpatialQueryResult {
            revision: self.revision,
            bounds,
            entity_ids,
        }
    }
}

fn cells_for_bounds(bounds: [f64; 4]) -> Option<Vec<(i32, i32)>> {
    if !bounds.iter().all(|value| value.is_finite()) {
        return None;
    }
    let min_x = (bounds[0].min(bounds[2]) / CELL_SIZE_PT).floor() as i32;
    let max_x = (bounds[0].max(bounds[2]) / CELL_SIZE_PT).floor() as i32;
    let min_y = (bounds[1].min(bounds[3]) / CELL_SIZE_PT).floor() as i32;
    let max_y = (bounds[1].max(bounds[3]) / CELL_SIZE_PT).floor() as i32;
    let width = i64::from(max_x) - i64::from(min_x) + 1;
    let height = i64::from(max_y) - i64::from(min_y) + 1;
    let count = u64::try_from(width).ok().and_then(|width| {
        u64::try_from(height)
            .ok()
            .map(|height| width.saturating_mul(height))
    })?;
    if count > MAX_ENUMERATED_CELLS {
        return None;
    }
    let mut cells = Vec::with_capacity(count as usize);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            cells.push((x, y));
        }
    }
    Some(cells)
}

fn intersects(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0].min(a[2]) <= b[0].max(b[2])
        && a[0].max(a[2]) >= b[0].min(b[2])
        && a[1].min(a[3]) <= b[1].max(b[3])
        && a[1].max(a[3]) >= b[1].min(b[3])
}

fn union_bounds(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    [
        a[0].min(a[2]).min(b[0].min(b[2])),
        a[1].min(a[3]).min(b[1].min(b[3])),
        a[0].max(a[2]).max(b[0].max(b[2])),
        a[1].max(a[3]).max(b[1].max(b[3])),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spatial_index_rebuilds_by_revision_and_returns_intersecting_scene_entities() {
        let mut engine = Engine::new();
        engine
            .execute_command_json(
                r#"{"type":"add-bond","begin":{"x":80.0,"y":80.0},"end":{"x":128.0,"y":80.0},"order":1,"variant":"single"}"#,
            )
            .expect("bond command succeeds");
        let hit = engine.spatial_query([70.0, 70.0, 140.0, 90.0]);
        assert_eq!(hit.revision, 1);
        assert_eq!(hit.entity_ids, vec!["obj_editor_molecule"]);
        let miss = engine.spatial_query([500.0, 500.0, 510.0, 510.0]);
        assert!(miss.entity_ids.is_empty());
    }

    #[test]
    fn cell_enumeration_is_bounded_for_huge_or_invalid_queries() {
        assert!(cells_for_bounds([0.0, 0.0, 100.0, 100.0]).is_some());
        assert!(cells_for_bounds([-1.0e12, -1.0e12, 1.0e12, 1.0e12]).is_none());
        assert!(cells_for_bounds([f64::NAN, 0.0, 1.0, 1.0]).is_none());
    }
}
