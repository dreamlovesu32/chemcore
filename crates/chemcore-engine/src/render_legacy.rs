use super::*;
use crate::DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT;

#[derive(Debug, Clone)]
struct LegacyLabelMetrics {
    visible: bool,
    pad: f64,
    label: String,
}

#[derive(Debug, Clone)]
struct LegacyCollapsedGroup {
    label: String,
    centroid: Point,
    connections: Vec<usize>,
}

pub(super) fn render_legacy_molecule_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemcoreDocument,
    object: &SceneObject,
    molblock: &str,
) {
    let Some(parsed) = parse_molblock(molblock) else {
        return;
    };
    let bbox = object.payload.bbox.unwrap_or([
        0.0,
        0.0,
        (parsed.max_x - parsed.min_x).max(1.0),
        (parsed.max_y - parsed.min_y).max(1.0),
    ]);
    let atom_points: Vec<Point> = parsed
        .atoms
        .iter()
        .map(|atom| legacy_map_atom_point(&parsed, atom, bbox[2], bbox[3], object))
        .collect();
    let (collapsed_groups, hidden_atoms, hidden_bonds) =
        legacy_build_collapsed_groups(&parsed, &atom_points);
    let label_metrics: Vec<LegacyLabelMetrics> = parsed
        .atoms
        .iter()
        .enumerate()
        .map(|(index, _)| {
            if hidden_atoms.contains(&index) {
                LegacyLabelMetrics {
                    visible: false,
                    pad: 0.0,
                    label: String::new(),
                }
            } else {
                legacy_label_metrics(&parsed, index)
            }
        })
        .collect();
    let stroke = molecule_stroke(document, object);
    let stroke_width = legacy_bond_stroke_width(document, object);
    let font_size = legacy_label_font_size(document, object);
    let font_family = legacy_label_font_family(document, object);
    let object_id = Some(object.id.clone());

    for (bond_index, bond) in parsed.bonds.iter().enumerate() {
        if hidden_bonds.contains(&bond_index) {
            continue;
        }
        render_legacy_bond(
            out,
            &parsed,
            bond,
            &atom_points,
            &hidden_atoms,
            &label_metrics,
            &stroke,
            stroke_width,
            object_id.clone(),
        );
    }

    for group in &collapsed_groups {
        for &outside_atom in &group.connections {
            if let Some(from) = atom_points.get(outside_atom).copied() {
                push_line(
                    out,
                    from,
                    group.centroid,
                    &stroke,
                    stroke_width,
                    Vec::new(),
                    RenderRole::DocumentBond,
                    object_id.clone(),
                );
            }
        }
        push_text(
            out,
            group.centroid.x,
            group.centroid.y,
            None,
            group.label.clone(),
            font_size,
            Some(font_family.clone()),
            Some(stroke.clone()),
            None,
            Vec::new(),
            object_id.clone(),
        );
    }

    for (index, atom) in parsed.atoms.iter().enumerate() {
        if hidden_atoms.contains(&index) {
            continue;
        }
        let metrics = &label_metrics[index];
        if !metrics.visible {
            continue;
        }
        let point = atom_points[index];
        let offset = legacy_atom_label_offset(&parsed, index);
        push_text(
            out,
            point.x + offset.x,
            point.y + offset.y,
            None,
            metrics.label.clone(),
            font_size,
            Some(font_family.clone()),
            Some(stroke.clone()),
            None,
            Vec::new(),
            object_id.clone(),
        );
        let _ = atom;
    }
}

fn legacy_map_atom_point(
    parsed: &LegacyMol,
    atom: &LegacyAtom,
    bbox_width: f64,
    bbox_height: f64,
    object: &SceneObject,
) -> Point {
    let width = (parsed.max_x - parsed.min_x).max(1.0);
    let height = (parsed.max_y - parsed.min_y).max(1.0);
    let scale = (bbox_width / width).min(bbox_height / height);
    let offset_x = (bbox_width - width * scale) / 2.0;
    let offset_y = (bbox_height - height * scale) / 2.0;
    Point::new(
        object.transform.translate[0] + (atom.x - parsed.min_x) * scale + offset_x,
        object.transform.translate[1] + (parsed.max_y - atom.y) * scale + offset_y,
    )
}

