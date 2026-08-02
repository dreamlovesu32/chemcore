use super::*;

#[test]
fn render_document_keeps_side_double_outer_line_full_length_when_only_opposite_side_single_is_attached(
) {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 68.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": {
                    "placement": "right"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);

    let mut indexed_lengths: Vec<_> = polygons
        .iter()
        .enumerate()
        .map(|(index, points)| (index, bond_axis_length(points).expect("bond axis length")))
        .collect();
    indexed_lengths.sort_by(|(_, a), (_, b)| a.total_cmp(b));

    let (short_index, short_length) = indexed_lengths[0];
    let (long_index, long_length) = indexed_lengths[1];
    let short_axis = bond_axis_from_points(&polygons[short_index]).expect("short axis");
    let long_axis = bond_axis_from_points(&polygons[long_index]).expect("long axis");
    assert!(
        (short_length - long_length).abs() < 0.05,
        "short={short_length} long={long_length}"
    );
    assert!(
        (short_axis.0.x - 20.0).abs() < 0.05 && (short_axis.1.x - 56.0).abs() < 0.05,
        "{short_axis:?}"
    );
    assert!(
        (long_axis.0.x - 20.0).abs() < 0.05 && (long_axis.1.x - 56.0).abs() < 0.05,
        "{long_axis:?}"
    );
}

#[test]
fn render_document_keeps_same_side_single_attached_side_double_outer_line_shortened() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": {
                    "placement": "right"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);

    let mut indexed_lengths: Vec<_> = polygons
        .iter()
        .enumerate()
        .map(|(index, points)| (index, bond_axis_length(points).expect("bond axis length")))
        .collect();
    indexed_lengths.sort_by(|(_, a), (_, b)| a.total_cmp(b));

    let short_length = indexed_lengths[0].1;
    let long_length = indexed_lengths[1].1;
    assert!(
        short_length < long_length - 0.05,
        "{short_length} {long_length}"
    );
}

#[test]
fn render_document_uses_adjacent_angle_for_side_double_secondary_inset() {
    let axis_length = 60.0;
    let center_distance = axis_length * 0.12;
    for (segmented, angle_degrees) in [false, true].into_iter().flat_map(|segmented| {
        [30.0_f64, 60.0, 90.0, 120.0]
            .into_iter()
            .map(move |angle| (segmented, angle))
    }) {
        let angle = angle_degrees.to_radians();
        let branch_dx = 30.0 * angle.cos();
        let branch_dy = 30.0 * angle.sin();
        let line_styles = if segmented {
            json!({ "main": "dashed", "left": "dashed", "right": "dashed" })
        } else {
            json!({})
        };
        let document = fragment_document(
            json!([
                { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
                { "id": "n2", "element": "C", "atomicNumber": 6, "position": [80.0, 40.0], "charge": 0, "numHydrogens": 0 },
                { "id": "n3", "element": "C", "atomicNumber": 6, "position": [20.0 - branch_dx, 40.0 - branch_dy], "charge": 0, "numHydrogens": 0 },
                { "id": "n4", "element": "C", "atomicNumber": 6, "position": [80.0 + branch_dx, 40.0 - branch_dy], "charge": 0, "numHydrogens": 0 }
            ]),
            json!([
                {
                    "id": "b1",
                    "begin": "n1",
                    "end": "n2",
                    "order": 2,
                    "strokeWidth": 1.0,
                    "bondSpacing": 12.0,
                    "lineStyles": line_styles,
                    "double": { "placement": "right" }
                },
                { "id": "b2", "begin": "n1", "end": "n3", "order": 1, "strokeWidth": 1.0 },
                { "id": "b3", "begin": "n2", "end": "n4", "order": 1, "strokeWidth": 1.0 }
            ]),
        );

        let axes: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
            .into_iter()
            .filter(|(bond_id, _)| bond_id == "b1")
            .filter_map(|(_, points)| bond_axis_from_points(&points))
            .collect();
        let secondary_y = axes
            .iter()
            .map(|(from, to)| (from.y + to.y) * 0.5)
            .min_by(f64::total_cmp)
            .expect("side-double secondary lane");
        let secondary_axes: Vec<_> = axes
            .iter()
            .filter(|(from, to)| ((from.y + to.y) * 0.5 - secondary_y).abs() <= 0.02)
            .collect();
        let secondary_start = secondary_axes
            .iter()
            .flat_map(|(from, to)| [from.x, to.x])
            .min_by(f64::total_cmp)
            .expect("secondary start");
        let secondary_end = secondary_axes
            .iter()
            .flat_map(|(from, to)| [from.x, to.x])
            .max_by(f64::total_cmp)
            .expect("secondary end");
        let rendered_length = secondary_end - secondary_start;
        let expected_inset = center_distance * (angle * 0.5).tan();
        let expected_length = axis_length - 2.0 * expected_inset;
        assert!(
            (rendered_length - expected_length).abs() <= 0.02,
            "segmented={segmented} angle={angle_degrees} rendered={rendered_length} expected={expected_length}"
        );
    }
}

#[test]
fn render_document_keeps_triple_outer_lines_full_at_single_bond_junctions() {
    for angle_degrees in [
        30.0_f64, 60.0, 90.0, 120.0, 150.0, 180.0, 210.0, 240.0, 270.0, 300.0, 330.0,
    ] {
        let angle = angle_degrees.to_radians();
        let branch = [20.0 + 30.0 * angle.cos(), 40.0 + 30.0 * angle.sin()];
        let document = fragment_document(
            json!([
                { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
                { "id": "n2", "element": "C", "atomicNumber": 6, "position": [80.0, 40.0], "charge": 0, "numHydrogens": 0 },
                { "id": "n3", "element": "C", "atomicNumber": 6, "position": branch, "charge": 0, "numHydrogens": 0 }
            ]),
            json!([
                { "id": "b1", "begin": "n1", "end": "n2", "order": 3, "strokeWidth": 1.0, "bondSpacing": 12.0 },
                { "id": "b2", "begin": "n1", "end": "n3", "order": 1, "strokeWidth": 1.0 }
            ]),
        );
        let axes: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
            .into_iter()
            .filter(|(bond_id, _)| bond_id == "b1")
            .filter_map(|(_, points)| bond_axis_from_points(&points))
            .filter(|(from, to)| ((from.y + to.y) * 0.5 - 40.0).abs() > 0.1)
            .collect();
        assert_eq!(axes.len(), 2, "angle={angle_degrees}");
        for (from, to) in axes {
            let minimum_x = from.x.min(to.x);
            let maximum_x = from.x.max(to.x);
            assert!(
                (minimum_x - 20.0).abs() <= 0.02 && (maximum_x - 80.0).abs() <= 0.02,
                "angle={angle_degrees} from={from:?} to={to:?}"
            );
        }
    }
}

#[test]
fn render_document_joins_center_double_only_on_occupied_side() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);

    let mut indexed_extensions: Vec<_> = polygons
        .iter()
        .enumerate()
        .map(|(index, polygon)| (index, polygon[1].x.max(polygon[2].x)))
        .collect();
    indexed_extensions.sort_by(|(_, a), (_, b)| a.total_cmp(b));
    let unchanged = &polygons[indexed_extensions[0].0];
    let extended = &polygons[indexed_extensions[1].0];

    assert!(
        (unchanged[1].x - 56.0).abs() < 0.001 && (unchanged[2].x - 56.0).abs() < 0.001,
        "{unchanged:?}"
    );
    assert!(
        (extended[1].x - 56.0).abs() > 0.05 || (extended[2].x - 56.0).abs() > 0.05,
        "{extended:?}"
    );
}

