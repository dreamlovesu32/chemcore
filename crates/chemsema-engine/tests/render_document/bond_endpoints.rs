use super::*;

#[test]
fn render_document_keeps_hash_bond_label_clip_without_extra_hash_retreat() {
    let labeled_nodes = json!([
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
                "position": [52.0, 44.0],
                "box": [50.0, 34.0, 62.0, 46.0],
                "runs": [{ "text": "N", "fontFamily": "Arial", "fontSize": 10.0, "fill": "#000000" }]
            }
        }
    ]);
    let hash_bond = json!({
        "id": "b1",
        "begin": "n1",
        "end": "n2",
        "order": 1,
        "strokeWidth": 0.85,
        "lineStyles": { "main": "hash", "left": "solid", "right": "solid" },
        "lineWeights": { "main": "normal", "left": "normal", "right": "normal" }
    });
    let isolated = fragment_document(labeled_nodes.clone(), json!([hash_bond.clone()]));
    let connected = fragment_document(
        json!([
            labeled_nodes[0].clone(),
            labeled_nodes[1].clone(),
            { "id": "n3", "element": "C", "atomicNumber": 6, "position": [74.0, 58.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            hash_bond,
            { "id": "b2", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 }
        ]),
    );
    let isolated_hash = object_bond_polygons_with_ids(&render_document(&isolated))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("isolated hash polygon");
    let connected_hash = object_bond_polygons_with_ids(&render_document(&connected))
        .into_iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points))
        .expect("connected hash polygon");

    assert!(polygons_have_same_vertices(
        &isolated_hash,
        &connected_hash,
        1.0e-4,
    ));
}

#[test]
fn render_document_retreats_hash_bond_segments_against_center_double_outer_line() {
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
                "strokeWidth": 0.85,
                "lineStyles": {
                    "main": "hash",
                    "left": "solid",
                    "right": "solid"
                },
                "lineWeights": {
                    "main": "normal",
                    "left": "normal",
                    "right": "normal"
                }
            }
        ]),
    );

    let primitives = render_document(&document);
    let hash_bond = object_bond_points_for_id(&primitives, "b2");
    assert!(!hash_bond.is_empty(), "hash bond segments");
    let connected_end =
        closest_points_to_target(&hash_bond, chemsema_engine::Point::new(56.0, 40.0), 2);
    let unit = chemsema_engine::Point::new(18.0, -28.0);
    let unit_length = (unit.x * unit.x + unit.y * unit.y).sqrt();
    let unit_x = unit.x / unit_length;
    let unit_y = unit.y / unit_length;
    let projections: Vec<_> = connected_end
        .iter()
        .map(|point| (point.x - 56.0) * unit_x + (point.y - 40.0) * unit_y)
        .collect();

    assert_eq!(connected_end.len(), 2);
    assert!(
        (projections[0] - projections[1]).abs() <= 1.0e-4,
        "{hash_bond:?} {projections:?}"
    );
    assert!(
        projections.iter().all(|projection| *projection > 0.05),
        "{hash_bond:?} {projections:?}"
    );
    assert!(object_knockout_polygons(&primitives).is_empty());
}

#[test]
fn render_document_retreats_hashed_wedge_stripes_against_center_double_outer_line() {
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
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "hashed-wedge",
                    "wideEnd": "begin"
                }
            }
        ]),
    );

    let primitives = render_document(&document);
    let hashed_wedge = object_bond_points_for_id(&primitives, "b2");
    assert!(!hashed_wedge.is_empty(), "hashed wedge polygons");
    let connected_end =
        closest_points_to_target(&hashed_wedge, chemsema_engine::Point::new(56.0, 40.0), 2);
    let unit = chemsema_engine::Point::new(18.0, -28.0);
    let unit_length = (unit.x * unit.x + unit.y * unit.y).sqrt();
    let unit_x = unit.x / unit_length;
    let unit_y = unit.y / unit_length;
    let projections: Vec<_> = connected_end
        .iter()
        .map(|point| (point.x - 56.0) * unit_x + (point.y - 40.0) * unit_y)
        .collect();

    assert_eq!(connected_end.len(), 2);
    assert!(
        (projections[0] - projections[1]).abs() <= 1.0e-4,
        "{hashed_wedge:?} {projections:?}"
    );
    assert!(
        projections.iter().all(|projection| *projection > 0.05),
        "{hashed_wedge:?} {projections:?}"
    );
}