fn legacy_atom_degree(parsed: &LegacyMol, atom_index: usize) -> usize {
    parsed
        .bonds
        .iter()
        .filter(|bond| bond.begin == atom_index || bond.end == atom_index)
        .count()
}

fn legacy_atom_needs_label(parsed: &LegacyMol, atom_index: usize) -> bool {
    parsed
        .atoms
        .get(atom_index)
        .is_some_and(|atom| atom.symbol != "C" || legacy_atom_degree(parsed, atom_index) == 0)
}

fn legacy_bond_neighbors<'a>(
    parsed: &'a LegacyMol,
    atom_index: usize,
    excluded_atom_index: usize,
) -> Vec<&'a LegacyAtom> {
    parsed
        .bonds
        .iter()
        .filter_map(|bond| {
            if bond.begin == atom_index && bond.end != excluded_atom_index {
                parsed.atoms.get(bond.end)
            } else if bond.end == atom_index && bond.begin != excluded_atom_index {
                parsed.atoms.get(bond.begin)
            } else {
                None
            }
        })
        .collect()
}

fn legacy_format_atom_label(atom: &LegacyAtom) -> String {
    let mut label = atom.symbol.clone();
    match atom.charge {
        1 => label.push('+'),
        value if value > 1 => label.push_str(&format!("{value}+")),
        -1 => label.push('−'),
        value if value < -1 => label.push_str(&format!("{}−", value.abs())),
        _ => {}
    }
    label
}

fn legacy_label_metrics(parsed: &LegacyMol, atom_index: usize) -> LegacyLabelMetrics {
    if !legacy_atom_needs_label(parsed, atom_index) {
        return LegacyLabelMetrics {
            visible: false,
            pad: 0.0,
            label: String::new(),
        };
    }
    let label = legacy_format_atom_label(&parsed.atoms[atom_index]);
    let width =
        px_to_pt(12.0_f64).max(label.chars().count() as f64 * px_to_pt(6.2) + px_to_pt(8.0));
    LegacyLabelMetrics {
        visible: true,
        pad: width / 2.0 - px_to_pt(2.0),
        label,
    }
}

fn legacy_atom_label_offset(parsed: &LegacyMol, atom_index: usize) -> Vector {
    let Some(atom) = parsed.atoms.get(atom_index) else {
        return Vector::new(0.0, 0.0);
    };
    let neighbors = legacy_bond_neighbors(parsed, atom_index, usize::MAX);
    if neighbors.is_empty() {
        return Vector::new(0.0, 0.0);
    }
    let (vx, vy) = neighbors
        .iter()
        .fold((0.0, 0.0), |(sum_x, sum_y), neighbor| {
            (sum_x + neighbor.x - atom.x, sum_y + neighbor.y - atom.y)
        });
    let length = vx.hypot(vy).max(1.0);
    Vector::new(
        (-vx / length) * px_to_pt(4.5),
        (vy / length) * px_to_pt(4.5),
    )
}

fn legacy_line_endpoints_with_label_padding(
    start: Point,
    end: Point,
    start_pad: f64,
    end_pad: f64,
) -> (Point, Point) {
    let direction = Vector::new(end.x - start.x, end.y - start.y).normalized();
    (
        Point::new(
            start.x + direction.x * start_pad,
            start.y + direction.y * start_pad,
        ),
        Point::new(end.x - direction.x * end_pad, end.y - direction.y * end_pad),
    )
}

fn legacy_choose_double_bond_side(parsed: &LegacyMol, bond: &LegacyMolBond) -> f64 {
    let Some(begin) = parsed.atoms.get(bond.begin) else {
        return -1.0;
    };
    let Some(end) = parsed.atoms.get(bond.end) else {
        return -1.0;
    };
    let dx = end.x - begin.x;
    let dy = end.y - begin.y;
    let length = dx.hypot(dy).max(1.0);
    let normal_x = -dy / length;
    let normal_y = dx / length;

    let mut score = 0.0;
    for neighbor in legacy_bond_neighbors(parsed, bond.begin, bond.end) {
        score += (neighbor.x - begin.x) * normal_x + (neighbor.y - begin.y) * normal_y;
    }
    for neighbor in legacy_bond_neighbors(parsed, bond.end, bond.begin) {
        score += (neighbor.x - end.x) * normal_x + (neighbor.y - end.y) * normal_y;
    }
    if score >= 0.0 {
        -1.0
    } else {
        1.0
    }
}

