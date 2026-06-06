use super::*;

pub(super) fn component_movable_node_ids(
    fragment: &crate::MoleculeFragment,
    component: &ComponentSelection,
) -> Vec<String> {
    let mut node_ids: BTreeSet<String> = component.node_ids.iter().cloned().collect();
    node_ids.extend(component.label_node_ids.iter().cloned());
    for bond_id in &component.bond_ids {
        let Some(bond) = fragment.bonds.iter().find(|bond| &bond.id == bond_id) else {
            continue;
        };
        node_ids.insert(bond.begin.clone());
        node_ids.insert(bond.end.clone());
    }
    node_ids.into_iter().collect()
}

#[derive(Clone, Copy)]
pub(super) enum AlignAxis {
    XMin,
    XMax,
    XCenter,
    YMin,
    YMax,
    YCenter,
}

#[derive(Clone, Copy)]
pub(super) enum DistributeAxis {
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy)]
pub(super) enum FlipAxis {
    Horizontal,
    Vertical,
}

pub(super) fn align_items(items: &mut [SelectionArrangeItem], axis: AlignAxis) -> bool {
    if items.len() < 2 {
        return false;
    }
    let target = match axis {
        AlignAxis::XMin => items
            .iter()
            .map(|item| item.bounds.min_x)
            .fold(f64::INFINITY, f64::min),
        AlignAxis::XMax => items
            .iter()
            .map(|item| item.bounds.max_x)
            .fold(f64::NEG_INFINITY, f64::max),
        AlignAxis::XCenter => {
            let min_x = items
                .iter()
                .map(|item| item.bounds.min_x)
                .fold(f64::INFINITY, f64::min);
            let max_x = items
                .iter()
                .map(|item| item.bounds.max_x)
                .fold(f64::NEG_INFINITY, f64::max);
            (min_x + max_x) * 0.5
        }
        AlignAxis::YMin => items
            .iter()
            .map(|item| item.bounds.min_y)
            .fold(f64::INFINITY, f64::min),
        AlignAxis::YMax => items
            .iter()
            .map(|item| item.bounds.max_y)
            .fold(f64::NEG_INFINITY, f64::max),
        AlignAxis::YCenter => {
            let min_y = items
                .iter()
                .map(|item| item.bounds.min_y)
                .fold(f64::INFINITY, f64::min);
            let max_y = items
                .iter()
                .map(|item| item.bounds.max_y)
                .fold(f64::NEG_INFINITY, f64::max);
            (min_y + max_y) * 0.5
        }
    };

    let mut changed = false;
    for item in items {
        let (dx, dy) = match axis {
            AlignAxis::XMin => (target - item.bounds.min_x, 0.0),
            AlignAxis::XMax => (target - item.bounds.max_x, 0.0),
            AlignAxis::XCenter => (target - item.bounds.center_x(), 0.0),
            AlignAxis::YMin => (0.0, target - item.bounds.min_y),
            AlignAxis::YMax => (0.0, target - item.bounds.max_y),
            AlignAxis::YCenter => (0.0, target - item.bounds.center_y()),
        };
        changed |= item.translate(dx, dy);
    }
    changed
}

pub(super) fn distribute_items(items: &mut [SelectionArrangeItem], axis: DistributeAxis) -> bool {
    if items.len() < 3 {
        return false;
    }
    match axis {
        DistributeAxis::Horizontal => items.sort_by(|a, b| {
            a.bounds
                .min_x
                .total_cmp(&b.bounds.min_x)
                .then(a.bounds.max_x.total_cmp(&b.bounds.max_x))
        }),
        DistributeAxis::Vertical => items.sort_by(|a, b| {
            a.bounds
                .min_y
                .total_cmp(&b.bounds.min_y)
                .then(a.bounds.max_y.total_cmp(&b.bounds.max_y))
        }),
    }
    let span_min = match axis {
        DistributeAxis::Horizontal => items.first().unwrap().bounds.min_x,
        DistributeAxis::Vertical => items.first().unwrap().bounds.min_y,
    };
    let span_max = match axis {
        DistributeAxis::Horizontal => items.last().unwrap().bounds.max_x,
        DistributeAxis::Vertical => items.last().unwrap().bounds.max_y,
    };
    let occupied: f64 = items
        .iter()
        .map(|item| match axis {
            DistributeAxis::Horizontal => item.bounds.width(),
            DistributeAxis::Vertical => item.bounds.height(),
        })
        .sum();
    let gap = (span_max - span_min - occupied) / (items.len() - 1) as f64;
    let mut cursor = span_min;
    let mut changed = false;
    for item in items {
        let (dx, dy) = match axis {
            DistributeAxis::Horizontal => {
                let dx = cursor - item.bounds.min_x;
                cursor += item.bounds.width() + gap;
                (dx, 0.0)
            }
            DistributeAxis::Vertical => {
                let dy = cursor - item.bounds.min_y;
                cursor += item.bounds.height() + gap;
                (0.0, dy)
            }
        };
        changed |= item.translate(dx, dy);
    }
    changed
}

