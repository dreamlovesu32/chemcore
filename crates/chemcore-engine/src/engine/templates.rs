use super::text_edit::refresh_attached_node_label_geometry_for_all_nodes;
use super::{EditorCommand, Engine};
use crate::{
    adjacent_directions, angle_between, direction_from_angle, hit_test_bond, hit_test_bond_center,
    hit_test_endpoint, nearest_angle, normalize_angle, Bond, BondAnchor, BondPreview,
    ChemcoreDocument, DoubleBond, DoubleBondPlacement, EndpointHit, Node, Point, PointerEvent,
    BOND_CENTER_HIT_RADIUS, BOND_HIT_RADIUS, DEFAULT_BOND_LENGTH, DRAG_START_THRESHOLD,
    ENDPOINT_HIT_RADIUS, GLOBAL_SNAP_ANGLES,
};

const RING_REUSE_RADIUS: f64 = crate::px_to_cm(5.0);

#[derive(Clone)]
enum TemplateAnchor {
    Endpoint(BondAnchor),
    Center(Point),
    Bond {
        bond_id: String,
        begin_id: String,
        end_id: String,
        begin: Point,
        end: Point,
    },
}

pub(super) struct TemplateDrag {
    start: Point,
    current: Point,
    anchor: TemplateAnchor,
    has_dragged: bool,
}

#[derive(Clone)]
struct RingPlan {
    vertices: Vec<RingVertex>,
    edges: Vec<RingEdge>,
    attach_edges: Vec<(String, usize)>,
}

#[derive(Clone)]
struct RingVertex {
    point: Point,
    node_id: Option<String>,
}

#[derive(Clone)]
struct RingEdge {
    begin: usize,
    end: usize,
    order: u8,
    double_placement: Option<DoubleBondPlacement>,
}

impl Engine {
    pub(super) fn pointer_move_template(&mut self, event: PointerEvent) {
        let point = event.point();
        if let Some(mut drag) = self.template_drag.take() {
            drag.current = point;
            if drag.start.distance(point) >= DRAG_START_THRESHOLD {
                drag.has_dragged = true;
            }
            self.state.overlay.hover_endpoint = None;
            self.state.overlay.hover_bond_center = None;
            self.state.overlay.hover_text_box = None;
            self.state.overlay.preview = None;
            if drag.has_dragged {
                let focus = template_drag_focus_point(&drag.anchor);
                self.state.overlay.preview = Some(BondPreview {
                    start: focus,
                    end: focus,
                });
                self.refresh_template_drag_anchor_overlay(&drag.anchor);
            }
            self.template_drag = Some(drag);
            return;
        }

        self.state.overlay.hover_endpoint = None;
        self.state.overlay.hover_bond_center = None;
        self.state.overlay.hover_text_box = None;
        self.state.overlay.preview = None;
        if let Some(endpoint) = hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
        {
            self.state.overlay.hover_endpoint = Some(endpoint);
            return;
        }
        self.state.overlay.hover_bond_center =
            hit_test_bond_center(&self.state.document, point, BOND_CENTER_HIT_RADIUS);
    }

    pub(super) fn pointer_down_template(&mut self, event: PointerEvent) {
        let point = event.point();
        self.drag = None;
        self.selection_drag = None;
        self.state.selection = crate::SelectionState::default();
        if let Some(endpoint) = hit_test_endpoint(&self.state.document, point, ENDPOINT_HIT_RADIUS)
        {
            self.template_drag = Some(TemplateDrag {
                start: point,
                current: point,
                anchor: TemplateAnchor::Endpoint(BondAnchor {
                    node_id: Some(endpoint.node_id),
                    point: endpoint.point,
                    label_anchor: endpoint.label_anchor,
                }),
                has_dragged: false,
            });
            return;
        }
        if let Some(bond) = hit_test_bond(&self.state.document, point, BOND_HIT_RADIUS) {
            let Some((begin_id, end_id)) = self.bond_node_ids(&bond.bond_id) else {
                return;
            };
            self.template_drag = Some(TemplateDrag {
                start: point,
                current: point,
                anchor: TemplateAnchor::Bond {
                    bond_id: bond.bond_id,
                    begin_id,
                    end_id,
                    begin: bond.begin,
                    end: bond.end,
                },
                has_dragged: false,
            });
            return;
        }
        self.template_drag = Some(TemplateDrag {
            start: point,
            current: point,
            anchor: TemplateAnchor::Center(point),
            has_dragged: false,
        });
    }

