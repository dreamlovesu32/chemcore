use super::*;
use crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT;

#[path = "render_objects/arrows.rs"]
mod arrows;
#[path = "render_objects/graphics.rs"]
mod graphics;
#[path = "render_objects/text.rs"]
mod text;

pub(super) use arrows::render_line_object;
pub(super) use graphics::{render_bracket_object, render_shape_object};
pub(super) use text::render_text_object;

fn text_anchor(align: &str) -> String {
    match align {
        "center" => "middle".to_string(),
        "right" => "end".to_string(),
        _ => "start".to_string(),
    }
}

fn fragment_label_font_size(label: &crate::NodeLabel) -> f64 {
    let mut size = label.font_size;
    for run in &label.runs {
        if let Some(run_size) = run.font_size {
            size = Some(size.map_or(run_size, |current| current.max(run_size)));
        }
    }
    size.unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT)
}

fn fragment_label_lines(label: &crate::NodeLabel) -> Vec<String> {
    if !label.lines.is_empty() {
        return label
            .lines
            .iter()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
    }
    if label.text.contains('\n') {
        return label
            .text
            .split('\n')
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect();
    }
    if label.text.trim().is_empty() {
        Vec::new()
    } else {
        vec![label.text.clone()]
    }
}

fn fragment_label_runs_for_line(
    label: &crate::NodeLabel,
    index: usize,
    line: &str,
) -> Vec<LabelRun> {
    if let Some(line_runs) = label.line_runs.get(index) {
        return line_runs.clone();
    }
    if index == 0 && !label.runs.is_empty() && !label.text.contains('\n') && label.lines.is_empty()
    {
        return label.runs.clone();
    }
    vec![LabelRun {
        text: line.to_string(),
        font_family: label.font_family.clone(),
        font_size: label.font_size,
        fill: label.fill.clone(),
        font_weight: None,
        font_style: None,
        underline: None,
        script: None,
    }]
}

fn fragment_label_position_world(label: &crate::NodeLabel, object: &SceneObject) -> Point {
    let position = label.position.unwrap_or([0.0, 0.0]);
    Point::new(
        object.transform.translate[0] + position[0],
        object.transform.translate[1] + position[1],
    )
}

fn polygon_list_bounds(polygons: &[Vec<Point>]) -> Option<(f64, f64, f64, f64)> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut found = false;
    for polygon in polygons {
        for point in polygon {
            found = true;
            min_x = min_x.min(point.x);
            min_y = min_y.min(point.y);
            max_x = max_x.max(point.x);
            max_y = max_y.max(point.y);
        }
    }
    found.then_some((min_x, min_y, max_x, max_y))
}

pub(super) fn render_molecule_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
) {
    let Some(resource_ref) = object.payload.resource_ref.as_ref() else {
        return;
    };
    let Some(resource) = document.resources.get(resource_ref) else {
        return;
    };
    match &resource.data {
        ResourceData::Fragment(fragment)
            if resource.resource_type == "molecule_fragment2d"
                || resource.encoding == "chemcore.molecule.fragment2d" =>
        {
            let node_map: BTreeMap<&str, &Node> = fragment
                .nodes
                .iter()
                .map(|node| (node.id.as_str(), node))
                .collect();
            let stroke = molecule_stroke(document, object);
            let object_id = Some(object.id.clone());
            let contact_kernel =
                build_main_bond_contact_kernel(document, object, &fragment.bonds, &node_map);

            let mut rendered_bonds: Vec<&Bond> = Vec::new();
            for bond in &fragment.bonds {
                render_bond_crossing_knockouts(
                    out,
                    document,
                    object,
                    &rendered_bonds,
                    &node_map,
                    bond,
                    object_id.clone(),
                );
                render_fragment_bond(
                    out,
                    document,
                    object,
                    &contact_kernel,
                    &fragment.bonds,
                    &node_map,
                    bond,
                    &stroke,
                    object_id.clone(),
                );
                rendered_bonds.push(bond);
            }
            render_main_bond_contact_patches(out, &contact_kernel, &stroke, object_id.clone());

            for node in &fragment.nodes {
                render_fragment_label(out, document, object, node, object_id.clone());
                render_fragment_node_invalid_marker(out, object, node, object_id.clone());
            }
        }
        ResourceData::Text(molblock) => {
            render_legacy_molecule_object(out, document, object, molblock);
        }
        _ => {}
    }
}