#[test]
fn render_document_retreats_hash_bond_and_ignores_it_for_other_bond_contacts() {
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
                "order": 1,
                "strokeWidth": 0.85,
                "lineStyles": {
                    "main": "hash",
                    "left": "solid",
                    "right": "solid"
                },
                "lineWeights": {
                    "main": "normal",
                    "left": "normal",
                    "right": "normal"
                }
            },
            { "id": "b2", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n2", "end": "n4", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let primitives = render_document(&document);
    let polygons = object_bond_polygons_with_ids(&primitives);
    let hash_bond = polygons
        .iter()
        .find_map(|(bond_id, points)| (bond_id == "b1").then_some(points.clone()))
        .expect("hash bond polygon");

    assert!(
        average_closest_distance_to_point(&hash_bond, chemsema_engine::Point::new(56.0, 40.0), 2)
            > 1.0,
        "{hash_bond:?}"
    );
}

#[test]
fn render_document_retreats_hashed_wedge_and_ignores_it_for_other_bond_contacts() {
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
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "hashed-wedge",
                    "wideEnd": "end"
                }
            },
            { "id": "b2", "begin": "n2", "end": "n3", "order": 1, "strokeWidth": 0.85 },
            { "id": "b3", "begin": "n2", "end": "n4", "order": 1, "strokeWidth": 0.85 }
        ]),
    );

    let primitives = render_document(&document);
    let hashed_wedge = object_bond_points_for_id(&primitives, "b1");
    assert!(!hashed_wedge.is_empty(), "hashed wedge polygons");

    assert!(
        average_closest_distance_to_point(
            &hashed_wedge,
            chemsema_engine::Point::new(56.0, 40.0),
            2
        ) > 1.0,
        "{hashed_wedge:?}"
    );
}

#[test]
fn render_document_retreats_hash_bond_against_solid_dashed_center_double_outer_line() {
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
                "double": { "placement": "center" },
                "lineStyles": {
                    "left": "solid",
                    "right": "dashed"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85,
                "lineStyles": {
                    "main": "hash",
                    "left": "solid",
                    "right": "solid"
                },
                "lineWeights": {
                    "main": "normal",
                    "left": "normal",
                    "right": "normal"
                }
            }
        ]),
    );

    let primitives = render_document(&document);
    let hash_bond = object_bond_points_for_id(&primitives, "b2");
    assert!(!hash_bond.is_empty(), "hash bond segments");
    let connected_end =
        closest_points_to_target(&hash_bond, chemsema_engine::Point::new(56.0, 40.0), 2);
    let unit = chemsema_engine::Point::new(18.0, -28.0);
    let unit_length = (unit.x * unit.x + unit.y * unit.y).sqrt();
    let unit_x = unit.x / unit_length;
    let unit_y = unit.y / unit_length;
    let projections: Vec<_> = connected_end
        .iter()
        .map(|point| (point.x - 56.0) * unit_x + (point.y - 40.0) * unit_y)
        .collect();

    assert_eq!(connected_end.len(), 2);
    assert!(
        (projections[0] - projections[1]).abs() <= 1.0e-4,
        "{hash_bond:?} {projections:?}"
    );
    assert!(
        projections.iter().all(|projection| *projection > 0.05),
        "{hash_bond:?} {projections:?}"
    );
    assert!(object_knockout_polygons(&primitives).is_empty());
}

