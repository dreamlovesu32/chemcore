use super::*;
use crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT;

#[path = "render_objects/arrows.rs"]
mod arrows;
#[path = "render_objects/graphics.rs"]
mod graphics;
#[path = "render_objects/text.rs"]
mod text;

pub(super) use arrows::render_line_object;
pub(super) use graphics::{
    render_bracket_object, render_curve_object, render_shape_object,
    render_stoichiometry_grid_object, render_table_object,
};
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
    for run in label.line_runs.iter().flatten() {
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
        outline: None,
        shadow: None,
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
    document: &ChemSemaDocument,
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
                || resource.encoding == "chemsema.molecule.fragment2d" =>
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

            render_fragment_molecular_colors(
                out,
                document,
                object,
                fragment,
                &node_map,
                object_id.clone(),
                None,
                None,
            );
            for bond in &fragment.bonds {
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
            render_main_bond_contact_patches(out, &contact_kernel, &stroke, object_id.clone());
            for bond in &fragment.bonds {
                render_fragment_bond_annotations(
                    out,
                    document,
                    object,
                    &node_map,
                    bond,
                    &stroke,
                    object_id.clone(),
                );
            }

            for node in &fragment.nodes {
                render_fragment_label(out, document, object, node, object_id.clone());
                render_fragment_nmr_assignments(out, document, object, node, object_id.clone());
                render_fragment_atom_properties(out, document, object, node, object_id.clone());
                render_external_connection_marker(
                    out,
                    document,
                    object,
                    fragment,
                    node,
                    &stroke,
                    object_id.clone(),
                );
                render_fragment_cdxml_node_markers(
                    out,
                    document,
                    object,
                    fragment,
                    node,
                    &stroke,
                    object_id.clone(),
                );
                render_fragment_atom_query_annotations(
                    out,
                    document,
                    object,
                    fragment,
                    node,
                    object_id.clone(),
                );
                render_fragment_node_invalid_marker(out, object, node, object_id.clone());
            }
        }
        ResourceData::Text(molblock) => {
            render_legacy_molecule_object(out, document, object, molblock);
        }
        _ => {}
    }
}

fn render_fragment_molecular_colors(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
    fragment: &MoleculeFragment,
    node_map: &BTreeMap<&str, &Node>,
    object_id: Option<String>,
    target_node_ids: Option<&BTreeSet<String>>,
    target_bond_ids: Option<&BTreeSet<String>>,
) {
    for area in &fragment.colored_areas {
        if target_bond_ids.is_some_and(|target_ids| {
            !area
                .basis_bonds
                .iter()
                .any(|bond_id| target_ids.contains(bond_id))
        }) {
            continue;
        }
        let Some(node_ids) = crate::ordered_colored_area_node_ids(fragment, &area.basis_bonds)
        else {
            continue;
        };
        let points = node_ids
            .iter()
            .filter_map(|node_id| node_map.get(node_id.as_str()))
            .map(|node| world_point(object, node))
            .collect::<Vec<_>>();
        if points.len() != node_ids.len() {
            continue;
        }
        out.push(RenderPrimitive::Polygon {
            role: RenderRole::DocumentMolecularColor,
            object_id: object_id.clone(),
            node_id: None,
            bond_id: area.basis_bonds.iter().min().cloned(),
            points,
            fill: area.color.clone(),
            stroke: area.color.clone(),
            stroke_width: 0.0,
        });
    }

    let default_bold_width = document
        .document
        .meta
        .pointer("/import/cdxml/defaults/boldWidth")
        .and_then(JsonValue::as_f64)
        .or_else(|| document.style.defaults.get("boldWidth").copied())
        .unwrap_or(BOLD_BOND_WIDTH);
    let default_margin_width = document
        .document
        .meta
        .pointer("/import/cdxml/defaults/marginWidth")
        .and_then(JsonValue::as_f64)
        .or_else(|| document.style.defaults.get("marginWidth").copied())
        .unwrap_or(crate::DEFAULT_BOND_MARGIN_WIDTH_PT.value());

    for bond in &fragment.bonds {
        if target_bond_ids.is_some_and(|target_ids| !target_ids.contains(&bond.id)) {
            continue;
        }
        let Some(color) = bond.highlight_color.as_ref() else {
            continue;
        };
        let (Some(begin), Some(end)) = (
            node_map.get(bond.begin.as_str()),
            node_map.get(bond.end.as_str()),
        ) else {
            continue;
        };
        let radius = bond.bold_width.unwrap_or(default_bold_width)
            + bond.margin_width.unwrap_or(default_margin_width);
        out.push(RenderPrimitive::Polyline {
            role: RenderRole::DocumentMolecularColor,
            object_id: object_id.clone(),
            bond_id: Some(bond.id.clone()),
            points: vec![world_point(object, begin), world_point(object, end)],
            stroke: color.clone(),
            stroke_width: radius * 2.0,
            dash_array: Vec::new(),
            line_cap: Some("round".to_string()),
            line_join: Some("round".to_string()),
        });
    }
    for node in &fragment.nodes {
        if target_node_ids.is_some_and(|target_ids| !target_ids.contains(&node.id)) {
            continue;
        }
        let Some(color) = node.highlight_color.as_ref() else {
            continue;
        };
        let radius = default_bold_width + default_margin_width;
        out.push(RenderPrimitive::Circle {
            role: RenderRole::DocumentMolecularColor,
            object_id: object_id.clone(),
            node_id: Some(node.id.clone()),
            center: world_point(object, node),
            radius,
            fill: color.clone(),
            stroke: color.clone(),
            stroke_width: 0.0,
        });
    }
}