fn render_legacy_bond(
    out: &mut Vec<RenderPrimitive>,
    parsed: &LegacyMol,
    bond: &LegacyMolBond,
    atom_points: &[Point],
    hidden_atoms: &BTreeSet<usize>,
    label_metrics: &[LegacyLabelMetrics],
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
) {
    if hidden_atoms.contains(&bond.begin) || hidden_atoms.contains(&bond.end) {
        return;
    }
    let Some(start_point) = atom_points.get(bond.begin).copied() else {
        return;
    };
    let Some(end_point) = atom_points.get(bond.end).copied() else {
        return;
    };
    let start_pad = label_metrics
        .get(bond.begin)
        .map(|metrics| metrics.pad)
        .unwrap_or(0.0);
    let end_pad = label_metrics
        .get(bond.end)
        .map(|metrics| metrics.pad)
        .unwrap_or(0.0);

    if bond.stereo == 1 {
        let (start, end) =
            legacy_line_endpoints_with_label_padding(start_point, end_point, start_pad, end_pad);
        let points = compute_solid_wedge_points(
            start,
            end,
            if end_pad > 0.0 {
                SOLID_WEDGE_END_INSET
            } else {
                0.0
            },
            legacy_wide_contact_directions(parsed, bond, bond.end, atom_points, hidden_atoms),
            stroke_width,
            solid_wedge_half_width(stroke_width),
        );
        push_polygon(
            out,
            points,
            stroke,
            stroke,
            stroke_width,
            RenderRole::DocumentBond,
            object_id,
        );
        return;
    }
    if bond.stereo == 6 {
        let (start, end) =
            legacy_line_endpoints_with_label_padding(start_point, end_point, start_pad, end_pad);
        for (segment_start, segment_end, segment_width) in
            compute_hashed_wedge_segments(start, end, stroke_width)
        {
            push_line(
                out,
                segment_start,
                segment_end,
                stroke,
                segment_width,
                Vec::new(),
                RenderRole::DocumentBond,
                object_id.clone(),
            );
        }
        return;
    }

    match bond.order {
        2 => render_legacy_double_bond(
            out,
            parsed,
            bond,
            start_point,
            end_point,
            start_pad > 0.0,
            end_pad > 0.0,
            stroke,
            stroke_width,
            object_id,
        ),
        3.. => render_legacy_triple_bond(
            out,
            parsed,
            bond,
            start_point,
            end_point,
            start_pad > 0.0,
            end_pad > 0.0,
            stroke,
            stroke_width,
            object_id,
        ),
        _ => push_line(
            out,
            start_point,
            end_point,
            stroke,
            stroke_width,
            Vec::new(),
            RenderRole::DocumentBond,
            object_id,
        ),
    }
}

fn render_legacy_double_bond(
    out: &mut Vec<RenderPrimitive>,
    parsed: &LegacyMol,
    bond: &LegacyMolBond,
    start: Point,
    end: Point,
    align_start: bool,
    align_end: bool,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
) {
    let degree_a = legacy_atom_degree(parsed, bond.begin);
    let degree_b = legacy_atom_degree(parsed, bond.end);
    let double_offset = double_bond_offset_distance(start, end, stroke_width);
    if degree_a == 1 || degree_b == 1 {
        let (normal_x, normal_y) = unit_normal(start, end);
        for offset in [-double_offset / 2.0, double_offset / 2.0] {
            push_line(
                out,
                Point::new(start.x + normal_x * offset, start.y + normal_y * offset),
                Point::new(end.x + normal_x * offset, end.y + normal_y * offset),
                stroke,
                stroke_width,
                Vec::new(),
                RenderRole::DocumentBond,
                object_id.clone(),
            );
        }
        return;
    }

    let side = legacy_choose_double_bond_side(parsed, bond);
    let (normal_x, normal_y) = unit_normal(start, end);
    let length = start.distance(end);
    let side_inset = (DOUBLE_BOND_SIDE_INSET * (stroke_width / VIEWER_BOND_STROKE))
        .max(length * DOUBLE_BOND_SIDE_INSET_RATIO);
    push_line(
        out,
        start,
        end,
        stroke,
        stroke_width,
        Vec::new(),
        RenderRole::DocumentBond,
        object_id.clone(),
    );
    let offset_start = Point::new(
        start.x + normal_x * double_offset * side,
        start.y + normal_y * double_offset * side,
    );
    let offset_end = Point::new(
        end.x + normal_x * double_offset * side,
        end.y + normal_y * double_offset * side,
    );
    let (short_start, short_end) = inset_bond_segment(
        offset_start,
        offset_end,
        if align_start { 0.0 } else { side_inset },
        if align_end { 0.0 } else { side_inset },
    );
    push_line(
        out,
        short_start,
        short_end,
        stroke,
        stroke_width,
        Vec::new(),
        RenderRole::DocumentBond,
        object_id,
    );
}