    pub(super) fn pointer_up_template(&mut self, event: PointerEvent) {
        let Some(drag) = self.template_drag.take() else {
            return;
        };
        let Some(plan) = self.template_ring_plan(&drag, event.point()) else {
            return;
        };
        let point = event.point();
        let inserted = self.with_command(
            EditorCommand::InsertTemplate {
                template: self.state.tool.template.clone(),
                x: point.x,
                y: point.y,
            },
            |engine| {
                engine.push_undo_snapshot();
                if !engine.insert_ring_plan(plan, false) {
                    engine.undo_stack.pop();
                    return false;
                }
                true
            },
        );
        if !inserted {
            return;
        }
        self.clear_interaction();
        self.pointer_move_template(PointerEvent {
            x: event.x,
            y: event.y,
            button: None,
            alt_key: event.alt_key,
        });
    }

    pub(super) fn template_preview_document(&self) -> Option<ChemcoreDocument> {
        let drag = self.template_drag.as_ref()?;
        if !drag.has_dragged {
            return None;
        }
        let plan = self.template_ring_plan(drag, drag.current)?;
        let mut document = self.state.document.clone();
        insert_ring_plan_into_document(&mut document, plan, true, &mut 0, self)?;
        Some(document)
    }

    fn template_ring_plan(&self, drag: &TemplateDrag, point: Point) -> Option<RingPlan> {
        let ring_size = selected_ring_size(&self.state.tool.template)?;
        let aromatic = self.state.tool.template == "benzene";
        match &drag.anchor {
            TemplateAnchor::Endpoint(anchor) => {
                let angle = if drag.has_dragged {
                    nearest_angle(angle_between(anchor.point, point), GLOBAL_SNAP_ANGLES)
                } else {
                    endpoint_click_ring_axis_angle(&self.state.document, anchor)
                };
                Some(endpoint_ring_plan(ring_size, anchor, angle, aromatic))
            }
            TemplateAnchor::Center(center) => {
                if drag.has_dragged {
                    let angle = nearest_angle(angle_between(*center, point), GLOBAL_SNAP_ANGLES);
                    Some(endpoint_ring_plan(
                        ring_size,
                        &BondAnchor {
                            node_id: None,
                            point: *center,
                            label_anchor: None,
                        },
                        angle,
                        aromatic,
                    ))
                } else {
                    Some(centered_ring_plan(ring_size, *center, 270.0, aromatic))
                }
            }
            TemplateAnchor::Bond {
                bond_id,
                begin_id,
                end_id,
                begin,
                end,
            } => {
                let side = if drag.has_dragged {
                    side_for_point(*begin, *end, point).unwrap_or(1.0)
                } else {
                    self.preferred_ring_side_for_bond(bond_id, *begin, *end)
                };
                Some(fused_bond_ring_plan(
                    ring_size,
                    aromatic,
                    begin_id.clone(),
                    end_id.clone(),
                    *begin,
                    *end,
                    side,
                ))
            }
        }
    }

    fn insert_ring_plan(&mut self, plan: RingPlan, preview: bool) -> bool {
        let mut ignored = 0;
        let mut document = self.state.document.clone();
        let changed =
            insert_ring_plan_into_document(&mut document, plan, preview, &mut ignored, self)
                .unwrap_or(false);
        if changed {
            self.state.document = document;
            self.next_id = self.infer_next_id();
        }
        changed
    }

    fn refresh_template_drag_anchor_overlay(&mut self, anchor: &TemplateAnchor) {
        match anchor {
            TemplateAnchor::Endpoint(anchor) => {
                let node_id = anchor
                    .node_id
                    .clone()
                    .unwrap_or_else(|| "__template_anchor".to_string());
                self.state.overlay.hover_endpoint = Some(EndpointHit {
                    node_id,
                    point: anchor.point,
                    distance: 0.0,
                    label_anchor: anchor.label_anchor.clone(),
                });
            }
            TemplateAnchor::Center(point) => {
                self.state.overlay.hover_endpoint = Some(EndpointHit {
                    node_id: "__template_anchor".to_string(),
                    point: *point,
                    distance: 0.0,
                    label_anchor: None,
                });
            }
            TemplateAnchor::Bond { begin, end, .. } => {
                let center = Point::new((begin.x + end.x) * 0.5, (begin.y + end.y) * 0.5);
                self.state.overlay.hover_bond_center =
                    hit_test_bond_center(&self.state.document, center, BOND_CENTER_HIT_RADIUS);
            }
        }
    }