pub(super) fn render_molecule_object_targets(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
    target_node_ids: &BTreeSet<String>,
    target_bond_ids: &BTreeSet<String>,
) {
    let Some(resource_ref) = object.payload.resource_ref.as_ref() else {
        return;
    };
    let Some(resource) = document.resources.get(resource_ref) else {
        return;
    };
    let ResourceData::Fragment(fragment) = &resource.data else {
        return;
    };
    if resource.resource_type != "molecule_fragment2d"
        && resource.encoding != "chemcore.molecule.fragment2d"
    {
        return;
    }

    let node_map: BTreeMap<&str, &Node> = fragment
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    let stroke = molecule_stroke(document, object);
    let object_id = Some(object.id.clone());
    let mut target_render_bond_ids = BTreeSet::new();
    let mut contact_node_ids = BTreeSet::new();
    for bond in &fragment.bonds {
        let touches_target_node =
            target_node_ids.contains(&bond.begin) || target_node_ids.contains(&bond.end);
        if target_bond_ids.contains(&bond.id) || touches_target_node {
            target_render_bond_ids.insert(bond.id.clone());
            contact_node_ids.insert(bond.begin.clone());
            contact_node_ids.insert(bond.end.clone());
        }
    }
    let contact_kernel = build_main_bond_contact_kernel_for_nodes(
        document,
        object,
        &fragment.bonds,
        &node_map,
        &contact_node_ids,
    );

    let mut rendered_bonds: Vec<&Bond> = Vec::new();
    for bond in &fragment.bonds {
        if target_render_bond_ids.contains(&bond.id) {
            render_bond_crossing_knockouts(
                out,
                document,
                object,
                &rendered_bonds,
                &node_map,
                bond,
                object_id.clone(),
            );
            render_fragment_bond(
                out,
                document,
                object,
                &contact_kernel,
                &fragment.bonds,
                &node_map,
                bond,
                &stroke,
                object_id.clone(),
            );
        }
        rendered_bonds.push(bond);
    }

    for node in &fragment.nodes {
        if target_node_ids.contains(&node.id) {
            render_fragment_label(out, document, object, node, object_id.clone());
            render_fragment_node_invalid_marker(out, object, node, object_id.clone());
        }
    }
}

fn render_bond_crossing_knockouts(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
    previous_bonds: &[&Bond],
    node_map: &BTreeMap<&str, &Node>,
    over_bond: &Bond,
    object_id: Option<String>,
) {
    let Some((over_start, over_end)) = bond_world_segment(object, node_map, over_bond) else {
        return;
    };
    let over_vector = Vector::new(over_end.x - over_start.x, over_end.y - over_start.y);
    if over_vector.length() <= EPSILON {
        return;
    }
    let over_unit = over_vector.normalized();
    let over_normal = Vector::new(-over_unit.y, over_unit.x);
    let over_stroke_width = bond_stroke_width(document, object, over_bond);
    let margin_width = margin_width_for_bond(over_bond, over_stroke_width);
    if margin_width <= EPSILON {
        return;
    }
    let over_width = crossing_bond_visual_width(over_bond, over_start, over_end, over_stroke_width);

    for under_bond in previous_bonds {
        // Bonds are painted in document order. The current bond is the visual
        // upper layer, so its knockout cuts only previously rendered bonds.
        if bonds_share_endpoint(over_bond, under_bond) {
            continue;
        }
        let Some((under_start, under_end)) = bond_world_segment(object, node_map, under_bond)
        else {
            continue;
        };
        let under_vector = Vector::new(under_end.x - under_start.x, under_end.y - under_start.y);
        if under_vector.length() <= EPSILON {
            continue;
        }
        let under_unit = under_vector.normalized();
        let crossing_sin = vector_cross(over_unit, under_unit).abs();
        if crossing_sin <= 0.1 {
            continue;
        }
        let Some(center) =
            interior_segment_intersection(over_start, over_end, under_start, under_end)
        else {
            continue;
        };
        let under_width = crossing_bond_visual_width(
            under_bond,
            under_start,
            under_end,
            bond_stroke_width(document, object, under_bond),
        );
        let half_length = (under_width * 0.5 / crossing_sin) + margin_width;
        let half_width = (over_width * 0.5) + margin_width;
        // A shallow crossing needs a longer cut along the upper bond direction;
        // dividing by sin(theta) keeps the visible gap roughly constant.
        push_bond_knockout_polygon(
            out,
            vec![
                center
                    .translated(over_unit.scaled(-half_length))
                    .translated(over_normal.scaled(-half_width)),
                center
                    .translated(over_unit.scaled(half_length))
                    .translated(over_normal.scaled(-half_width)),
                center
                    .translated(over_unit.scaled(half_length))
                    .translated(over_normal.scaled(half_width)),
                center
                    .translated(over_unit.scaled(-half_length))
                    .translated(over_normal.scaled(half_width)),
            ],
            object_id.clone(),
            over_bond.id.clone(),
        );
    }
}