#[test]
fn render_document_keeps_center_double_joined_line_normal_width() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.6,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.6
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);

    for polygon in &polygons {
        let (start_width, end_width) =
            bond_polygon_normal_widths(polygon).expect("center double polygon width");
        assert!(
            (start_width - 0.6).abs() <= 1.0e-6 && (end_width - 0.6).abs() <= 1.0e-6,
            "{polygon:?} start={start_width} end={end_width}"
        );
    }
}

#[test]
fn render_document_keeps_center_double_original_for_straight_through_180_degrees() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [92.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);
    for polygon in polygons {
        assert!((polygon[1].x - 56.0).abs() < 0.001, "{polygon:?}");
        assert!((polygon[2].x - 56.0).abs() < 0.001, "{polygon:?}");
    }
}

#[test]
fn render_document_keeps_center_double_original_for_angles_over_162_degrees() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [91.45, 33.75], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);
    for polygon in polygons {
        assert!((polygon[1].x - 56.0).abs() < 0.05, "{polygon:?}");
        assert!((polygon[2].x - 56.0).abs() < 0.05, "{polygon:?}");
    }
}

#[test]
fn render_document_uses_larger_individual_label_retreat_for_both_center_double_lines() {
    let document = normalize_test_document(&fragment_document(
        json!([
            {
                "id": "n1",
                "element": "C",
                "atomicNumber": 6,
                "position": [20.0, 60.0],
                "charge": 0,
                "numHydrogens": 0
            },
            {
                "id": "n2",
                "element": "N",
                "atomicNumber": 7,
                "position": [56.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [56.0, 45.0],
                    "box": [51.0, 34.0, 61.0, 46.0]
                }
            }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            }
        ]),
    ));

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2);

    let axis = chemsema_engine::Point::new(20.0 - 56.0, 60.0 - 40.0);
    let axis_length = (axis.x * axis.x + axis.y * axis.y).sqrt();
    let unit_x = axis.x / axis_length;
    let unit_y = axis.y / axis_length;
    let endpoint_retreats: Vec<_> = polygons
        .iter()
        .map(|polygon| {
            let (from, to) = bond_axis_from_points(polygon).expect("bond axis");
            let endpoint = if from.distance(chemsema_engine::Point::new(56.0, 40.0))
                <= to.distance(chemsema_engine::Point::new(56.0, 40.0))
            {
                from
            } else {
                to
            };
            (endpoint.x - 56.0) * unit_x + (endpoint.y - 40.0) * unit_y
        })
        .collect();

    assert!(
        (endpoint_retreats[0] - endpoint_retreats[1]).abs() <= 1.0e-4,
        "parallel center-double lines should apply the larger of their independently computed label retreats: {polygons:?} {endpoint_retreats:?}"
    );
    assert!(
        endpoint_retreats.iter().all(|retreat| *retreat > 0.0),
        "{polygons:?} {endpoint_retreats:?}"
    );
    let axis_lengths: Vec<_> = polygons
        .iter()
        .map(|polygon| {
            let (from, to) = bond_axis_from_points(polygon).expect("bond axis");
            from.distance(to)
        })
        .collect();
    assert!(
        (axis_lengths[0] - axis_lengths[1]).abs() <= 1.0e-4,
        "center-double strokes must remain equal length after label retreat: {polygons:?} {axis_lengths:?}"
    );
}