    fn bond_node_ids(&self, bond_id: &str) -> Option<(String, String)> {
        let entry = self.state.document.editable_fragment()?;
        let bond = entry
            .fragment
            .bonds
            .iter()
            .find(|bond| bond.id == bond_id)?;
        Some((bond.begin.clone(), bond.end.clone()))
    }

    fn preferred_ring_side_for_bond(&self, bond_id: &str, begin: Point, end: Point) -> f64 {
        let left_count = self.bond_substituent_side_count(bond_id, begin, end, 1.0);
        let right_count = self.bond_substituent_side_count(bond_id, begin, end, -1.0);
        if left_count < right_count {
            return 1.0;
        }
        if right_count < left_count {
            return -1.0;
        }
        let ring_size = selected_ring_size(&self.state.tool.template).unwrap_or(6);
        let left_score = self.ring_reuse_score(&fused_bond_ring_plan(
            ring_size,
            false,
            String::new(),
            String::new(),
            begin,
            end,
            1.0,
        ));
        let right_score = self.ring_reuse_score(&fused_bond_ring_plan(
            ring_size,
            false,
            String::new(),
            String::new(),
            begin,
            end,
            -1.0,
        ));
        if right_score > left_score {
            -1.0
        } else {
            1.0
        }
    }

    fn bond_substituent_side_count(
        &self,
        bond_id: &str,
        begin: Point,
        end: Point,
        side: f64,
    ) -> usize {
        let Some(entry) = self.state.document.editable_fragment() else {
            return 0;
        };
        let Some((begin_id, end_id)) = self.bond_node_ids(bond_id) else {
            return 0;
        };
        entry
            .fragment
            .bonds
            .iter()
            .filter(|bond| bond.id != bond_id)
            .filter_map(|bond| {
                let other_id = if bond.begin == begin_id {
                    Some(bond.end.as_str())
                } else if bond.end == begin_id {
                    Some(bond.begin.as_str())
                } else if bond.begin == end_id {
                    Some(bond.end.as_str())
                } else if bond.end == end_id {
                    Some(bond.begin.as_str())
                } else {
                    None
                }?;
                entry
                    .fragment
                    .nodes
                    .iter()
                    .find(|node| node.id == other_id)
                    .map(|node| entry.world_point_for_node(node))
            })
            .filter(|point| side_for_point(begin, end, *point).is_some_and(|value| value == side))
            .count()
    }

    fn ring_reuse_score(&self, plan: &RingPlan) -> usize {
        plan.vertices
            .iter()
            .filter(|vertex| self.reusable_node_id_at(vertex.point).is_some())
            .count()
    }

    fn reusable_node_id_at(&self, point: Point) -> Option<String> {
        let entry = self.state.document.editable_fragment()?;
        for node in &entry.fragment.nodes {
            let node_point = entry.world_point_for_node(node);
            if node_point.distance(point) <= RING_REUSE_RADIUS {
                return Some(node.id.clone());
            }
            if let Some(label) = &node.label {
                if let Some(bounds) = label.bbox() {
                    let center = Point::new(
                        entry.object.transform.translate[0] + (bounds[0] + bounds[2]) * 0.5,
                        entry.object.transform.translate[1] + (bounds[1] + bounds[3]) * 0.5,
                    );
                    if center.distance(point) <= RING_REUSE_RADIUS {
                        return Some(node.id.clone());
                    }
                }
            }
        }
        None
    }
}

fn selected_ring_size(template: &str) -> Option<usize> {
    match template {
        "ring-3" => Some(3),
        "ring-4" => Some(4),
        "ring-5" => Some(5),
        "ring-6" | "benzene" => Some(6),
        "ring-7" => Some(7),
        "ring-8" => Some(8),
        _ => None,
    }
}