fn render_legacy_triple_bond(
    out: &mut Vec<RenderPrimitive>,
    parsed: &LegacyMol,
    bond: &LegacyMolBond,
    start: Point,
    end: Point,
    align_start: bool,
    align_end: bool,
    stroke: &str,
    stroke_width: f64,
    object_id: Option<String>,
) {
    let degree_a = legacy_atom_degree(parsed, bond.begin);
    let degree_b = legacy_atom_degree(parsed, bond.end);
    let scale = stroke_width / VIEWER_BOND_STROKE;
    let triple_offset = triple_bond_offset_distance(start, end, stroke_width);
    let length = start.distance(end);
    let side_inset = (DOUBLE_BOND_SIDE_INSET * scale).max(length * DOUBLE_BOND_SIDE_INSET_RATIO);
    let (normal_x, normal_y) = unit_normal(start, end);

    push_line(
        out,
        start,
        end,
        stroke,
        stroke_width,
        Vec::new(),
        RenderRole::DocumentBond,
        object_id.clone(),
    );

    for side in [1.0, -1.0] {
        let offset_start = Point::new(
            start.x + normal_x * triple_offset * side,
            start.y + normal_y * triple_offset * side,
        );
        let offset_end = Point::new(
            end.x + normal_x * triple_offset * side,
            end.y + normal_y * triple_offset * side,
        );
        let (short_start, short_end) = inset_bond_segment(
            offset_start,
            offset_end,
            if degree_a == 1 || align_start {
                0.0
            } else {
                side_inset
            },
            if degree_b == 1 || align_end {
                0.0
            } else {
                side_inset
            },
        );
        push_line(
            out,
            short_start,
            short_end,
            stroke,
            stroke_width,
            Vec::new(),
            RenderRole::DocumentBond,
            object_id.clone(),
        );
    }
}

fn legacy_has_visible_atom_label(
    parsed: &LegacyMol,
    atom_index: usize,
    hidden_atoms: &BTreeSet<usize>,
) -> bool {
    !hidden_atoms.contains(&atom_index) && legacy_atom_needs_label(parsed, atom_index)
}

fn legacy_is_side_double(parsed: &LegacyMol, bond: &LegacyMolBond) -> bool {
    bond.order == 2
        && legacy_atom_degree(parsed, bond.begin) > 1
        && legacy_atom_degree(parsed, bond.end) > 1
}

fn legacy_is_wide_contact_candidate(parsed: &LegacyMol, bond: &LegacyMolBond) -> bool {
    (bond.order == 1 && bond.stereo == 0) || legacy_is_side_double(parsed, bond)
}

fn legacy_wide_contact_directions(
    parsed: &LegacyMol,
    bond: &LegacyMolBond,
    wide_atom_index: usize,
    atom_points: &[Point],
    hidden_atoms: &BTreeSet<usize>,
) -> Vec<Vector> {
    if legacy_has_visible_atom_label(parsed, wide_atom_index, hidden_atoms) {
        return Vec::new();
    }
    let Some(wide_point) = atom_points.get(wide_atom_index).copied() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for other_bond in &parsed.bonds {
        if std::ptr::eq(other_bond, bond) || !legacy_is_wide_contact_candidate(parsed, other_bond) {
            continue;
        }
        if other_bond.begin != wide_atom_index && other_bond.end != wide_atom_index {
            continue;
        }
        let other_atom_index = if other_bond.begin == wide_atom_index {
            other_bond.end
        } else {
            other_bond.begin
        };
        if hidden_atoms.contains(&other_atom_index) {
            continue;
        }
        if let Some(other_point) = atom_points.get(other_atom_index).copied() {
            let vector = Vector::new(other_point.x - wide_point.x, other_point.y - wide_point.y);
            if vector.length() > 1.0e-6 {
                out.push(vector);
            }
        }
    }
    out
}