#[test]
fn render_document_side_double_uses_anchor_glyph_retreat_once_and_keeps_lines_equal() {
    let document = fragment_document(
        json!([
            {
                "id": "n1",
                "element": "P",
                "atomicNumber": 15,
                "position": [62.0, 12.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "P(OPh)",
                    "position": [58.0, 15.9],
                    "box": [58.0, 6.0, 94.0, 22.0],
                    "runs": [{ "text": "P(OPh)", "fontFamily": "Arial", "fontSize": 10.0, "script": "normal" }],
                    "glyphPolygons": [
                        [[58.0, 8.0], [66.0, 8.0], [66.0, 18.0], [58.0, 18.0]],
                        [[67.0, 6.0], [69.0, 6.0], [69.0, 22.0], [67.0, 22.0]],
                        [[71.0, 8.0], [77.0, 8.0], [77.0, 18.0], [71.0, 18.0]],
                        [[79.0, 8.0], [85.0, 8.0], [85.0, 18.0], [79.0, 18.0]],
                        [[90.0, 6.0], [92.0, 6.0], [92.0, 22.0], [90.0, 22.0]]
                    ]
                }
            },
            {
                "id": "n2",
                "element": "O",
                "atomicNumber": 8,
                "position": [62.0, 36.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "O",
                    "position": [56.0, 39.9],
                    "box": [56.0, 32.0, 64.0, 42.0],
                    "glyphPolygons": [[[56.0, 32.0], [64.0, 32.0], [64.0, 42.0], [56.0, 42.0]]]
                }
            }
        ]),
        json!([{
            "id": "b1",
            "begin": "n1",
            "end": "n2",
            "order": 2,
            "strokeWidth": 0.6,
            "double": { "placement": "left" }
        }]),
    );

    let axes: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| bond_axis_from_points(&points).expect("bond axis"))
        .collect();
    assert_eq!(axes.len(), 2);
    let main_axis = axes
        .iter()
        .min_by(|left, right| {
            let left_x = (left.0.x + left.1.x) * 0.5;
            let right_x = (right.0.x + right.1.x) * 0.5;
            (left_x - 62.0).abs().total_cmp(&(right_x - 62.0).abs())
        })
        .expect("main axis");
    assert!(
        ((main_axis.0.x + main_axis.1.x) * 0.5 - 62.0).abs() <= 0.02,
        "without EndAttach the side-double main line must use the structural node even when cached glyph geometry is shifted: {axes:?}"
    );
    let lengths: Vec<_> = axes.iter().map(|(from, to)| from.distance(*to)).collect();
    assert!(
        (lengths[0] - lengths[1]).abs() <= 1.0e-4,
        "side-double lines must share the larger single-pass glyph retreat: {axes:?}"
    );
    for (from, to) in axes {
        let label_exit_y = from.y.min(to.y);
        assert!(
            (label_exit_y - 18.0).abs() <= 0.02,
            "the synthetic internal-row rectangle must not over-clip a bond anchored on P: {from:?} {to:?}"
        );
    }
}

#[test]
fn parse_cdxml_side_double_terminal_label_tracks_double_bond_midline() {
    let source = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="70 80 140 130" BondLength="14.4" LabelFont="3" LabelSize="10" MarginWidth="1.6">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="70 80 140 130">
    <fragment id="2" BoundingBox="80 90 135 125">
      <n id="3" p="85 100"/>
      <n id="4" p="100 100" NodeType="Nickname">
        <t id="5" BoundingBox="96.7 94 134.3 105" p="96.7 103.9" LabelAlignment="Left" LabelJustification="Left" InterpretChemically="yes">
          <s font="3" size="10" face="0" color="0">P(OPh)</s>
          <s font="3" size="10" face="96" color="0">2</s>
        </t>
      </n>
      <n id="6" p="100 114.4" Element="8" NumHydrogens="0">
        <t id="7" BoundingBox="94.8 109.9 102.6 118.8" p="94.8 118.1" LabelJustification="Left" InterpretChemically="yes">
          <s font="3" size="10" face="0" color="0">O</s>
        </t>
      </n>
      <b id="8" B="3" E="4" Order="1"/>
      <b id="9" B="4" E="6" Order="2" DoublePosition="Left"/>
    </fragment>
  </page>
</CDXML>"#;

    let document = parse_cdxml_document(source, Some("side-double terminal label"))
        .expect("CDXML should import");
    let fragment = document
        .editable_fragments()
        .into_iter()
        .next()
        .expect("fragment");
    let oxygen = fragment
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "6")
        .expect("oxygen node");
    let oxygen_label = oxygen.label.as_ref().expect("oxygen label");
    let oxygen_baseline = oxygen_label.position.expect("oxygen baseline");
    assert!(
        (oxygen_baseline[0] - oxygen.position[0] + 5.16).abs() <= 0.02
            && (oxygen_baseline[1] - oxygen.position[1] - 3.90).abs() <= 0.02,
        "terminal O baseline must follow the midpoint of the asymmetric double-bond lines: {oxygen:?}"
    );

    let axes: Vec<_> = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                bond_id: Some(bond_id),
                points,
                ..
            } if role == RenderRole::DocumentBond && bond_id == "9" => {
                bond_axis_from_points(&points)
            }
            _ => None,
        })
        .collect();
    assert_eq!(axes.len(), 2, "{axes:?}");
    for (from, to) in &axes {
        assert!(
            (from.x - to.x).abs() <= 0.05,
            "side-double lines must remain vertical on the source node axis: {axes:?}"
        );
    }
    assert!(
        (axes[0].0.distance(axes[0].1) - axes[1].0.distance(axes[1].1)).abs() <= 1.0e-4,
        "terminal side-double lines must remain equal after label retreat: {axes:?}"
    );
}