pub(super) fn flip_items(items: &mut [SelectionArrangeItem], axis: FlipAxis) -> bool {
    if items.is_empty() {
        return false;
    }
    let mut bounds = items[0].bounds;
    for item in items.iter().skip(1) {
        bounds.include_bounds(item.bounds);
    }
    let pivot = match axis {
        FlipAxis::Horizontal => bounds.center_x(),
        FlipAxis::Vertical => bounds.center_y(),
    };
    let mut changed = false;
    for item in items {
        match axis {
            FlipAxis::Horizontal => changed |= item.flip_horizontal(pivot),
            FlipAxis::Vertical => changed |= item.flip_vertical(pivot),
        }
    }
    changed
}

impl SelectionArrangeItem {
    fn translate(&mut self, dx: f64, dy: f64) -> bool {
        if dx.abs() <= crate::EPSILON && dy.abs() <= crate::EPSILON {
            return false;
        }
        self.bounds.min_x += dx;
        self.bounds.max_x += dx;
        self.bounds.min_y += dy;
        self.bounds.max_y += dy;
        true
    }

    fn flip_horizontal(&mut self, pivot_x: f64) -> bool {
        let min_x = 2.0 * pivot_x - self.bounds.max_x;
        let max_x = 2.0 * pivot_x - self.bounds.min_x;
        let changed = (self.bounds.min_x - min_x).abs() > crate::EPSILON
            || (self.bounds.max_x - max_x).abs() > crate::EPSILON
            || self.node_ids.len() > 1;
        self.bounds.min_x = min_x;
        self.bounds.max_x = max_x;
        self.mirror_x = Some(pivot_x);
        changed
    }

    fn flip_vertical(&mut self, pivot_y: f64) -> bool {
        let min_y = 2.0 * pivot_y - self.bounds.max_y;
        let max_y = 2.0 * pivot_y - self.bounds.min_y;
        let changed = (self.bounds.min_y - min_y).abs() > crate::EPSILON
            || (self.bounds.max_y - max_y).abs() > crate::EPSILON
            || self.node_ids.len() > 1;
        self.bounds.min_y = min_y;
        self.bounds.max_y = max_y;
        self.mirror_y = Some(pivot_y);
        changed
    }
}

pub(super) fn apply_arrange_items_to_document(engine: &mut Engine, items: &[SelectionArrangeItem]) {
    for item in items {
        for object_id in &item.text_object_ids {
            let Some(object) = engine
                .state
                .document
                .objects
                .iter_mut()
                .find(|object| object.id == *object_id)
            else {
                continue;
            };
            let Some(current_bounds) = text_object_world_bounds(object) else {
                continue;
            };
            object.transform.translate = [
                round2(object.transform.translate[0] + item.bounds.min_x - current_bounds[0]),
                round2(object.transform.translate[1] + item.bounds.min_y - current_bounds[1]),
            ];
        }
    }

    let stroke_width = engine.options.bond_stroke_world_pt().value();
    let Some(mut entry) = engine.state.document.editable_fragment_mut() else {
        return;
    };
    let object_translate = entry.object.transform.translate;
    for item in items {
        for node_id in &item.node_ids {
            let Some(node) = entry
                .fragment
                .nodes
                .iter_mut()
                .find(|node| node.id == *node_id)
            else {
                continue;
            };
            let original_world = Point::new(
                object_translate[0] + node.position[0],
                object_translate[1] + node.position[1],
            );
            let scale_x = if item.original_bounds.width() <= crate::EPSILON {
                0.0
            } else {
                (original_world.x - item.original_bounds.min_x) / item.original_bounds.width()
            };
            let scale_y = if item.original_bounds.height() <= crate::EPSILON {
                0.0
            } else {
                (original_world.y - item.original_bounds.min_y) / item.original_bounds.height()
            };
            let mut next_world = Point::new(
                item.bounds.min_x + item.bounds.width() * scale_x,
                item.bounds.min_y + item.bounds.height() * scale_y,
            );
            if let Some(pivot_x) = item.mirror_x {
                next_world.x = 2.0 * pivot_x - original_world.x;
            }
            if let Some(pivot_y) = item.mirror_y {
                next_world.y = 2.0 * pivot_y - original_world.y;
            }
            node.position = [
                round2(next_world.x - object_translate[0]),
                round2(next_world.y - object_translate[1]),
            ];
        }
    }
    refresh_attached_node_label_geometry_for_all_nodes(
        entry.fragment,
        object_translate,
        stroke_width,
    );
    entry.update_bounds();
    drop(entry);
    engine.refresh_symbol_chemistry();
}