fn bond_world_segment(
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
) -> Option<(Point, Point)> {
    let begin = world_point(object, node_map.get(bond.begin.as_str()).copied()?);
    let end = world_point(object, node_map.get(bond.end.as_str()).copied()?);
    Some((begin, end))
}

fn bonds_share_endpoint(first: &Bond, second: &Bond) -> bool {
    first.begin == second.begin
        || first.begin == second.end
        || first.end == second.begin
        || first.end == second.end
}

fn interior_segment_intersection(a1: Point, a2: Point, b1: Point, b2: Point) -> Option<Point> {
    let a = Vector::new(a2.x - a1.x, a2.y - a1.y);
    let b = Vector::new(b2.x - b1.x, b2.y - b1.y);
    let denom = vector_cross(a, b);
    if denom.abs() <= EPSILON {
        return None;
    }
    let offset = Vector::new(b1.x - a1.x, b1.y - a1.y);
    let t = vector_cross(offset, b) / denom;
    let u = vector_cross(offset, a) / denom;
    if t <= 1.0e-6 || t >= 1.0 - 1.0e-6 || u <= 1.0e-6 || u >= 1.0 - 1.0e-6 {
        return None;
    }
    Some(Point::new(a1.x + a.x * t, a1.y + a.y * t))
}

fn crossing_bond_visual_width(bond: &Bond, start: Point, end: Point, stroke_width: f64) -> f64 {
    if let Some(stereo_kind) = bond_stereo_kind(bond) {
        return match stereo_kind {
            BondStereoKind::SolidWedgeBegin
            | BondStereoKind::SolidWedgeEnd
            | BondStereoKind::HashedWedgeBegin
            | BondStereoKind::HashedWedgeEnd
            | BondStereoKind::HollowWedgeBegin
            | BondStereoKind::HollowWedgeEnd => {
                solid_wedge_half_width_for_bond(bond, stroke_width) * 2.0
            }
        };
    }

    match bond.order {
        0 | 1 => line_weight_stroke_width_for_bond(bond, stroke_width, bond.line_weights.main),
        2 => {
            let side_mode = bond.double.as_ref().map(|double| double.placement);
            let (first, second) = match side_mode {
                Some(DoubleBondPlacement::Left) | Some(DoubleBondPlacement::Right) => (
                    bond.line_weights.main,
                    outer_line_weight_for_crossing(bond, side_mode),
                ),
                _ => (bond.line_weights.left, bond.line_weights.right),
            };
            let first_width = line_weight_stroke_width_for_bond(bond, stroke_width, first);
            let second_width = line_weight_stroke_width_for_bond(bond, stroke_width, second);
            double_bond_center_distance_for_bond_weights(
                bond,
                start,
                end,
                stroke_width,
                first,
                second,
            ) + 0.5 * (first_width + second_width)
        }
        _ => {
            let offset = triple_bond_offset_distance(start, end, stroke_width);
            let left_width =
                line_weight_stroke_width_for_bond(bond, stroke_width, bond.line_weights.left);
            let right_width =
                line_weight_stroke_width_for_bond(bond, stroke_width, bond.line_weights.right);
            offset * 2.0 + 0.5 * (left_width + right_width)
        }
    }
}

fn outer_line_weight_for_crossing(
    bond: &Bond,
    side_mode: Option<DoubleBondPlacement>,
) -> crate::BondLineWeight {
    match side_mode {
        Some(DoubleBondPlacement::Left) => bond.line_weights.left,
        Some(DoubleBondPlacement::Right) => bond.line_weights.right,
        _ => bond.line_weights.right,
    }
}

fn render_fragment_node_invalid_marker(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    node: &Node,
    object_id: Option<String>,
) {
    if chemical_check_disabled(&node.meta) {
        return;
    }
    if !crate::node_has_charge_symbol_invalid(node) {
        return;
    }
    let center = Point::new(
        object.transform.translate[0] + node.position[0],
        object.transform.translate[1] + node.position[1],
    );
    out.push(RenderPrimitive::Circle {
        role: RenderRole::DocumentDiagnostic,
        object_id,
        node_id: Some(node.id.clone()),
        center,
        radius: crate::ENDPOINT_FOCUS_RADIUS,
        fill: "none".to_string(),
        stroke: "#d32f2f".to_string(),
        stroke_width: 1.0,
    });
}