#[test]
fn parse_cdxml_begin_attach_uses_internal_label_glyph_and_round_trips_stably() {
    let source = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BoundingBox="300 590 370 630" BondLength="14.4" LabelFont="3" LabelSize="10" MarginWidth="1.6">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="300 590 370 630">
    <fragment id="2" BoundingBox="309.75 598.802 362.063 624.05">
      <n id="3" p="311.625 617.025" NodeType="Fragment">
        <t id="4" BoundingBox="309.75 612.55 362.063 624.05" p="309.75 621.5" LabelAlignment="Left" LabelJustification="Left" InterpretChemically="yes">
          <s font="3" size="10" face="0" color="0">(PhO)</s>
          <s font="3" size="10" face="96" color="0">2</s>
          <s font="3" size="10" face="0" color="0">POH</s>
        </t>
      </n>
      <n id="5" p="343.569 603.527" Element="8" NumHydrogens="0">
        <t id="6" BoundingBox="339.663 598.802 347.413 607.302" p="339.663 607.202" LabelJustification="Left">
          <s font="3" size="10" face="96" color="0">O</s>
        </t>
      </n>
      <b id="7" B="3" BeginAttach="6" E="5" Order="2"/>
    </fragment>
  </page>
</CDXML>"#;

    let document = parse_cdxml_document(source, Some("internal label attachment"))
        .expect("CDXML should import");
    let fragment = document
        .editable_fragments()
        .into_iter()
        .next()
        .expect("fragment");
    assert_eq!(
        fragment.fragment.bonds[0]
            .meta
            .pointer("/endpointAttachments/begin/characterIndex")
            .and_then(serde_json::Value::as_u64),
        Some(6)
    );
    assert_eq!(
        fragment.fragment.bonds[0]
            .meta
            .pointer("/endpointAttachments/begin/character"),
        Some(&json!("P"))
    );

    let polygons: Vec<_> = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                bond_id: Some(bond_id),
                points,
                ..
            } if role == RenderRole::DocumentBond && bond_id == "7" => Some(points),
            _ => None,
        })
        .collect();
    assert_eq!(polygons.len(), 2);
    let axes: Vec<_> = polygons
        .iter()
        .map(|polygon| bond_axis_from_points(polygon).expect("bond axis"))
        .collect();
    for (from, to) in &axes {
        assert!(
            (from.x - to.x).abs() <= 0.15,
            "BeginAttach=6 must make the P=O center double vertical: {axes:?}"
        );
    }
    assert!(
        (axes[0].0.distance(axes[0].1) - axes[1].0.distance(axes[1].1)).abs() <= 1.0e-4,
        "the two P=O strokes must remain equal length: {axes:?}"
    );

    let first_export = document_to_cdxml(&document);
    assert!(first_export.contains("BeginAttach=\"6\""), "{first_export}");
    let reopened = parse_cdxml_document(&first_export, Some("internal label attachment"))
        .expect("exported CDXML should reopen");
    let second_export = document_to_cdxml(&reopened);
    assert_eq!(
        second_export, first_export,
        "attachment export must stabilize"
    );
}

#[test]
fn render_document_clips_center_double_lines_against_glyph_polygon_only() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "C",
                "atomicNumber": 6,
                "position": [50.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "isPlaceholder": true,
                "label": {
                    "text": "•",
                    "position": [49.0, 42.5],
                    "box": [49.4, 36.5, 50.6, 43.5],
                    "glyphPolygons": [[
                        [49.4, 36.5],
                        [50.6, 36.5],
                        [50.6, 43.5],
                        [49.4, 43.5]
                    ]]
                }
            },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [80.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.6,
                "labelClipMargin": 9.0,
                "bondSpacing": 18.0,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 2,
                "strokeWidth": 0.6,
                "labelClipMargin": 9.0,
                "bondSpacing": 18.0,
                "double": { "placement": "center" }
            }
        ]),
    );

    let polygons = object_bond_polygons_with_ids(&render_document(&document));
    let b1_axes: Vec<_> = polygons
        .iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| bond_axis_from_points(points).expect("b1 axis"))
        .collect();
    let b2_axes: Vec<_> = polygons
        .iter()
        .filter(|(bond_id, _)| bond_id == "b2")
        .map(|(_, points)| bond_axis_from_points(points).expect("b2 axis"))
        .collect();

    assert_eq!(b1_axes.len(), 2, "{polygons:?}");
    assert_eq!(b2_axes.len(), 2, "{polygons:?}");
    for (from, to) in b1_axes {
        let label_endpoint_x = from.x.max(to.x);
        assert!(
            label_endpoint_x <= 49.45,
            "left center-double line should stop at the dot glyph polygon, ignoring legacy labelClipMargin: {polygons:?}"
        );
    }
    for (from, to) in b2_axes {
        let label_endpoint_x = from.x.min(to.x);
        assert!(
            label_endpoint_x >= 50.55,
            "right center-double line should stop at the dot glyph polygon, ignoring legacy labelClipMargin: {polygons:?}"
        );
    }
}

#[test]
fn render_document_keeps_terminal_side_double_offset_with_label_retreat() {
    let unlabeled = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "N", "atomicNumber": 7, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            }
        ]),
    );
    let labeled = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "N",
                "atomicNumber": 7,
                "position": [56.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [56.0, 45.0],
                    "box": [51.0, 34.0, 61.0, 46.0]
                }
            }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            }
        ]),
    );

    let unlabeled_polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&unlabeled))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    let labeled_polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&labeled))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(unlabeled_polygons.len(), 2);
    assert_eq!(labeled_polygons.len(), 2);

    let unlabeled_axes: Vec<_> = unlabeled_polygons
        .iter()
        .map(|polygon| bond_axis_from_points(polygon).expect("bond axis"))
        .collect();
    let labeled_axes: Vec<_> = labeled_polygons
        .iter()
        .map(|polygon| bond_axis_from_points(polygon).expect("bond axis"))
        .collect();
    let unlabeled_gap = ((unlabeled_axes[0].0.y + unlabeled_axes[0].1.y) * 0.5
        - (unlabeled_axes[1].0.y + unlabeled_axes[1].1.y) * 0.5)
        .abs();
    let labeled_gap = ((labeled_axes[0].0.y + labeled_axes[0].1.y) * 0.5
        - (labeled_axes[1].0.y + labeled_axes[1].1.y) * 0.5)
        .abs();

    assert!(
        (unlabeled_gap - labeled_gap).abs() <= 1.0e-4,
        "{unlabeled_gap} {labeled_gap}"
    );
}

#[test]
fn render_document_ignores_legacy_label_clip_margin_for_glyph_polygons() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "N",
                "atomicNumber": 7,
                "position": [56.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [56.0, 45.0],
                    "box": [51.0, 34.0, 61.0, 46.0],
                    "glyphPolygons": [[
                        [51.0, 34.0],
                        [61.0, 34.0],
                        [61.0, 46.0],
                        [51.0, 46.0]
                    ]]
                }
            }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85, "labelClipMargin": 9.0 }
        ]),
    );

    let polygon = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("bond polygon should render");
    let (from, to) = bond_axis_from_points(&polygon).expect("bond axis");
    let label_endpoint = if from.x > to.x { from } else { to };

    assert!(
        (label_endpoint.x - 51.0).abs() < 0.02,
        "glyph polygon clipping should ignore legacy labelClipMargin and avoid adding a second margin retreat: {polygon:?}"
    );
}