fn endpoint_click_ring_axis_angle(document: &ChemcoreDocument, anchor: &BondAnchor) -> f64 {
    let Some(node_id) = anchor.node_id.as_deref() else {
        return crate::default_angle_for_anchor(document, anchor);
    };
    let Some(entry) = document.editable_fragment() else {
        return crate::default_angle_for_anchor(document, anchor);
    };
    let directions = adjacent_directions(&entry, node_id);
    if directions.len() == 1 {
        normalize_angle(directions[0] + 180.0)
    } else {
        crate::default_angle_for_anchor(document, anchor)
    }
}

fn endpoint_ring_plan(
    ring_size: usize,
    anchor: &BondAnchor,
    angle: f64,
    aromatic: bool,
) -> RingPlan {
    let direction = direction_from_angle(angle);
    let side = DEFAULT_BOND_LENGTH;
    let radius = side / (2.0 * (std::f64::consts::PI / ring_size as f64).sin());
    let center = anchor.point.translated(direction.scaled(radius));
    let first_vector = crate::Vector::new(anchor.point.x - center.x, anchor.point.y - center.y);
    let vertices = regular_vertices_from_vector(ring_size, center, first_vector, 1.0)
        .into_iter()
        .enumerate()
        .map(|(index, point)| RingVertex {
            point,
            node_id: if index == 0 {
                anchor.node_id.clone()
            } else {
                None
            },
        })
        .collect::<Vec<_>>();
    let edges = ring_edges_for_vertices(&vertices, aromatic, 0);
    RingPlan {
        vertices,
        edges,
        attach_edges: Vec::new(),
    }
}

fn centered_ring_plan(ring_size: usize, center: Point, angle: f64, aromatic: bool) -> RingPlan {
    let side = DEFAULT_BOND_LENGTH;
    let radius = side / (2.0 * (std::f64::consts::PI / ring_size as f64).sin());
    let direction = direction_from_angle(angle);
    let first_vector = direction.scaled(radius);
    let vertices = regular_vertices_from_vector(ring_size, center, first_vector, 1.0)
        .into_iter()
        .map(|point| RingVertex {
            point,
            node_id: None,
        })
        .collect::<Vec<_>>();
    let edges = ring_edges_for_vertices(&vertices, aromatic, 0);
    RingPlan {
        vertices,
        edges,
        attach_edges: Vec::new(),
    }
}

fn fused_bond_ring_plan(
    ring_size: usize,
    aromatic: bool,
    begin_id: String,
    end_id: String,
    begin: Point,
    end: Point,
    side_sign: f64,
) -> RingPlan {
    let side = begin.distance(end).max(DEFAULT_BOND_LENGTH);
    let apothem = side / (2.0 * (std::f64::consts::PI / ring_size as f64).tan());
    let unit = crate::Vector::new((end.x - begin.x) / side, (end.y - begin.y) / side);
    let normal = crate::Vector::new(-unit.y, unit.x).scaled(side_sign);
    let center = Point::new(
        (begin.x + end.x) * 0.5 + normal.x * apothem,
        (begin.y + end.y) * 0.5 + normal.y * apothem,
    );
    let begin_vector = crate::Vector::new(begin.x - center.x, begin.y - center.y);
    let positive = regular_vertices_from_vector(ring_size, center, begin_vector, 1.0);
    let negative = regular_vertices_from_vector(ring_size, center, begin_vector, -1.0);
    let points = if positive
        .get(1)
        .is_some_and(|point| point.distance(end) <= 0.05)
    {
        positive
    } else {
        negative
    };
    let vertices = points
        .into_iter()
        .enumerate()
        .map(|(index, point)| RingVertex {
            point,
            node_id: match index {
                0 if !begin_id.is_empty() => Some(begin_id.clone()),
                1 if !end_id.is_empty() => Some(end_id.clone()),
                _ => None,
            },
        })
        .collect::<Vec<_>>();
    let edges = ring_edges_for_vertices(&vertices, aromatic, 1);
    RingPlan {
        vertices,
        edges,
        attach_edges: Vec::new(),
    }
}