#[test]
fn render_document_retreats_hashed_wedge_against_double_dashed_center_double_outer_line() {
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
                "double": { "placement": "center" },
                "lineStyles": {
                    "left": "dashed",
                    "right": "dashed"
                }
            },
            {
                "id": "b2",
                "begin": "n2",
                "end": "n3",
                "order": 1,
                "strokeWidth": 0.85,
                "stereo": {
                    "kind": "hashed-wedge",
                    "wideEnd": "begin"
                }
            }
        ]),
    );

    let primitives = render_document(&document);
    let hashed_wedge = object_bond_points_for_id(&primitives, "b2");
    assert!(!hashed_wedge.is_empty(), "hashed wedge polygons");
    let connected_end =
        closest_points_to_target(&hashed_wedge, chemsema_engine::Point::new(56.0, 40.0), 2);
    let unit = chemsema_engine::Point::new(18.0, -28.0);
    let unit_length = (unit.x * unit.x + unit.y * unit.y).sqrt();
    let unit_x = unit.x / unit_length;
    let unit_y = unit.y / unit_length;
    let projections: Vec<_> = connected_end
        .iter()
        .map(|point| (point.x - 56.0) * unit_x + (point.y - 40.0) * unit_y)
        .collect();

    assert_eq!(connected_end.len(), 2);
    assert!(
        (projections[0] - projections[1]).abs() <= 1.0e-4,
        "{hashed_wedge:?} {projections:?}"
    );
    assert!(
        projections.iter().all(|projection| *projection > 0.05),
        "{hashed_wedge:?} {projections:?}"
    );
}

#[test]
fn render_document_uses_length_percent_with_line_width_floor_for_side_double_offset() {
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
                "order": 2,
                "strokeWidth": 0.85,
                "double": {
                    "placement": "right"
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
                "order": 2,
                "strokeWidth": 0.85,
                "double": {
                    "placement": "right"
                }
            }
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
    let expected_short_offset = (36.0_f64 * 0.12).max(2.5 * 0.85);
    let expected_long_offset = (72.0_f64 * 0.12).max(2.5 * 0.85);
    assert!(
        (short_offset - expected_short_offset).abs() < 0.05,
        "{short_offset}"
    );
    assert!(
        (long_offset - expected_long_offset).abs() < 0.05,
        "{long_offset}"
    );
}

#[test]
fn render_document_increases_side_double_offset_for_bold_main_line() {
    let normal_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
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
    let bold_document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
        ]),
        json!([
            {
                "id": "b1",
                "begin": "n1",
                "end": "n2",
                "order": 2,
                "strokeWidth": 0.85,
                "double": { "placement": "right" },
                "lineWeights": {
                    "main": "bold",
                    "left": "normal",
                    "right": "normal"
                }
            }
        ]),
    );

    let normal_lines = object_bond_centerlines(&render_document(&normal_document));
    let bold_lines = object_bond_centerlines(&render_document(&bold_document));

    let normal_offset = normal_lines
        .iter()
        .map(|(from, to)| ((from.y + to.y) / 2.0 - 40.0).abs())
        .max_by(|a, b| a.total_cmp(b))
        .unwrap();
    let bold_offset = bold_lines
        .iter()
        .map(|(from, to)| ((from.y + to.y) / 2.0 - 40.0).abs())
        .max_by(|a, b| a.total_cmp(b))
        .unwrap();

    assert!(
        bold_offset > normal_offset + 0.01,
        "{normal_offset} {bold_offset}"
    );
}

#[test]
fn render_document_keeps_terminal_side_double_outer_line_equal_length() {
    let document = fragment_document(
        json!([
            { "id": "n1", "element": "C", "atomicNumber": 6, "position": [20.0, 40.0], "charge": 0, "numHydrogens": 0 },
            { "id": "n2", "element": "C", "atomicNumber": 6, "position": [56.0, 40.0], "charge": 0, "numHydrogens": 0 }
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
        "{short_length} {long_length}"
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