#[test]
fn render_document_treats_horizontal_label_interior_as_rectangular_clip() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [27.0, 40.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "C",
                "atomicNumber": 6,
                "position": [27.0, 16.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "Ph",
                    "position": [27.0, 20.0],
                    "box": [20.0, 10.0, 34.0, 22.0],
                    "runs": [{ "text": "Ph", "fontFamily": "Arial", "fontSize": 10.0, "script": "normal" }],
                    "glyphPolygons": [
                        [[20.0, 10.0], [24.0, 10.0], [24.0, 22.0], [20.0, 22.0]],
                        [[30.0, 10.0], [34.0, 10.0], [34.0, 22.0], [30.0, 22.0]]
                    ]
                }
            }
        ]),
        json!([{ "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }]),
    );

    let polygon = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("bond polygon should render");
    let label_edge_y = polygon
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);

    assert!(
        (label_edge_y - 22.0).abs() < 0.02,
        "horizontal multi-character labels should bridge only the overlapping internal glyph gap without adding a second margin retreat: {polygon:?}"
    );
}

#[test]
fn render_document_rebuilds_vertically_separated_label_clip_from_runs() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [45.0, 20.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "N",
                "atomicNumber": 7,
                "position": [22.0, 20.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "NH",
                    "position": [22.0, 24.0],
                    "box": [20.0, 6.0, 34.0, 26.0],
                    "runs": [
                        { "text": "N", "fontFamily": "Arial", "fontSize": 10.0, "script": "normal" },
                        { "text": "H", "fontFamily": "Arial", "fontSize": 7.0, "script": "superscript" }
                    ],
                    "glyphPolygons": [
                        [[20.0, 14.0], [24.0, 14.0], [24.0, 26.0], [20.0, 26.0]],
                        [[30.0, 6.0], [34.0, 6.0], [34.0, 12.0], [30.0, 12.0]]
                    ]
                }
            }
        ]),
        json!([{ "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 }]),
    );
    let document = normalize_test_document(&document);

    let label = document
        .editable_fragment()
        .expect("fragment")
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "n2")
        .and_then(|node| node.label.as_ref())
        .expect("label");
    let normal_bounds = label.glyph_polygons[0]
        .iter()
        .fold([f64::INFINITY, f64::NEG_INFINITY], |bounds, point| {
            [bounds[0].min(point[1]), bounds[1].max(point[1])]
        });
    let superscript_bounds = label.glyph_polygons[1]
        .iter()
        .fold([f64::INFINITY, f64::NEG_INFINITY], |bounds, point| {
            [bounds[0].min(point[1]), bounds[1].max(point[1])]
        });
    assert!(superscript_bounds[1] < normal_bounds[1]);

    let polygon = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("bond polygon should render");
    let (from, to) = bond_axis_from_points(&polygon).expect("bond axis");
    let label_endpoint = if from.x < to.x { from } else { to };

    assert!(
        label_endpoint.x > 24.5,
        "rendering must use the rebuilt real-outline retreat rather than the stale authored rectangles: {polygon:?}"
    );
}

#[test]
fn render_document_clips_solid_wedge_wide_endpoint_against_outline_lines() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [42.0, 54.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "C",
                "atomicNumber": 6,
                "position": [27.0, 70.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "t-Bu",
                    "position": [27.0, 73.5],
                    "box": [12.0, 65.0, 29.3, 72.4],
                    "glyphPolygons": [
                        [[12.0, 65.0], [16.0, 65.0], [16.0, 72.4], [12.0, 72.4]],
                        [[17.0, 65.0], [21.0, 65.0], [21.0, 72.4], [17.0, 72.4]],
                        [[22.0, 65.0], [25.0, 65.0], [25.0, 72.4], [22.0, 72.4]],
                        [[26.0, 65.0], [29.3, 65.0], [29.3, 72.4], [26.0, 72.4]]
                    ]
                }
            }
        ]),
        json!([{
            "id": "b1",
            "begin": "n1",
            "end": "n2",
            "order": 1,
            "strokeWidth": 0.6,
            "wedgeWidth": 2.0,
            "stereo": { "kind": "solid-wedge", "wideEnd": "end" }
        }]),
    );
    let document = normalize_test_document(&document);

    let polygon = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("solid wedge should render");
    let cap_center = Point::new(
        (polygon[1].x + polygon[2].x) * 0.5,
        (polygon[1].y + polygon[2].y) * 0.5,
    );

    assert!(
        polygon[1].x >= 29.3 - 0.02 && polygon[2].x > 30.0 && cap_center.x > 29.9,
        "solid wedge should use the most conservative label retreat from its center and outline lines: {polygon:?}"
    );
}

#[test]
fn render_document_clips_hashed_wedge_wide_endpoint_against_outline_lines() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [42.0, 54.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "C",
                "atomicNumber": 6,
                "position": [27.0, 70.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "t-Bu",
                    "position": [27.0, 73.5],
                    "box": [12.0, 65.0, 29.3, 72.4],
                    "glyphPolygons": [
                        [[12.0, 65.0], [16.0, 65.0], [16.0, 72.4], [12.0, 72.4]],
                        [[17.0, 65.0], [21.0, 65.0], [21.0, 72.4], [17.0, 72.4]],
                        [[22.0, 65.0], [25.0, 65.0], [25.0, 72.4], [22.0, 72.4]],
                        [[26.0, 65.0], [29.3, 65.0], [29.3, 72.4], [26.0, 72.4]]
                    ]
                }
            }
        ]),
        json!([{
            "id": "b1",
            "begin": "n1",
            "end": "n2",
            "order": 1,
            "strokeWidth": 0.6,
            "wedgeWidth": 2.0,
            "stereo": { "kind": "hashed-wedge", "wideEnd": "end" }
        }]),
    );
    let document = normalize_test_document(&document);

    let points = object_bond_points_for_id(&render_document(&document), "b1");
    assert!(!points.is_empty(), "hashed wedge should render stripes");
    let cap_points = closest_points_to_target(&points, Point::new(27.0, 70.0), 2);
    let cap_center_x =
        cap_points.iter().map(|point| point.x).sum::<f64>() / cap_points.len() as f64;

    assert!(
        cap_center_x > 29.9,
        "hashed wedge label clipping should use the same outline-aware retreat as solid wedges: {points:?}"
    );
}