fn regular_vertices_from_vector(
    ring_size: usize,
    center: Point,
    first_vector: crate::Vector,
    direction: f64,
) -> Vec<Point> {
    let step = direction * 2.0 * std::f64::consts::PI / ring_size as f64;
    (0..ring_size)
        .map(|index| {
            let angle = step * index as f64;
            let cos = angle.cos();
            let sin = angle.sin();
            Point::new(
                center.x + first_vector.x * cos - first_vector.y * sin,
                center.y + first_vector.x * sin + first_vector.y * cos,
            )
        })
        .collect()
}

fn ring_edges_for_vertices(
    vertices: &[RingVertex],
    aromatic: bool,
    first_double_edge_index: usize,
) -> Vec<RingEdge> {
    let center = ring_vertices_center(vertices);
    (0..vertices.len())
        .map(|index| {
            let next = (index + 1) % vertices.len();
            let aromatic_double = aromatic && index % 2 == first_double_edge_index % 2;
            RingEdge {
                begin: index,
                end: next,
                order: if aromatic_double { 2 } else { 1 },
                double_placement: aromatic_double.then(|| {
                    inward_double_placement(vertices[index].point, vertices[next].point, center)
                }),
            }
        })
        .collect()
}

fn ring_vertices_center(vertices: &[RingVertex]) -> Point {
    let count = vertices.len().max(1) as f64;
    Point::new(
        vertices.iter().map(|vertex| vertex.point.x).sum::<f64>() / count,
        vertices.iter().map(|vertex| vertex.point.y).sum::<f64>() / count,
    )
}

fn template_drag_focus_point(anchor: &TemplateAnchor) -> Point {
    match anchor {
        TemplateAnchor::Endpoint(anchor) => anchor.point,
        TemplateAnchor::Center(point) => *point,
        TemplateAnchor::Bond { begin, end, .. } => {
            Point::new((begin.x + end.x) * 0.5, (begin.y + end.y) * 0.5)
        }
    }
}

fn inward_double_placement(begin: Point, end: Point, center: Point) -> DoubleBondPlacement {
    match side_for_point(begin, end, center).unwrap_or(1.0) {
        side if side > 0.0 => DoubleBondPlacement::Left,
        _ => DoubleBondPlacement::Right,
    }
}

fn side_for_point(begin: Point, end: Point, point: Point) -> Option<f64> {
    let cross = (end.x - begin.x) * (point.y - begin.y) - (end.y - begin.y) * (point.x - begin.x);
    if cross.abs() <= crate::EPSILON {
        None
    } else if cross > 0.0 {
        Some(1.0)
    } else {
        Some(-1.0)
    }
}

fn insert_ring_plan_into_document(
    document: &mut ChemcoreDocument,
    plan: RingPlan,
    preview: bool,
    preview_counter: &mut usize,
    engine: &Engine,
) -> Option<bool> {
    let stroke_width = template_stroke_width(document, &plan, engine);
    let line_styles = engine.pending_line_styles();
    let line_weights = engine.pending_line_weights();
    let mut node_ids = Vec::new();
    let mut nodes_to_insert = Vec::new();
    let mut changed = false;

    {
        let entry = document.editable_fragment()?;
        let object_translate = entry.object.transform.translate;
        for (index, vertex) in plan.vertices.iter().enumerate() {
            if let Some(node_id) = &vertex.node_id {
                node_ids.push(node_id.clone());
                continue;
            }
            if let Some(node_id) = reusable_node_id_in_entry(&entry, vertex.point) {
                node_ids.push(node_id);
                continue;
            }
            let local = Point::new(
                vertex.point.x - object_translate[0],
                vertex.point.y - object_translate[1],
            );
            let node_id = if preview {
                format!("__preview_ring_node_{index}")
            } else {
                format!("n_{}", engine.next_id + *preview_counter as u64)
            };
            *preview_counter += 1;
            nodes_to_insert.push(Node::carbon(node_id.clone(), local));
            node_ids.push(node_id);
            changed = true;
        }
    }

    let mut entry = document.editable_fragment_mut()?;
    let object_translate = entry.object.transform.translate;
    entry.fragment.nodes.extend(nodes_to_insert);

    for edge in plan.edges {
        changed |= insert_ring_bond(
            &mut entry,
            &node_ids[edge.begin],
            &node_ids[edge.end],
            edge.order,
            edge.double_placement,
            preview,
            preview_counter,
            engine,
            stroke_width,
            line_styles.clone(),
            line_weights.clone(),
        );
    }
    for (existing_id, vertex_index) in plan.attach_edges {
        changed |= insert_ring_bond(
            &mut entry,
            &existing_id,
            &node_ids[vertex_index],
            1,
            None,
            preview,
            preview_counter,
            engine,
            stroke_width,
            line_styles.clone(),
            line_weights.clone(),
        );
    }

    refresh_attached_node_label_geometry_for_all_nodes(
        entry.fragment,
        object_translate,
        stroke_width,
    );
    entry.update_bounds();
    Some(changed)
}