pub(super) fn render_fragment_label(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
    node: &Node,
    object_id: Option<String>,
) {
    let Some(label) = node.label.as_ref() else {
        return;
    };
    if !label.has_visible_text() {
        return;
    }

    let font_size = fragment_label_font_size(label);
    let text_anchor = text_anchor(label.align.as_deref().unwrap_or("left"));
    let font_family = label.font_family.clone().or_else(|| {
        object
            .style_ref
            .as_ref()
            .and_then(|style_ref| document.styles.get(style_ref))
            .and_then(|style| style_string(style, "fontFamily"))
    });
    let fill = label.fill.clone().or_else(|| {
        object
            .style_ref
            .as_ref()
            .and_then(|style_ref| document.styles.get(style_ref))
            .and_then(|style| style_string(style, "fill"))
    });
    let knockout_polygons = label_polygons_world(node, object);
    if knockout_polygons.is_empty() {
        if let Some(box_value) = label_box_world(node, object) {
            out.push(RenderPrimitive::Rect {
                role: RenderRole::DocumentKnockout,
                object_id: object_id.clone(),
                node_id: Some(node.id.clone()),
                x: box_value.x1,
                y: box_value.y1,
                width: (box_value.x2 - box_value.x1).max(0.0),
                height: (box_value.y2 - box_value.y1).max(0.0),
                fill: Some(document.document.page.background.clone()),
                stroke: None,
                stroke_width: 0.0,
                rx: None,
                ry: None,
                dash_array: Vec::new(),
                fill_gradient: None,
            });
        }
    } else {
        for polygon in knockout_polygons {
            push_label_knockout_polygon(out, polygon, object_id.clone(), node.id.clone());
        }
    }
    if fragment_label_is_invalid(label) {
        let invalid_box = polygon_list_bounds(&label_polygons_world(node, object))
            .map(|(x1, y1, x2, y2)| RectBox { x1, y1, x2, y2 })
            .or_else(|| label_box_world(node, object));
        if let Some(box_value) = invalid_box {
            out.push(RenderPrimitive::Rect {
                role: RenderRole::DocumentDiagnostic,
                object_id: None,
                node_id: None,
                x: box_value.x1,
                y: box_value.y1,
                width: (box_value.x2 - box_value.x1).max(0.0),
                height: (box_value.y2 - box_value.y1).max(0.0),
                fill: Some("none".to_string()),
                stroke: Some("#d32f2f".to_string()),
                stroke_width: 1.0,
                rx: None,
                ry: None,
                dash_array: Vec::new(),
                fill_gradient: None,
            });
        }
    }

    let lines = fragment_label_lines(label);
    if lines.is_empty() {
        return;
    }
    let world_position = fragment_label_position_world(label, object);
    if lines.len() == 1 {
        let primitive = RenderPrimitive::Text {
            role: RenderRole::DocumentText,
            object_id,
            node_id: Some(node.id.clone()),
            x: world_position.x,
            y: world_position.y,
            baseline_offset: Some(font_size * 0.82),
            dominant_baseline: None,
            text: String::new(),
            font_size,
            font_family,
            fill,
            text_anchor: Some(text_anchor),
            line_height: None,
            preserve_lines: false,
            box_width: None,
            runs: fragment_label_runs_for_line(label, 0, &lines[0]),
            rotate: 0.0,
            rotate_center: None,
        };
        out.push(primitive);
        return;
    }

    let label_box = label_box_world(node, object);
    let line_height = label_box
        .map(|box_value| (box_value.y2 - box_value.y1) / lines.len() as f64)
        .unwrap_or(font_size * 1.05);
    let box_top = label_box
        .map(|box_value| box_value.y1)
        .unwrap_or(world_position.y - line_height * 0.82);
    for (index, line) in lines.iter().enumerate() {
        let baseline_y = box_top + line_height * index as f64 + line_height * 0.82;
        push_text_for_node(
            out,
            world_position.x,
            baseline_y,
            Some(line_height * 0.82),
            String::new(),
            font_size,
            font_family.clone(),
            fill.clone(),
            Some(text_anchor.clone()),
            fragment_label_runs_for_line(label, index, line),
            object_id.clone(),
            Some(node.id.clone()),
        );
    }
}

fn fragment_label_is_invalid(label: &crate::NodeLabel) -> bool {
    if chemical_check_disabled(&label.meta) {
        return false;
    }
    label
        .meta
        .get("labelRecognition")
        .and_then(|value| value.get("status"))
        .and_then(serde_json::Value::as_str)
        == Some("invalid")
}

fn chemical_check_disabled(meta: &serde_json::Value) -> bool {
    meta.get("chemicalCheck")
        .and_then(serde_json::Value::as_bool)
        == Some(false)
}