#[test]
fn render_document_acs_template_does_not_add_label_clip_margin() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n2",
                "element": "N",
                "atomicNumber": 7,
                "position": [56.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [56.0, 45.0],
                    "box": [51.0, 34.0, 61.0, 46.0],
                    "glyphPolygons": [[
                        [51.0, 34.0],
                        [61.0, 34.0],
                        [61.0, 46.0],
                        [51.0, 46.0]
                    ]]
                }
            }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.6,
                "boldWidth": 2.0,
                "hashSpacing": 2.5,
                "bondSpacing": 18.0
            }
        ]),
    );

    let polygon = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("bond polygon should render");
    let (from, to) = bond_axis_from_points(&polygon).expect("bond axis");
    let label_endpoint = if from.x > to.x { from } else { to };
    let margin = 51.0 - label_endpoint.x;

    assert!(
        margin.abs() < 0.02,
        "ACS label clipping should use the source-margin glyph polygon without adding a second margin retreat: {margin} {polygon:?}"
    );
}

#[test]
fn render_document_keeps_center_double_parallel_with_branches_and_labeled_endpoint() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [44.0, 22.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [44.0, 58.0], "charge": 0, "numHydrogens": 0 },
            {
                "id": "n4",
                "element": "N",
                "atomicNumber": 7,
                "position": [92.0, 40.0],
                "charge": 0,
                "numHydrogens": 0,
                "label": {
                    "text": "N",
                    "position": [92.0, 45.0],
                    "box": [87.0, 34.0, 97.0, 46.0]
                }
            },
            { "id": "n5", "element": "C", "atomicNumber": 6, "position": [110.0, 22.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n2", "end": "n4", "order": 2, "strokeWidth": 0.85, "double": { "placement": "center" } },
            { "id": "b2", "begin": "n2", "end": "n1", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b4", "begin": "n4", "end": "n5", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let polygons: Vec<_> = object_bond_polygons_with_ids(&render_document(&document))
        .into_iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points)
        .collect();
    assert_eq!(polygons.len(), 2, "{polygons:?}");

    let axes: Vec<_> = polygons
        .iter()
        .map(|polygon| bond_axis_from_points(polygon).expect("bond axis"))
        .collect();
    let first_direction =
        chemsema_engine::Point::new(axes[0].1.x - axes[0].0.x, axes[0].1.y - axes[0].0.y);
    let second_direction =
        chemsema_engine::Point::new(axes[1].1.x - axes[1].0.x, axes[1].1.y - axes[1].0.y);
    let first_angle = first_direction.y.atan2(first_direction.x).to_degrees();
    let second_angle = second_direction.y.atan2(second_direction.x).to_degrees();

    assert!(
        angular_distance(first_angle, second_angle) <= 1.0e-4,
        "{polygons:?} {first_angle} {second_angle}"
    );
}

#[test]
fn render_document_extends_center_double_lines_to_branch_bonds_and_branches_join_each_other() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [74.0, 68.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            },
            {
                "id": "b3",
                "begin": "n2",
                "end": "n4",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let polygons = object_bond_polygons_with_ids(&render_document(&document));
    let center_double: Vec<_> = polygons
        .iter()
        .filter(|(bond_id, _)| bond_id == "b1")
        .map(|(_, points)| points.clone())
        .collect();
    assert_eq!(center_double.len(), 2);
    let branch_up = polygons
        .iter()
        .find(|(bond_id, _)| bond_id == "b2")
        .map(|(_, points)| points.clone())
        .expect("upper branch polygon");
    let branch_down = polygons
        .iter()
        .find(|(bond_id, _)| bond_id == "b3")
        .map(|(_, points)| points.clone())
        .expect("lower branch polygon");

    assert!(
        center_double.iter().all(|polygon| {
            let end_points = [polygon[1], polygon[2]];
            end_points
                .iter()
                .all(|point| point_lies_on_polygon_boundary(*point, &branch_up, 1.0e-4))
                || end_points
                    .iter()
                    .all(|point| point_lies_on_polygon_boundary(*point, &branch_down, 1.0e-4))
        }),
        "{center_double:?} {branch_up:?} {branch_down:?}"
    );
    assert!(center_double.iter().any(|polygon| {
        [polygon[1], polygon[2]]
            .iter()
            .all(|point| point_lies_on_polygon_boundary(*point, &branch_up, 1.0e-4))
    }));
    assert!(center_double.iter().any(|polygon| {
        [polygon[1], polygon[2]]
            .iter()
            .all(|point| point_lies_on_polygon_boundary(*point, &branch_down, 1.0e-4))
    }));
    assert!(shared_point_count(&branch_up, &branch_down, 1.0e-4) >= 2);
}

#[test]
fn render_document_joins_same_side_double_outer_polygons() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "left" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "left" }
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let b1_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b1",
        chemsema_engine::Point::new(56.0, 40.0),
    );
    let b2_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b2",
        chemsema_engine::Point::new(56.0, 40.0),
    );
    let b1_main =
        side_double_main_polygon_for_bond(&polygons, "b1", chemsema_engine::Point::new(56.0, 40.0));
    let b2_main =
        side_double_main_polygon_for_bond(&polygons, "b2", chemsema_engine::Point::new(56.0, 40.0));

    assert_eq!(shared_point_count(&b1_outer, &b2_outer, 1.0e-4), 2);
    assert_eq!(shared_point_count(&b1_outer, &b1_main, 1.0e-4), 0);
    assert_eq!(shared_point_count(&b2_outer, &b2_main, 1.0e-4), 0);
}