pub(super) fn render_molecule_object_targets(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
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
        && resource.encoding != "chemsema.molecule.fragment2d"
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
    for bond in &fragment.bonds {
        let touches_target_node =
            target_node_ids.contains(&bond.begin) || target_node_ids.contains(&bond.end);
        if target_bond_ids.contains(&bond.id) || touches_target_node {
            target_render_bond_ids.insert(bond.id.clone());
        }
    }
    expand_target_render_bond_ids_for_contact_nodes(&mut target_render_bond_ids, &fragment.bonds);
    expand_target_render_bond_ids_for_crossings(
        &mut target_render_bond_ids,
        document,
        object,
        &fragment.bonds,
        &node_map,
    );
    let mut contact_node_ids = BTreeSet::new();
    for bond in &fragment.bonds {
        if target_render_bond_ids.contains(&bond.id) {
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

    render_fragment_molecular_colors(
        out,
        document,
        object,
        fragment,
        &node_map,
        object_id.clone(),
        Some(target_node_ids),
        Some(&target_render_bond_ids),
    );
    for bond in &fragment.bonds {
        if target_render_bond_ids.contains(&bond.id) {
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
    }
    render_main_bond_contact_patches(out, &contact_kernel, &stroke, object_id.clone());
    for bond in &fragment.bonds {
        if target_render_bond_ids.contains(&bond.id) {
            render_fragment_bond_annotations(
                out,
                document,
                object,
                &node_map,
                bond,
                &stroke,
                object_id.clone(),
            );
        }
    }

    let mut label_render_node_ids = target_node_ids.clone();
    label_render_node_ids.extend(contact_node_ids);
    for node in &fragment.nodes {
        if label_render_node_ids.contains(&node.id) {
            render_fragment_label(out, document, object, node, object_id.clone());
            render_fragment_nmr_assignments(out, document, object, node, object_id.clone());
            render_fragment_atom_properties(out, document, object, node, object_id.clone());
            render_external_connection_marker(
                out,
                document,
                object,
                fragment,
                node,
                &stroke,
                object_id.clone(),
            );
            render_fragment_cdxml_node_markers(
                out,
                document,
                object,
                fragment,
                node,
                &stroke,
                object_id.clone(),
            );
            render_fragment_atom_query_annotations(
                out,
                document,
                object,
                fragment,
                node,
                object_id.clone(),
            );
            render_fragment_node_invalid_marker(out, object, node, object_id.clone());
        }
    }
}

fn render_fragment_bond_annotations(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    bond: &Bond,
    fill: &str,
    object_id: Option<String>,
) {
    let show_query = bond.properties.show_query.unwrap_or_else(|| {
        document
            .document
            .meta
            .pointer("/import/cdxml/defaults/showBondQuery")
            .and_then(JsonValue::as_bool)
            .unwrap_or(true)
    });
    let show_reaction = bond.properties.show_reaction.unwrap_or_else(|| {
        document
            .document
            .meta
            .pointer("/import/cdxml/defaults/showBondRxn")
            .and_then(JsonValue::as_bool)
            .unwrap_or(true)
    });
    let show_stereo = bond.properties.show_stereo.unwrap_or_else(|| {
        document
            .document
            .meta
            .pointer("/import/cdxml/defaults/showBondStereo")
            .and_then(JsonValue::as_bool)
            .unwrap_or(false)
    });

    let mut query = String::new();
    if show_query {
        query.push_str(match bond.properties.topology {
            crate::BondTopology::Unspecified => "",
            crate::BondTopology::Ring => "Rng",
            crate::BondTopology::Chain => "Chn",
            crate::BondTopology::RingOrChain => "R/C",
        });
    }
    if show_reaction
        && matches!(
            bond.properties.reaction_participation,
            crate::BondReactionParticipation::ReactionCenter
                | crate::BondReactionParticipation::MakeOrBreak
                | crate::BondReactionParticipation::ChangeType
                | crate::BondReactionParticipation::MakeAndChange
        )
    {
        query.push_str("Rxn");
    }
    if bond.properties.query_orders.len() >= 2 {
        query.push_str(
            &bond
                .properties
                .query_orders
                .iter()
                .map(|value| value.mnemonic())
                .collect::<Vec<_>>()
                .join("/"),
        );
    }
    let stereo = if show_stereo {
        match bond.properties.absolute_stereo {
            crate::BondAbsoluteStereo::E => Some("(E)"),
            crate::BondAbsoluteStereo::Z => Some("(Z)"),
            crate::BondAbsoluteStereo::Unspecified | crate::BondAbsoluteStereo::None => None,
        }
    } else {
        None
    };
    if query.is_empty() && stereo.is_none() {
        return;
    }

    let (Some(begin), Some(end)) = (
        node_map.get(bond.begin.as_str()),
        node_map.get(bond.end.as_str()),
    ) else {
        return;
    };
    let begin = world_point(object, begin);
    let end = world_point(object, end);
    let mut axis = Vector::new(end.x - begin.x, end.y - begin.y);
    let length = axis.length();
    if length <= EPSILON {
        return;
    }
    axis = axis.scaled(1.0 / length);
    if axis.x < -EPSILON || (axis.x.abs() <= EPSILON && axis.y < 0.0) {
        axis = axis.scaled(-1.0);
    }
    let normal = Vector::new(-axis.y, axis.x);
    let midpoint = Point::new((begin.x + end.x) * 0.5, (begin.y + end.y) * 0.5);
    let label_size = document
        .document
        .meta
        .pointer("/import/cdxml/defaults/labelSize")
        .and_then(JsonValue::as_f64)
        .unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT);
    let font_size = label_size * 0.75;
    let font_family = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
        .and_then(|style| style_string(style, "fontFamily"));

    if let Some(stereo) = stereo {
        push_bond_annotation_text(
            out,
            midpoint,
            normal,
            -1.0,
            stereo,
            true,
            font_size,
            font_family.clone(),
            fill,
            object_id.clone(),
        );
    }
    if !query.is_empty() {
        push_bond_annotation_text(
            out,
            midpoint,
            normal,
            if stereo.is_some() { 1.0 } else { -1.0 },
            &query,
            false,
            font_size,
            font_family,
            fill,
            object_id,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_bond_annotation_text(
    out: &mut Vec<RenderPrimitive>,
    midpoint: Point,
    normal: Vector,
    side: f64,
    text: &str,
    italic: bool,
    font_size: f64,
    font_family: Option<String>,
    fill: &str,
    object_id: Option<String>,
) {
    let width = annotation_text_width(text, font_size);
    let height = font_size * 1.061_333_333;
    let horizontal_gap = font_size * 0.29;
    let vertical_gap = font_size * if side > 0.0 { 0.29 } else { 0.11 };
    let center = Point::new(
        midpoint.x + side * normal.x * (width * 0.5 + horizontal_gap),
        midpoint.y + side * normal.y * (height * 0.5 + vertical_gap),
    );
    let top = center.y - height * 0.5;
    push_text_for_node(
        out,
        center.x,
        top,
        Some(font_size * 0.82),
        String::new(),
        font_size,
        font_family.clone(),
        Some(fill.to_string()),
        Some("middle".to_string()),
        vec![LabelRun {
            text: text.to_string(),
            font_family,
            font_size: Some(font_size),
            fill: Some(fill.to_string()),
            font_weight: Some(400),
            font_style: Some(if italic { "italic" } else { "normal" }.to_string()),
            underline: Some(false),
            outline: Some(false),
            shadow: Some(false),
            script: Some("normal".to_string()),
        }],
        object_id,
        None,
    );
}

fn render_fragment_nmr_assignments(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
    node: &Node,
    object_id: Option<String>,
) {
    for assignment in &node.nmr_assignments {
        if assignment.validate().is_err() {
            continue;
        }
        let mut annotation_node = node.clone();
        annotation_node.label = Some(assignment.label.clone());
        annotation_node.nmr_assignments.clear();
        render_fragment_label(out, document, object, &annotation_node, object_id.clone());
    }
}

fn render_fragment_atom_properties(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
    node: &Node,
    object_id: Option<String>,
) {
    let properties = &node.atom_properties;
    if properties.is_default() {
        return;
    }
    let center = world_point(object, node);
    let font_size = node
        .label
        .as_ref()
        .map(fragment_label_font_size)
        .unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT);
    let annotation_size = font_size * 0.75;
    let fill = node
        .label
        .as_ref()
        .and_then(|label| label.fill.clone())
        .or_else(|| {
            object
                .style_ref
                .as_ref()
                .and_then(|style_ref| document.styles.get(style_ref))
                .and_then(|style| style_string(style, "fill"))
        })
        .unwrap_or_else(|| "#000000".to_string());
    let font_family = node
        .label
        .as_ref()
        .and_then(|label| label.font_family.clone())
        .or_else(|| {
            object
                .style_ref
                .as_ref()
                .and_then(|style_ref| document.styles.get(style_ref))
                .and_then(|style| style_string(style, "fontFamily"))
        });

    let mut bounds = label_box_world(node, object).unwrap_or(RectBox {
        x1: center.x - font_size * 0.3,
        y1: center.y - font_size * 0.45,
        x2: center.x + font_size * 0.3,
        y2: center.y + font_size * 0.45,
    });
    if properties.isotope_mass.is_some()
        && node
            .label
            .as_ref()
            .is_none_or(|label| !label.has_visible_text())
    {
        let element = if node.element.trim().is_empty() {
            "C"
        } else {
            node.element.as_str()
        };
        push_atom_property_text(
            out,
            document,
            center.x,
            center.y - font_size * 0.42,
            element,
            font_size,
            font_family.clone(),
            &fill,
            "middle",
            false,
            object_id.clone(),
            &node.id,
        );
        bounds = RectBox {
            x1: center.x - font_size * 0.35,
            y1: center.y - font_size * 0.55,
            x2: center.x + font_size * 0.35,
            y2: center.y + font_size * 0.55,
        };
    }

    if let Some(mass) = properties.isotope_mass {
        let annotation_top = (bounds.y1 + bounds.y2 - annotation_size) * 0.5;
        push_atom_property_text(
            out,
            document,
            bounds.x1 - font_size * 0.1875,
            annotation_top,
            &mass.to_string(),
            annotation_size,
            font_family.clone(),
            &fill,
            "end",
            false,
            object_id.clone(),
            &node.id,
        );
    }

    let has_attached_radical_symbol =
        crate::node_attached_electron_symbols(node)
            .iter()
            .any(|symbol| {
                symbol
                    .get("radicalDelta")
                    .and_then(JsonValue::as_i64)
                    .unwrap_or(0)
                    > 0
            });
    if !has_attached_radical_symbol {
        let radical_text = match properties.radical {
            crate::AtomRadical::None => None,
            crate::AtomRadical::Singlet => Some("••"),
            crate::AtomRadical::Doublet => Some("•"),
            crate::AtomRadical::Triplet => Some("• •"),
        };
        if let Some(radical_text) = radical_text {
            push_atom_property_text(
                out,
                document,
                bounds.x2 + annotation_size * 0.15,
                bounds.y1 - annotation_size * 0.15,
                radical_text,
                annotation_size,
                font_family.clone(),
                &fill,
                "start",
                false,
                object_id.clone(),
                &node.id,
            );
        }
    }

    let default_show_number = document
        .document
        .meta
        .pointer("/import/cdxml/defaults/showAtomNumber")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);
    let has_linked_atom_number = has_linked_atom_annotation(document, &node.id, &["atom_number"]);
    let stereo_is_visible = properties.show_atom_stereo.unwrap_or_else(|| {
        document
            .document
            .meta
            .pointer("/import/cdxml/defaults/showAtomStereo")
            .and_then(JsonValue::as_bool)
            .unwrap_or(false)
    }) && properties.cip_stereo.is_some();
    if !has_linked_atom_number && properties.show_atom_number.unwrap_or(default_show_number) {
        if let Some(number) = properties.atom_number.as_deref() {
            let number_on_left = stereo_is_visible
                || properties.radical != crate::AtomRadical::None
                || has_attached_radical_symbol;
            let (x, y) = indicator_position(
                properties.atom_number_position.as_ref(),
                if number_on_left {
                    bounds.x1 - font_size * 0.1875
                } else {
                    bounds.x2 + font_size * 0.1875
                },
                (bounds.y1 + bounds.y2 - annotation_size) * 0.5,
                center,
            );
            push_atom_property_text(
                out,
                document,
                x,
                y,
                number,
                annotation_size,
                font_family.clone(),
                &fill,
                if number_on_left { "end" } else { "start" },
                false,
                object_id.clone(),
                &node.id,
            );
        }
    }

    let default_show_stereo = document
        .document
        .meta
        .pointer("/import/cdxml/defaults/showAtomStereo")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);
    let has_linked_stereo =
        has_linked_atom_annotation(document, &node.id, &["stereo", "enhanced_stereo"]);
    if !has_linked_stereo && properties.show_atom_stereo.unwrap_or(default_show_stereo) {
        if let Some(stereo) = properties.cip_stereo.as_deref() {
            let (x, y) = indicator_position(
                properties.stereo_position.as_ref(),
                bounds.x2 + font_size * 0.1875,
                (bounds.y1 + bounds.y2 - annotation_size) * 0.5,
                center,
            );
            let stereo_text = if stereo.starts_with('(') && stereo.ends_with(')') {
                stereo.to_string()
            } else {
                format!("({stereo})")
            };
            push_atom_property_text(
                out,
                document,
                x,
                y,
                &stereo_text,
                annotation_size,
                font_family,
                &fill,
                "start",
                true,
                object_id,
                &node.id,
            );
        }
    }
}

fn has_linked_atom_annotation(document: &ChemSemaDocument, node_id: &str, roles: &[&str]) -> bool {
    document.scene_objects().into_iter().any(|object| {
        object
            .meta
            .get("attachedNodeId")
            .and_then(JsonValue::as_str)
            == Some(node_id)
            && object
                .meta
                .get("role")
                .and_then(JsonValue::as_str)
                .is_some_and(|role| roles.contains(&role))
    })
}

fn indicator_position(
    position: Option<&crate::IndicatorPosition>,
    default_x: f64,
    default_y: f64,
    center: Point,
) -> (f64, f64) {
    let Some(position) = position else {
        return (default_x, default_y);
    };
    if let Some([x, y]) = position.absolute {
        return (x, y);
    }
    if let Some([x, y]) = position.offset {
        return (center.x + x, center.y + y);
    }
    if let Some(angle) = position.angle.filter(|angle| angle.is_finite()) {
        let radius = position
            .offset
            .map(|offset| offset[0].hypot(offset[1]))
            .unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT);
        let radians = angle.to_radians();
        return (
            center.x + radians.cos() * radius,
            center.y + radians.sin() * radius,
        );
    }
    (default_x, default_y)
}

#[allow(clippy::too_many_arguments)]
fn push_atom_property_text(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    x: f64,
    y: f64,
    text: &str,
    font_size: f64,
    font_family: Option<String>,
    fill: &str,
    anchor: &str,
    italic: bool,
    object_id: Option<String>,
    node_id: &str,
) {
    let width = annotation_text_width(text, font_size);
    let left = if anchor == "end" {
        x - width
    } else if anchor == "middle" {
        x - width * 0.5
    } else {
        x
    };
    out.push(RenderPrimitive::Rect {
        role: RenderRole::DocumentKnockout,
        object_id: object_id.clone(),
        node_id: Some(node_id.to_string()),
        x: left - 0.35,
        y: y - font_size * 0.18,
        width: width + 0.7,
        height: font_size + 0.35,
        fill: Some(document.document.page.background.clone()),
        stroke: None,
        stroke_width: 0.0,
        rx: None,
        ry: None,
        dash_array: Vec::new(),
        fill_gradient: None,
    });
    let runs = if italic {
        vec![LabelRun {
            text: text.to_string(),
            font_family: font_family.clone(),
            font_size: Some(font_size),
            fill: Some(fill.to_string()),
            font_weight: None,
            font_style: Some("italic".to_string()),
            underline: None,
            outline: None,
            shadow: None,
            script: Some("normal".to_string()),
        }]
    } else {
        Default::default()
    };
    push_text_for_node(
        out,
        x,
        y,
        Some(font_size * 0.82),
        text.to_string(),
        font_size,
        font_family,
        Some(fill.to_string()),
        Some(anchor.to_string()),
        runs,
        object_id,
        Some(node_id.to_string()),
    );
}

fn annotation_text_width(text: &str, font_size: f64) -> f64 {
    text.chars()
        .map(|character| crate::glyph_kernel::shared_estimated_char_width(character, font_size))
        .sum()
}

fn render_external_connection_marker(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
    fragment: &MoleculeFragment,
    node: &Node,
    stroke: &str,
    object_id: Option<String>,
) {
    let Some(connection) = node.external_connection.as_ref() else {
        return;
    };
    let center = world_point(object, node);
    let label_size = external_connection_label_size(document);
    let line_width = external_connection_line_width(document);
    let diamond_radius = label_size * 0.375 + line_width;
    let node_id = Some(node.id.clone());

    match connection.connection_type {
        crate::ExternalConnectionType::Unspecified | crate::ExternalConnectionType::Diamond => {
            push_external_connection_diamond(
                out,
                center,
                diamond_radius,
                diamond_radius,
                stroke,
                stroke,
                0.0,
                object_id.clone(),
                node_id.clone(),
            );
            let ordinal = fragment
                .nodes
                .iter()
                .filter(|candidate| candidate.external_connection.is_some())
                .position(|candidate| candidate.id == node.id)
                .map(|index| index + 1)
                .unwrap_or(1);
            push_text_for_node(
                out,
                center.x,
                center.y + label_size * 0.27,
                None,
                ordinal.to_string(),
                label_size * 0.72,
                Some("Arial".to_string()),
                Some("#ffffff".to_string()),
                Some("middle".to_string()),
                Vec::new(),
                object_id,
                node_id,
            );
        }
        crate::ExternalConnectionType::Star => {
            push_text_for_node(
                out,
                center.x,
                center.y + label_size * 0.30,
                None,
                "*".to_string(),
                label_size,
                Some("Symbol".to_string()),
                Some(stroke.to_string()),
                Some("middle".to_string()),
                Vec::new(),
                object_id,
                node_id,
            );
        }
        crate::ExternalConnectionType::PolymerBead => {
            let radius = label_size * 0.75 + line_width * 2.0;
            for layer in 0..32 {
                let t = layer as f64 / 31.0;
                let layer_radius = radius * (1.0 - 0.8428 * t);
                let shift = radius * 0.4844 * t;
                let channel = (255.0 * (t * std::f64::consts::FRAC_PI_2).sin()).round() as u8;
                let fill = format!("#{channel:02x}{channel:02x}{channel:02x}");
                out.push(RenderPrimitive::Circle {
                    role: RenderRole::DocumentGraphic,
                    object_id: object_id.clone(),
                    node_id: node_id.clone(),
                    center: Point::new(center.x - shift, center.y - shift),
                    radius: layer_radius,
                    fill,
                    stroke: "none".to_string(),
                    stroke_width: 0.0,
                });
            }
            out.push(RenderPrimitive::Circle {
                role: RenderRole::DocumentGraphic,
                object_id,
                node_id,
                center,
                radius,
                fill: "none".to_string(),
                stroke: stroke.to_string(),
                stroke_width: line_width,
            });
        }
        crate::ExternalConnectionType::Wavy => {
            render_external_connection_wavy(
                out, document, object, fragment, node, stroke, object_id,
            );
        }
        crate::ExternalConnectionType::Residue
        | crate::ExternalConnectionType::Peptide
        | crate::ExternalConnectionType::Dna
        | crate::ExternalConnectionType::Rna
        | crate::ExternalConnectionType::Terminus
        | crate::ExternalConnectionType::Sulfide => {
            push_external_connection_diamond(
                out,
                center,
                diamond_radius,
                diamond_radius * (2.0 / 3.0),
                "#b3b3b3",
                stroke,
                line_width,
                object_id,
                node_id,
            );
        }
        crate::ExternalConnectionType::Nucleotide
        | crate::ExternalConnectionType::UnlinkedBranch => {
            push_external_connection_diamond(
                out,
                center,
                diamond_radius,
                diamond_radius,
                stroke,
                stroke,
                0.0,
                object_id,
                node_id,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn push_external_connection_diamond(
    out: &mut Vec<RenderPrimitive>,
    center: Point,
    radius_x: f64,
    radius_y: f64,
    fill: &str,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
    node_id: Option<String>,
) {
    out.push(RenderPrimitive::Polygon {
        role: RenderRole::DocumentGraphic,
        object_id,
        node_id,
        bond_id: None,
        points: vec![
            Point::new(center.x - radius_x, center.y),
            Point::new(center.x, center.y - radius_y),
            Point::new(center.x + radius_x, center.y),
            Point::new(center.x, center.y + radius_y),
        ],
        fill: fill.to_string(),
        stroke: stroke.to_string(),
        stroke_width,
    });
}

fn render_external_connection_wavy(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
    fragment: &MoleculeFragment,
    node: &Node,
    stroke: &str,
    object_id: Option<String>,
) {
    let center = world_point(object, node);
    let line_width = external_connection_line_width(document);
    let raw_span = external_connection_label_size(document) * 1.5 + line_width * 4.0;
    let span = raw_span.round();
    let connected_axis = fragment.bonds.iter().find_map(|bond| {
        let other_id = if bond.begin == node.id {
            Some(bond.end.as_str())
        } else if bond.end == node.id {
            Some(bond.begin.as_str())
        } else {
            None
        }?;
        let other = fragment
            .nodes
            .iter()
            .find(|candidate| candidate.id == other_id)?;
        let other = world_point(object, other);
        let vector = Vector::new(center.x - other.x, center.y - other.y);
        (vector.length() > EPSILON).then(|| vector.scaled(1.0 / vector.length()))
    });
    // ChemDraw orients a connected marker perpendicular to its first incident
    // bond. An unconnected marker has no molecular direction, so its documented
    // canonical orientation is vertical (a horizontal connection axis).
    let axis = match connected_axis {
        Some(axis) => axis,
        None => Vector::new(1.0, 0.0),
    };
    let tangent = Vector::new(-axis.y, axis.x);
    let start = Point::new(
        center.x - tangent.x * span * 0.5,
        center.y - tangent.y * span * 0.5,
    );
    let segments = (raw_span * 2.0).ceil().max(1.0) as usize;
    let advance = span / segments as f64;
    let amplitude = 0.5;
    let mut d = format!("M {:.4} {:.4}", start.x, start.y);
    let mut points = vec![start];
    for index in 0..segments {
        let phase = index % 4;
        let (
            from_offset,
            control1_offset,
            control2_offset,
            to_offset,
            control1_fraction,
            control2_fraction,
        ) = match phase {
            0 => (0.0, -0.552 * amplitude, -amplitude, -amplitude, 0.0, 0.448),
            1 => (-amplitude, -amplitude, -0.552 * amplitude, 0.0, 0.552, 1.0),
            2 => (0.0, 0.552 * amplitude, amplitude, amplitude, 0.0, 0.448),
            _ => (amplitude, amplitude, 0.552 * amplitude, 0.0, 0.552, 1.0),
        };
        let segment_start = Point::new(
            start.x + tangent.x * advance * index as f64 + axis.x * from_offset,
            start.y + tangent.y * advance * index as f64 + axis.y * from_offset,
        );
        if index > 0 {
            points.push(segment_start);
        }
        let next = Point::new(
            start.x + tangent.x * advance * (index + 1) as f64 + axis.x * to_offset,
            start.y + tangent.y * advance * (index + 1) as f64 + axis.y * to_offset,
        );
        let c1 = Point::new(
            start.x
                + tangent.x * advance * (index as f64 + control1_fraction)
                + axis.x * control1_offset,
            start.y
                + tangent.y * advance * (index as f64 + control1_fraction)
                + axis.y * control1_offset,
        );
        let c2 = Point::new(
            start.x
                + tangent.x * advance * (index as f64 + control2_fraction)
                + axis.x * control2_offset,
            start.y
                + tangent.y * advance * (index as f64 + control2_fraction)
                + axis.y * control2_offset,
        );
        d.push_str(&format!(
            " C {:.4} {:.4} {:.4} {:.4} {:.4} {:.4}",
            c1.x, c1.y, c2.x, c2.y, next.x, next.y
        ));
        points.extend([c1, c2, next]);
    }
    out.push(RenderPrimitive::Path {
        role: RenderRole::DocumentGraphic,
        object_id,
        bond_id: None,
        d,
        points,
        stroke: stroke.to_string(),
        stroke_width: line_width,
        dash_array: Vec::new(),
        line_cap: Some("butt".to_string()),
        line_join: Some("round".to_string()),
        rotate: 0.0,
        rotate_center: None,
    });
}

fn external_connection_label_size(document: &ChemSemaDocument) -> f64 {
    document
        .document
        .meta
        .pointer("/import/cdxml/defaults/labelSize")
        .and_then(JsonValue::as_f64)
        .unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT)
}

fn external_connection_line_width(document: &ChemSemaDocument) -> f64 {
    document
        .document
        .meta
        .pointer("/import/cdxml/defaults/lineWidth")
        .and_then(JsonValue::as_f64)
        .unwrap_or(DEFAULT_BOND_STROKE)
}

fn render_fragment_cdxml_node_markers(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
    fragment: &MoleculeFragment,
    node: &Node,
    stroke: &str,
    object_id: Option<String>,
) {
    let cdxml = node.meta.pointer("/import/cdxml");
    let h_dot = cdxml
        .and_then(|meta| meta.get("hDot"))
        .and_then(JsonValue::as_bool)
        == Some(true);
    let h_dash = cdxml
        .and_then(|meta| meta.get("hDash"))
        .and_then(JsonValue::as_bool)
        == Some(true);
    let is_unbonded_multi_attachment = cdxml
        .and_then(|meta| meta.get("nodeType"))
        .and_then(JsonValue::as_str)
        == Some("MultiAttachment")
        && !fragment
            .bonds
            .iter()
            .any(|bond| bond.begin == node.id || bond.end == node.id);
    if !h_dot && !h_dash && !is_unbonded_multi_attachment {
        return;
    }

    let center = world_point(object, node);
    let line_width = document
        .document
        .meta
        .pointer("/import/cdxml/defaults/lineWidth")
        .and_then(JsonValue::as_f64)
        .unwrap_or(DEFAULT_BOND_STROKE);
    let bold_width = document
        .document
        .meta
        .pointer("/import/cdxml/defaults/boldWidth")
        .and_then(JsonValue::as_f64)
        .unwrap_or(BOLD_BOND_WIDTH);
    let bond_length = document
        .document
        .meta
        .pointer("/import/cdxml/defaults/bondLength")
        .and_then(JsonValue::as_f64)
        .unwrap_or(crate::DEFAULT_BOND_LENGTH);

    if h_dot {
        out.push(RenderPrimitive::Circle {
            role: RenderRole::DocumentGraphic,
            object_id: object_id.clone(),
            node_id: Some(node.id.clone()),
            center,
            radius: bold_width * 0.5,
            fill: stroke.to_string(),
            stroke: stroke.to_string(),
            stroke_width: 0.0,
        });
    }
    if h_dash {
        let half_width = bold_width * 0.2625;
        for offset_y in [bold_width * 0.75, bold_width * 1.275] {
            push_cdxml_node_marker_line(
                out,
                object_id.clone(),
                Point::new(center.x - half_width, center.y + offset_y),
                Point::new(center.x + half_width, center.y + offset_y),
                stroke,
                line_width,
            );
        }
    }
    if is_unbonded_multi_attachment {
        // ChemDraw's unbonded MultiAttachment placeholder spans roughly 30%
        // of the document bond length (three full rays crossing at the node).
        let radius = bond_length * 0.15;
        for angle_degrees in [90.0_f64, 30.0, -30.0] {
            let angle = angle_degrees.to_radians();
            let dx = radius * angle.cos();
            let dy = radius * angle.sin();
            push_cdxml_node_marker_line(
                out,
                object_id.clone(),
                Point::new(center.x - dx, center.y - dy),
                Point::new(center.x + dx, center.y + dy),
                stroke,
                line_width,
            );
        }
    }
}

fn push_cdxml_node_marker_line(
    out: &mut Vec<RenderPrimitive>,
    object_id: Option<String>,
    from: Point,
    to: Point,
    stroke: &str,
    stroke_width: f64,
) {
    out.push(RenderPrimitive::Path {
        role: RenderRole::DocumentGraphic,
        object_id,
        bond_id: None,
        d: format!("M {:.4} {:.4} L {:.4} {:.4}", from.x, from.y, to.x, to.y),
        points: vec![from, to],
        stroke: stroke.to_string(),
        stroke_width,
        dash_array: Vec::new(),
        line_cap: Some("butt".to_string()),
        line_join: Some("miter".to_string()),
        rotate: 0.0,
        rotate_center: None,
    });
}

fn render_fragment_atom_query_annotations(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
    fragment: &MoleculeFragment,
    node: &Node,
    object_id: Option<String>,
) {
    let show_atom_query = node.atom_properties.show_atom_query.unwrap_or_else(|| {
        document
            .document
            .meta
            .pointer("/import/cdxml/defaults/showAtomQuery")
            .and_then(JsonValue::as_bool)
            .unwrap_or(true)
    });
    if !show_atom_query {
        return;
    }

    let properties = &node.atom_properties;
    let mut query = String::new();
    if let Some(value) = properties.substituents_exactly {
        query.push_str(&format!("X{value}"));
    } else if let Some(value) = properties.substituents_up_to {
        query.push_str(&format!("U{value}"));
    } else if let Some(value) = properties.free_sites {
        query.push('*');
        if value != 1 {
            query.push_str(&value.to_string());
        }
    }
    if properties.unsaturated_bonds != crate::UnsaturatedBonds::Unspecified {
        query.push('S');
    }
    if properties.ring_bond_count != crate::RingBondCount::Unspecified {
        query.push('R');
    }
    if properties.reaction_change {
        query.push('C');
    }
    if properties.reaction_stereo != crate::AtomReactionStereo::Unspecified {
        query.push('T');
    }
    if properties.translation != crate::QueryTranslation::Equal {
        query.push('L');
    }
    if properties.isotopic_abundance != crate::IsotopicAbundance::Unspecified {
        query.push('I');
    }
    let restrict_implicit_hydrogens = node
        .meta
        .pointer("/import/cdxml/restrictImplicitHydrogens")
        .and_then(JsonValue::as_bool)
        == Some(true);
    if query.is_empty() && !restrict_implicit_hydrogens {
        return;
    }

    let font_size = node
        .label
        .as_ref()
        .map(fragment_label_font_size)
        .unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT);
    let query_size = font_size * 0.75;
    let font_family = node
        .label
        .as_ref()
        .and_then(|label| label.font_family.clone())
        .or_else(|| {
            object
                .style_ref
                .as_ref()
                .and_then(|style_ref| document.styles.get(style_ref))
                .and_then(|style| style_string(style, "fontFamily"))
        });
    let fill = node
        .label
        .as_ref()
        .and_then(|label| label.fill.clone())
        .or_else(|| {
            object
                .style_ref
                .as_ref()
                .and_then(|style_ref| document.styles.get(style_ref))
                .and_then(|style| style_string(style, "fill"))
        });
    let center = world_point(object, node);
    let bounds = label_box_world(node, object).unwrap_or(RectBox {
        x1: center.x - font_size * 0.3,
        y1: center.y - font_size * 0.45,
        x2: center.x + font_size * 0.3,
        y2: center.y + font_size * 0.45,
    });
    if restrict_implicit_hydrogens {
        push_text_for_node(
            out,
            center.x + font_size * 0.17,
            bounds.y1 - font_size * 0.07,
            Some(font_size * 0.82),
            String::new(),
            font_size,
            font_family.clone(),
            fill.clone(),
            Some("start".to_string()),
            vec![LabelRun {
                text: "H".to_string(),
                font_family: font_family.clone(),
                font_size: Some(font_size),
                fill: fill.clone(),
                font_weight: Some(400),
                font_style: Some("normal".to_string()),
                underline: Some(false),
                outline: Some(false),
                shadow: Some(false),
                script: Some("normal".to_string()),
            }],
            object_id.clone(),
            Some(node.id.clone()),
        );
    }
    if query.is_empty() {
        return;
    }
    let direction = query_connection_direction(fragment, node);
    let horizontal = direction.x.abs() >= direction.y.abs();
    let left_annotation_width = properties
        .isotope_mass
        .map(|mass| annotation_text_width(&mass.to_string(), query_size))
        .unwrap_or(0.0);
    let (x, y, anchor) = if horizontal && direction.x >= 0.0 {
        (
            bounds.x1 - font_size * 0.1875 - left_annotation_width,
            (bounds.y1 + bounds.y2 - query_size) * 0.5,
            "end",
        )
    } else if horizontal {
        (
            bounds.x2 + font_size * 0.1875,
            (bounds.y1 + bounds.y2 - query_size) * 0.5,
            "start",
        )
    } else if direction.y < 0.0 {
        (
            (bounds.x1 + bounds.x2) * 0.5,
            bounds.y2 + query_size * 0.15,
            "middle",
        )
    } else {
        (
            (bounds.x1 + bounds.x2) * 0.5,
            bounds.y1 - query_size * 1.05,
            "middle",
        )
    };
    push_atom_query_text(
        out,
        document,
        x,
        y,
        &query,
        query_size,
        font_family,
        fill.as_deref().unwrap_or("#000000"),
        anchor,
        object_id,
        &node.id,
    );
}

#[allow(clippy::too_many_arguments)]
fn push_atom_query_text(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    x: f64,
    y: f64,
    text: &str,
    query_size: f64,
    font_family: Option<String>,
    fill: &str,
    anchor: &str,
    object_id: Option<String>,
    node_id: &str,
) {
    let mut runs = Vec::new();
    let mut width = 0.0;
    if let Some(rest) = text.strip_prefix('*') {
        let star_size = query_size + 0.8;
        width += annotation_text_width("*", star_size);
        runs.push(LabelRun {
            text: "*".to_string(),
            font_family: Some("Symbol".to_string()),
            font_size: Some(star_size),
            fill: Some(fill.to_string()),
            font_weight: Some(400),
            font_style: Some("normal".to_string()),
            underline: Some(false),
            outline: Some(false),
            shadow: Some(false),
            script: Some("normal".to_string()),
        });
        if !rest.is_empty() {
            width += annotation_text_width(rest, query_size);
            runs.push(LabelRun {
                text: rest.to_string(),
                font_family: font_family.clone(),
                font_size: Some(query_size),
                fill: Some(fill.to_string()),
                font_weight: Some(400),
                font_style: Some("normal".to_string()),
                underline: Some(false),
                outline: Some(false),
                shadow: Some(false),
                script: Some("normal".to_string()),
            });
        }
    } else {
        width = annotation_text_width(text, query_size);
        runs.push(LabelRun {
            text: text.to_string(),
            font_family: font_family.clone(),
            font_size: Some(query_size),
            fill: Some(fill.to_string()),
            font_weight: Some(400),
            font_style: Some("normal".to_string()),
            underline: Some(false),
            outline: Some(false),
            shadow: Some(false),
            script: Some("normal".to_string()),
        });
    }
    let left = match anchor {
        "end" => x - width,
        "middle" => x - width * 0.5,
        _ => x,
    };
    out.push(RenderPrimitive::Rect {
        role: RenderRole::DocumentKnockout,
        object_id: object_id.clone(),
        node_id: Some(node_id.to_string()),
        x: left - 0.35,
        y: y - query_size * 0.18,
        width: width + 0.7,
        height: query_size + 1.15,
        fill: Some(document.document.page.background.clone()),
        stroke: None,
        stroke_width: 0.0,
        rx: None,
        ry: None,
        dash_array: Vec::new(),
        fill_gradient: None,
    });
    push_text_for_node(
        out,
        x,
        y,
        Some(query_size * 0.82),
        String::new(),
        query_size,
        font_family,
        Some(fill.to_string()),
        Some(anchor.to_string()),
        runs,
        object_id,
        Some(node_id.to_string()),
    );
}

fn query_connection_direction(fragment: &MoleculeFragment, node: &Node) -> Vector {
    let positions: BTreeMap<&str, Point> = fragment
        .nodes
        .iter()
        .map(|candidate| (candidate.id.as_str(), candidate.point()))
        .collect();
    let mut directions = Vec::new();
    for bond in &fragment.bonds {
        let other_id = if bond.begin == node.id {
            Some(bond.end.as_str())
        } else if bond.end == node.id {
            Some(bond.begin.as_str())
        } else {
            None
        };
        if let Some(other) = other_id.and_then(|id| positions.get(id)) {
            let vector = Vector::new(other.x - node.position[0], other.y - node.position[1]);
            let length = vector.x.hypot(vector.y);
            if length > crate::EPSILON {
                directions.push(vector.y.atan2(vector.x).to_degrees());
            }
        }
    }
    if directions.is_empty() {
        return Vector::new(-1.0, 0.0);
    }
    let open_angle = crate::largest_angular_gap(&directions).center.to_radians();
    Vector::new(-open_angle.cos(), -open_angle.sin())
}

fn expand_target_render_bond_ids_for_contact_nodes(
    target_render_bond_ids: &mut BTreeSet<String>,
    bonds: &[Bond],
) {
    if target_render_bond_ids.is_empty() {
        return;
    }

    let mut contact_node_ids = BTreeSet::new();
    for bond in bonds {
        if target_render_bond_ids.contains(&bond.id) {
            contact_node_ids.insert(bond.begin.clone());
            contact_node_ids.insert(bond.end.clone());
        }
    }

    for bond in bonds {
        if contact_node_ids.contains(&bond.begin) || contact_node_ids.contains(&bond.end) {
            target_render_bond_ids.insert(bond.id.clone());
        }
    }
}

fn expand_target_render_bond_ids_for_crossings(
    target_render_bond_ids: &mut BTreeSet<String>,
    document: &ChemSemaDocument,
    object: &SceneObject,
    bonds: &[Bond],
    node_map: &BTreeMap<&str, &Node>,
) {
    if target_render_bond_ids.is_empty() {
        return;
    }
    let mut extra = BTreeSet::new();
    let target_indices: Vec<usize> = bonds
        .iter()
        .enumerate()
        .filter_map(|(index, bond)| target_render_bond_ids.contains(&bond.id).then_some(index))
        .collect();
    for target_index in target_indices {
        for other_index in 0..bonds.len() {
            if target_index == other_index {
                continue;
            }
            let (under_bond, over_bond) = if target_index < other_index {
                (&bonds[target_index], &bonds[other_index])
            } else {
                (&bonds[other_index], &bonds[target_index])
            };
            if bonds_have_crossing_margin(document, object, node_map, over_bond, under_bond) {
                extra.insert(over_bond.id.clone());
            }
        }
    }
    target_render_bond_ids.extend(extra);
}

fn bonds_have_crossing_margin(
    document: &ChemSemaDocument,
    object: &SceneObject,
    node_map: &BTreeMap<&str, &Node>,
    over_bond: &Bond,
    under_bond: &Bond,
) -> bool {
    if bonds_share_endpoint(over_bond, under_bond) {
        return false;
    }
    let under_crossings = imported_cdxml_crossing_bonds(under_bond);
    let over_crossings = imported_cdxml_crossing_bonds(over_bond);
    if (under_crossings.is_some() || over_crossings.is_some())
        && !under_crossings
            .as_ref()
            .is_some_and(|ids| ids.contains(&over_bond.id))
        && !over_crossings
            .as_ref()
            .is_some_and(|ids| ids.contains(&under_bond.id))
    {
        return false;
    }
    let Some((over_start, over_end)) = bond_world_segment(object, node_map, over_bond) else {
        return false;
    };
    let Some((under_start, under_end)) = bond_world_segment(object, node_map, under_bond) else {
        return false;
    };
    let over_vector = Vector::new(over_end.x - over_start.x, over_end.y - over_start.y);
    let under_vector = Vector::new(under_end.x - under_start.x, under_end.y - under_start.y);
    if over_vector.length() <= EPSILON || under_vector.length() <= EPSILON {
        return false;
    }
    let crossing_sin = vector_cross(over_vector.normalized(), under_vector.normalized()).abs();
    if crossing_sin <= 0.1 {
        return false;
    }
    if interior_segment_intersection(over_start, over_end, under_start, under_end).is_some() {
        return true;
    }

    let under_stroke_width = bond_stroke_width(document, object, under_bond);
    let over_stroke_width = bond_stroke_width(document, object, over_bond);
    let margin_width = document_margin_width_for_bond(document, over_bond, over_stroke_width);
    if margin_width <= EPSILON {
        return false;
    }
    let under_envelope =
        document_bond_crossing_envelope(under_bond, under_start, under_end, under_stroke_width);
    let over_envelope =
        document_bond_crossing_envelope(over_bond, over_start, over_end, over_stroke_width);
    let Some(under_polygon) = crossing_strip_polygon_for_segment(
        under_start,
        under_end,
        under_envelope.silhouette_start,
        under_envelope.silhouette_end,
        0.05,
        0.0,
    ) else {
        return false;
    };
    let Some(over_polygon) = crossing_strip_polygon_for_segment(
        over_start,
        over_end,
        over_envelope.clearance_start,
        over_envelope.clearance_end,
        margin_width,
        margin_width,
    ) else {
        return false;
    };
    let overlap = intersect_convex_polygons(&under_polygon, &over_polygon);
    overlap.len() >= 3 && polygon_area_signed(&overlap).abs() > 1.0e-4
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
        stroke_width: 0.5,
    });
}

pub(super) fn render_fragment_label(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
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
                node_id: Some(node.id.clone()),
                x: box_value.x1,
                y: box_value.y1,
                width: (box_value.x2 - box_value.x1).max(0.0),
                height: (box_value.y2 - box_value.y1).max(0.0),
                fill: Some("none".to_string()),
                stroke: Some("#d32f2f".to_string()),
                stroke_width: 0.5,
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
    let line_height = label
        .line_height
        .filter(|value| value.is_finite() && *value > 0.0)
        .unwrap_or_else(|| crate::molecule_label_line_advance(font_size));
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
            line_height: Some(line_height),
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
    let box_top = label_box
        .map(|box_value| box_value.y1)
        .unwrap_or(world_position.y - font_size * 0.82);
    let mut baseline_advance = 0.0;
    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            baseline_advance += label
                .line_advances
                .get(index - 1)
                .copied()
                .unwrap_or(line_height);
        }
        let baseline_y = box_top + baseline_advance + font_size * 0.82;
        push_text_for_node(
            out,
            world_position.x,
            baseline_y,
            Some(font_size * 0.82),
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
    if meta
        .get("defaultChemical")
        .and_then(serde_json::Value::as_bool)
        == Some(false)
    {
        return true;
    }
    meta.get("chemicalCheck")
        .and_then(serde_json::Value::as_bool)
        == Some(false)
}