fn legacy_build_collapsed_groups(
    parsed: &LegacyMol,
    atom_points: &[Point],
) -> (Vec<LegacyCollapsedGroup>, BTreeSet<usize>, BTreeSet<usize>) {
    let mut collapsed_groups = Vec::new();
    let mut hidden_atoms = BTreeSet::new();
    let mut hidden_bonds = BTreeSet::new();

    for sgroup in &parsed.sgroups {
        if sgroup.kind != "SUP" || sgroup.label.trim().is_empty() || sgroup.atoms.is_empty() {
            continue;
        }
        let group_atoms: BTreeSet<usize> = sgroup.atoms.iter().copied().collect();
        let mut connections = Vec::new();
        for (bond_index, bond) in parsed.bonds.iter().enumerate() {
            let begin_inside = group_atoms.contains(&bond.begin);
            let end_inside = group_atoms.contains(&bond.end);
            if begin_inside && end_inside {
                hidden_bonds.insert(bond_index);
            } else if begin_inside || end_inside {
                connections.push(if begin_inside { bond.end } else { bond.begin });
                hidden_bonds.insert(bond_index);
            }
        }
        hidden_atoms.extend(group_atoms.iter().copied());

        let points: Vec<Point> = sgroup
            .atoms
            .iter()
            .filter_map(|index| atom_points.get(*index).copied())
            .collect();
        if points.is_empty() {
            continue;
        }

        let mut anchor_point = None;
        let mut direction = None;
        for bond_index in &sgroup.bonds {
            let Some(bond) = parsed.bonds.get(*bond_index) else {
                continue;
            };
            let inside_atom_index = if group_atoms.contains(&bond.begin) {
                Some(bond.begin)
            } else if group_atoms.contains(&bond.end) {
                Some(bond.end)
            } else {
                None
            };
            let Some(inside_atom_index) = inside_atom_index else {
                continue;
            };
            anchor_point = atom_points.get(inside_atom_index).copied();
            if let Some(vector) = sgroup.vectors.get(bond_index) {
                direction = Some(Vector::new(vector.x, -vector.y));
            }
            break;
        }

        let anchor_point = anchor_point.unwrap_or_else(|| {
            Point::new(
                points.iter().map(|point| point.x).sum::<f64>() / points.len() as f64,
                points.iter().map(|point| point.y).sum::<f64>() / points.len() as f64,
            )
        });
        let direction = direction.unwrap_or_else(|| {
            if let Some(outside_atom) = connections
                .first()
                .and_then(|index| atom_points.get(*index))
                .copied()
            {
                Vector::new(
                    anchor_point.x - outside_atom.x,
                    anchor_point.y - outside_atom.y,
                )
            } else {
                Vector::new(0.0, -1.0)
            }
        });
        let unit = direction.normalized();
        let label_width = px_to_pt(20.0_f64)
            .max(sgroup.label.chars().count() as f64 * px_to_pt(7.1) + px_to_pt(8.0));
        collapsed_groups.push(LegacyCollapsedGroup {
            label: sgroup.label.clone(),
            centroid: Point::new(
                anchor_point.x + unit.x * (label_width * 0.45 + px_to_pt(7.0)),
                anchor_point.y + unit.y * px_to_pt(10.0),
            ),
            connections,
        });
    }

    (collapsed_groups, hidden_atoms, hidden_bonds)
}

fn legacy_bond_stroke_width(document: &ChemcoreDocument, object: &SceneObject) -> f64 {
    object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
        .and_then(|style| {
            style_number(style, "strokeWidth").or_else(|| style_number(style, "stroke_width"))
        })
        .unwrap_or(VIEWER_BOND_STROKE)
}

fn legacy_label_font_family(document: &ChemcoreDocument, object: &SceneObject) -> String {
    object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
        .and_then(|style| style_string(style, "fontFamily"))
        .unwrap_or_else(|| "Arial".to_string())
}

fn legacy_label_font_size(document: &ChemcoreDocument, object: &SceneObject) -> f64 {
    object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref))
        .and_then(|style| {
            style_number(style, "fontSize").or_else(|| style_number(style, "font_size"))
        })
        .unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT)
}