#[test]
fn render_document_keeps_opposite_side_double_outer_polygons_inset() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "left" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let b1_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b1",
        chemsema_engine::Point::new(56.0, 40.0),
    );
    let b2_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b2",
        chemsema_engine::Point::new(56.0, 40.0),
    );

    assert_eq!(shared_point_count(&b1_outer, &b2_outer, 1.0e-4), 0);
}

#[test]
fn render_document_joins_side_double_outer_polygon_for_straight_through_180_degrees() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [92.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let b1_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b1",
        chemsema_engine::Point::new(56.0, 40.0),
    );
    let b2_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b2",
        chemsema_engine::Point::new(56.0, 40.0),
    );

    assert_eq!(shared_point_count(&b1_outer, &b2_outer, 1.0e-4), 2);
}

#[test]
fn render_document_joins_inner_side_double_outer_polygon_against_triple_outer_polygon() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 12.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 3,
                "strokeWidth": 0.85
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let b1_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b1",
        chemsema_engine::Point::new(56.0, 40.0),
    );
    let triple_shared = polygons
        .iter()
        .filter(|(bond_id, _)| bond_id == "b2")
        .any(|(_, points)| shared_point_count(&b1_outer, points, 1.0e-4) == 2);

    assert!(triple_shared, "{polygons:?}");
}

#[test]
fn render_document_retreats_side_double_outer_polygon_for_acute_single_bond_angles() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [30.0, 60.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let b1_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b1",
        chemsema_engine::Point::new(56.0, 40.0),
    );
    let single_shared = polygons
        .iter()
        .filter(|(bond_id, _)| bond_id == "b2")
        .any(|(_, points)| shared_point_count(&b1_outer, points, 1.0e-4) == 2);

    assert!(!single_shared, "{polygons:?}");
}

#[test]
fn render_document_retreats_side_double_outer_polygon_against_center_double_reference_axis() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [30.0, 60.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "center" }
            }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let side_double_outer = side_double_outer_polygon_for_bond(
        &polygons,
        "b1",
        chemsema_engine::Point::new(56.0, 40.0),
    );
    let centered_double: Vec<_> = polygons
        .iter()
        .filter(|(bond_id, _)| bond_id == "b2")
        .collect();

    assert!(centered_double
        .iter()
        .all(|(_, points)| { shared_point_count(&side_double_outer, points, 1.0e-4) == 0 }));
}

#[test]
fn render_document_scales_triple_offset_with_bond_length() {
    let short_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 3, "strokeWidth": 0.85 }
        ]),
    );
    let long_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 80.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [92.0, 80.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 3, "strokeWidth": 0.85 }
        ]),
    );

    let short_lines = object_bond_centerlines(&render_document(&short_document));
    let long_lines = object_bond_centerlines(&render_document(&long_document));

    let short_offset = short_lines
        .iter()
        .map(|(from, to)| ((from.y + to.y) / 2.0 - 40.0).abs())
        .max_by(|a, b| a.total_cmp(b))
        .unwrap();
    let long_offset = long_lines
        .iter()
        .map(|(from, to)| ((from.y + to.y) / 2.0 - 80.0).abs())
        .max_by(|a, b| a.total_cmp(b))
        .unwrap();

    assert!(
        (long_offset - short_offset * 2.0).abs() < 0.05,
        "{short_offset} {long_offset}"
    );
}

#[test]
fn render_document_keeps_solid_wedge_cap_width_constant_when_bond_is_longer() {
    let short_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "solid-wedge",
                    "wideEnd": "end"
                }
            }
        ]),
    );
    let long_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 80.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [92.0, 80.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "solid-wedge",
                    "wideEnd": "end"
                }
            }
        ]),
    );

    let short_polygon = render_document(&short_document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("short wedge polygon");
    let long_polygon = render_document(&long_document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("long wedge polygon");

    let short_width = ((short_polygon[1].x - short_polygon[2].x).powi(2)
        + (short_polygon[1].y - short_polygon[2].y).powi(2))
    .sqrt();
    let long_width = ((long_polygon[1].x - long_polygon[2].x).powi(2)
        + (long_polygon[1].y - long_polygon[2].y).powi(2))
    .sqrt();

    assert!(
        (short_width - long_width).abs() < 0.05,
        "{short_width} {long_width}"
    );
}

#[test]
fn render_document_uses_explicit_solid_wedge_wide_and_tip_widths() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [34.4, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.6,
                "boldWidth": 2.0,
                "wedgeWidth": 2.0,
                "stereo": {
                    "kind": "solid-wedge",
                    "wideEnd": "end"
                }
            }
        ]),
    );

    let polygon = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("solid wedge polygon");

    let tip_width =
        ((polygon[0].x - polygon[3].x).powi(2) + (polygon[0].y - polygon[3].y).powi(2)).sqrt();
    let wide_width =
        ((polygon[1].x - polygon[2].x).powi(2) + (polygon[1].y - polygon[2].y).powi(2)).sqrt();

    assert!((tip_width - 0.6).abs() < 0.01, "{tip_width}");
    assert!((wide_width - 2.0).abs() < 0.01, "{wide_width}");
}

#[test]
fn render_document_uses_acs_template_wedge_width_for_legacy_json_without_wedge_width() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [34.4, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 1,
                "strokeWidth": 0.6,
                "boldWidth": 2.0,
                "hashSpacing": 2.5,
                "bondSpacing": 18.0,
                "stereo": {
                    "kind": "solid-wedge",
                    "wideEnd": "end"
                }
            }
        ]),
    );

    let polygon = render_document(&document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                role,
                object_id,
                points,
                ..
            } if role == RenderRole::DocumentBond
                && object_id.as_deref() == Some("obj_molecule_001") =>
            {
                Some(points)
            }
            _ => None,
        })
        .expect("solid wedge polygon");

    let wide_width =
        ((polygon[1].x - polygon[2].x).powi(2) + (polygon[1].y - polygon[2].y).powi(2)).sqrt();

    assert!((wide_width - 3.0).abs() < 0.01, "{wide_width}");
}