fn template_stroke_width(document: &ChemcoreDocument, plan: &RingPlan, engine: &Engine) -> f64 {
    let fallback = engine.options.bond_stroke_world_cm().value();
    let Some(entry) = document.editable_fragment() else {
        return fallback;
    };
    let style_width = entry
        .object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
        .and_then(|style| {
            style
                .get("strokeWidth")
                .or_else(|| style.get("stroke_width"))
                .and_then(|value| value.as_f64())
        })
        .unwrap_or(fallback);

    let existing_node_ids = plan
        .vertices
        .iter()
        .filter_map(|vertex| vertex.node_id.as_deref())
        .chain(
            plan.attach_edges
                .iter()
                .map(|(node_id, _)| node_id.as_str()),
        )
        .collect::<std::collections::BTreeSet<_>>();

    entry
        .fragment
        .bonds
        .iter()
        .find(|bond| {
            existing_node_ids.contains(bond.begin.as_str())
                || existing_node_ids.contains(bond.end.as_str())
        })
        .map(|bond| {
            if bond.stroke_width > 0.0 {
                bond.stroke_width
            } else {
                style_width
            }
        })
        .unwrap_or(style_width)
}

fn insert_ring_bond(
    entry: &mut crate::EditableFragmentMut<'_>,
    begin_id: &str,
    end_id: &str,
    order: u8,
    double_placement: Option<DoubleBondPlacement>,
    preview: bool,
    preview_counter: &mut usize,
    engine: &Engine,
    stroke_width: f64,
    line_styles: crate::BondLineStyles,
    line_weights: crate::BondLineWeights,
) -> bool {
    if begin_id == end_id
        || entry.fragment.bonds.iter().any(|bond| {
            (bond.begin == begin_id && bond.end == end_id)
                || (bond.begin == end_id && bond.end == begin_id)
        })
    {
        return false;
    }
    let bond_id = if preview {
        let id = format!("__preview_ring_bond_{}", *preview_counter);
        *preview_counter += 1;
        id
    } else {
        let id = format!("b_{}", engine.next_id + *preview_counter as u64);
        *preview_counter += 1;
        id
    };
    entry.fragment.bonds.push(Bond {
        id: bond_id,
        begin: begin_id.to_string(),
        end: end_id.to_string(),
        order: order.max(1),
        double: double_placement.map(|placement| DoubleBond {
            placement,
            center_exit_side: None,
            frozen: false,
        }),
        stereo: None,
        stroke_width,
        bold_width: Some(engine.options.bold_bond_width_world_cm().value()),
        wedge_width: Some(engine.options.wedge_width_world_cm().value()),
        label_clip_margin: Some(engine.options.label_clip_margin_world_cm().value()),
        hash_spacing: Some(engine.options.hash_spacing_world_cm().value()),
        bond_spacing: Some(engine.options.bond_spacing_percent()),
        line_styles,
        line_weights,
        meta: serde_json::Value::Null,
    });
    true
}

fn reusable_node_id_in_entry(entry: &crate::EditableFragment<'_>, point: Point) -> Option<String> {
    for node in &entry.fragment.nodes {
        let node_point = entry.world_point_for_node(node);
        if node_point.distance(point) <= RING_REUSE_RADIUS {
            return Some(node.id.clone());
        }
        if let Some(label) = &node.label {
            if let Some(bounds) = label.bbox() {
                let center = Point::new(
                    entry.object.transform.translate[0] + (bounds[0] + bounds[2]) * 0.5,
                    entry.object.transform.translate[1] + (bounds[1] + bounds[3]) * 0.5,
                );
                if center.distance(point) <= RING_REUSE_RADIUS {
                    return Some(node.id.clone());
                }
            }
        }
    }
    None
}