#[test]
fn render_document_emits_three_way_main_contact_patches() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 8.82], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [74.0, 71.18], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 },
            { "id": "b2", "begin": "n1", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n1", "end": "n4", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = centered_bond_polygons(&primitives, chemsema_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 3);
    assert_eq!(
        polygons.iter().filter(|points| points.len() == 5).count(),
        3
    );
    assert!(polygons.iter().all(|points| polygon_area(points) > 0.01));
}

#[test]
fn render_document_clips_solid_wedge_in_three_way_main_contact() {
    for (begin, end, wide_end) in [("n1", "n3", "begin"), ("n3", "n1", "end")] {
        let document = fragment_document(
            json!([
                { "id": "n1", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
                { "id": "n2", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
                { "id": "n3", "element": "C", "atomicNumber": 6, "position": [96.0, 4.0], "charge": 0, "numHydrogens": 0 },
                { "id": "n4", "element": "C", "atomicNumber": 6, "position": [68.0, 88.0], "charge": 0, "numHydrogens": 0 }
            ]),
            json!([
                {
                    "id": "b1",
                    "begin": begin,
                    "end": end,
                    "order": 1,
                    "strokeWidth": 0.85,
                    "stereo": {
                        "kind": "solid-wedge",
                        "wideEnd": wide_end
                    }
                },
                { "id": "b2", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 },
                { "id": "b3", "begin": "n1", "end": "n4", "order": 1, "strokeWidth": 0.85 }
            ]),
        );

        let primitives = render_document(&document);
        let polygons = object_bond_polygons_with_ids(&primitives);
        let wedge = polygons
            .iter()
            .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
            .expect("solid wedge polygon");
        assert_eq!(wedge.len(), 5, "{wide_end} {wedge:?}");
        assert!(wedge
            .iter()
            .any(|point| point.distance(chemsema_engine::Point::new(56.0, 40.0)) <= 0.001));

        let centered = centered_bond_polygons(&primitives, chemsema_engine::Point::new(56.0, 40.0));
        assert_eq!(centered.len(), 3, "{wide_end} {centered:?}");
        assert!(centered.iter().all(|points| polygon_area(points) > 0.01));
        let center_patches = polygons
            .iter()
            .filter_map(|(bond_id, points)| bond_id.is_empty().then_some(points))
            .collect::<Vec<_>>();
        assert!(center_patches.is_empty(), "{wide_end} {polygons:?}");
    }
}

#[test]
fn render_document_uses_extended_intersections_for_solid_wedge_three_way_contact() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [cdxml_cm_to_pt(7.5), cdxml_cm_to_pt(6.5)], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [cdxml_cm_to_pt(6.45), cdxml_cm_to_pt(6.5)], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [cdxml_cm_to_pt(7.682330586550277), cdxml_cm_to_pt(5.465951859337181)], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [cdxml_cm_to_pt(7.859121150491952), cdxml_cm_to_pt(7.486677251825204)], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b_left", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": cdxml_cm_to_pt(0.035) },
            { "id": "b_up", "begin": "n1", "end": "n3", "order": 1, "strokeWidth": cdxml_cm_to_pt(0.035) },
            {
                "id": "b_wedge",
                "begin": "n1",
                "end": "n4",
                "order": 1,
                "strokeWidth": cdxml_cm_to_pt(0.035),
                "stereo": {
                    "kind": "solid-wedge",
                    "wideEnd": "begin"
                }
            }
        ]),
    );

    let expected_up_wedge_intersection =
        chemsema_engine::Point::new(214.1234207734643, 178.46000173212423);
    let contact_center = chemsema_engine::Point::new(cdxml_cm_to_pt(7.5), cdxml_cm_to_pt(6.5));
    let polygons = object_bond_polygons_with_ids(&render_document(&document));
    let up = polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b_up").then_some(points))
        .expect("upper single polygon");
    let wedge = polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b_wedge").then_some(points))
        .expect("solid wedge polygon");

    assert!(
        up.iter()
            .any(|point| point.distance(expected_up_wedge_intersection) <= cdxml_cm_to_pt(0.001)),
        "{up:?}"
    );
    assert!(
        wedge
            .iter()
            .any(|point| point.distance(expected_up_wedge_intersection) <= cdxml_cm_to_pt(0.001)),
        "{wedge:?}"
    );
    let has_edge = |points: &[chemsema_engine::Point],
                    first: chemsema_engine::Point,
                    second: chemsema_engine::Point| {
        (0..points.len()).any(|index| {
            let next = (index + 1) % points.len();
            (points[index].distance(first) <= cdxml_cm_to_pt(0.001)
                && points[next].distance(second) <= cdxml_cm_to_pt(0.001))
                || (points[index].distance(second) <= cdxml_cm_to_pt(0.001)
                    && points[next].distance(first) <= cdxml_cm_to_pt(0.001))
        })
    };
    assert!(
        has_edge(up, expected_up_wedge_intersection, contact_center),
        "{up:?}"
    );
    assert!(
        has_edge(wedge, expected_up_wedge_intersection, contact_center),
        "{wedge:?}"
    );
}

#[test]
fn render_document_emits_four_way_main_contact_patches() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [92.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n4", "element": "C", "atomicNumber": 6, "position": [56.0, 4.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n5", "element": "C", "atomicNumber": 6, "position": [56.0, 76.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            { "id": "b1", "begin": "n1", "end": "n2", "order": 1, "strokeWidth": 0.85 },
            { "id": "b2", "begin": "n1", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n1", "end": "n4", "order": 1, "strokeWidth": 0.85 },
            { "id": "b4", "begin": "n1", "end": "n5", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = centered_bond_polygons(&primitives, chemsema_engine::Point::new(56.0, 40.0));
    assert_eq!(polygons.len(), 4);
    assert_eq!(
        polygons.iter().filter(|points| points.len() == 5).count(),
        4
    );
    assert!(polygons.iter().all(|points| polygon_area(points) > 0.01));
}
