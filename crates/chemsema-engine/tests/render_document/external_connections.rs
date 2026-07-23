use super::*;
use chemsema_engine::{ExternalConnectionType, RenderPrimitive};

const TYPES: [(&str, ExternalConnectionType); 13] = [
    ("Unspecified", ExternalConnectionType::Unspecified),
    ("Diamond", ExternalConnectionType::Diamond),
    ("Star", ExternalConnectionType::Star),
    ("PolymerBead", ExternalConnectionType::PolymerBead),
    ("Wavy", ExternalConnectionType::Wavy),
    ("Residue", ExternalConnectionType::Residue),
    ("Peptide", ExternalConnectionType::Peptide),
    ("DNA", ExternalConnectionType::Dna),
    ("RNA", ExternalConnectionType::Rna),
    ("Terminus", ExternalConnectionType::Terminus),
    ("Sulfide", ExternalConnectionType::Sulfide),
    ("Nucleotide", ExternalConnectionType::Nucleotide),
    ("UnlinkedBranch", ExternalConnectionType::UnlinkedBranch),
];

fn external_connection_cdxml() -> String {
    let nodes = TYPES
        .iter()
        .enumerate()
        .map(|(index, (name, _))| {
            let type_attr = if *name == "Unspecified" {
                String::new()
            } else {
                format!(r#" ExternalConnectionType="{name}""#)
            };
            format!(
                r#"<n id="e{index}" p="{} 30" NodeType="ExternalConnectionPoint"{type_attr} ExternalConnectionNum="{}"/>"#,
                30 + index * 18,
                index + 1
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let bonds = TYPES
        .iter()
        .enumerate()
        .map(|(index, _)| format!(r#"<b id="b{index}" B="center" E="e{index}"/>"#))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="18" LineWidth="0.6" LabelSize="10">
  <page id="1" BoundingBox="0 0 300 80">
    <fragment id="f1">
      <n id="center" p="150 60"/>
      {nodes}
      {bonds}
    </fragment>
  </page>
</CDXML>"#
    )
}

#[test]
fn external_connection_types_and_numbers_are_native_and_roundtrip() {
    let document = parse_cdxml_document(&external_connection_cdxml(), Some("external connections"))
        .expect("external connections should parse");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment");
    assert_eq!(fragment.nodes.len(), TYPES.len() + 1);
    for (index, (_, expected_type)) in TYPES.iter().enumerate() {
        let connection = fragment
            .nodes
            .iter()
            .find(|node| node.id == format!("e{index}"))
            .expect("external node")
            .external_connection
            .as_ref()
            .expect("native external connection");
        assert_eq!(connection.connection_type, *expected_type);
        assert_eq!(connection.number, Some((index + 1) as u16));
    }

    let json = serde_json::to_value(&document).expect("serialize CCJS");
    let nodes = json
        .pointer("/resources")
        .and_then(serde_json::Value::as_object)
        .and_then(|resources| {
            resources
                .values()
                .find_map(|resource| resource.pointer("/data/nodes"))
        })
        .and_then(serde_json::Value::as_array)
        .expect("CCJS nodes");
    assert!(nodes
        .iter()
        .filter(|node| {
            node.get("id")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|id| id.starts_with('e'))
        })
        .all(|node| node.get("externalConnection").is_some()));
    assert!(nodes
        .iter()
        .all(|node| node.get("isExternalConnectionPoint").is_none()));

    let exported = document_to_cdxml(&document);
    for (index, (name, _)) in TYPES.iter().enumerate() {
        if *name != "Unspecified" {
            assert!(
                exported.contains(&format!(r#"ExternalConnectionType="{name}""#)),
                "missing {name}"
            );
        }
        assert!(exported.contains(&format!(r#"ExternalConnectionNum="{}""#, index + 1)));
    }
}

#[test]
fn invalid_external_connection_values_are_rejected_instead_of_falling_back() {
    let invalid_type = r#"<CDXML><page><fragment><n id="n1" p="10 10" NodeType="ExternalConnectionPoint" ExternalConnectionType="FutureType"/></fragment></page></CDXML>"#;
    let error = parse_cdxml_document(invalid_type, None).expect_err("unknown type must fail");
    assert!(error.contains("invalid ExternalConnectionType `FutureType`"));

    let invalid_number = r#"<CDXML><page><fragment><n id="n1" p="10 10" NodeType="ExternalConnectionPoint" ExternalConnectionNum="not-a-number"/></fragment></page></CDXML>"#;
    let error = parse_cdxml_document(invalid_number, None).expect_err("invalid number must fail");
    assert!(error.contains("invalid ExternalConnectionNum `not-a-number`"));
}

#[test]
fn external_connection_markers_use_chemdraw_size_rules_and_retreat() {
    let source = r#"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="60" LineWidth="0.6" LabelSize="10">
  <page id="1" BoundingBox="0 0 160 100">
    <fragment id="f1">
      <n id="a" p="40 50"/>
      <n id="diamond" p="100 50" NodeType="ExternalConnectionPoint" ExternalConnectionType="Diamond"/>
      <b id="b1" B="a" E="diamond"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(source, Some("diamond marker")).expect("CDXML");
    let primitives = render_document(&document);
    let polygon = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Polygon {
                node_id: Some(node_id),
                points,
                ..
            } if node_id == "diamond" => Some(points),
            _ => None,
        })
        .expect("diamond polygon");
    let radius = polygon[2].x - polygon[1].x;
    assert_close(radius, 10.0 * 0.375 + 0.6);

    let bond_end_x = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Line {
                bond_id: Some(bond_id),
                from,
                to,
                ..
            } if bond_id == "b1" => Some(from.x.max(to.x)),
            RenderPrimitive::Polygon {
                bond_id: Some(bond_id),
                points,
                ..
            } if bond_id == "b1" => points.iter().map(|point| point.x).reduce(f64::max),
            _ => None,
        })
        .reduce(f64::max)
        .expect("rendered bond");
    assert_close(polygon[1].x - bond_end_x, radius);
}

#[test]
fn wavy_external_connection_uses_chemdraw_span_frequency_and_bond_orientation() {
    let source = r#"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BondLength="60" LineWidth="0.6" LabelSize="10">
  <page id="1" BoundingBox="0 0 160 100">
    <fragment id="f1">
      <n id="a" p="40 50"/>
      <n id="wavy" p="100 50" NodeType="ExternalConnectionPoint" ExternalConnectionType="Wavy"/>
      <b id="b1" B="a" E="wavy"/>
    </fragment>
  </page>
</CDXML>"#;
    let document = parse_cdxml_document(source, Some("wavy marker")).expect("CDXML");
    let primitives = render_document(&document);
    let (path, points) = primitives
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Path {
                d, points, bond_id, ..
            } if bond_id.is_none() && d.starts_with('M') => Some((d, points)),
            _ => None,
        })
        .expect("wavy marker path");

    let raw_span: f64 = 10.0 * 1.5 + 0.6 * 4.0;
    let span = raw_span.round();
    let segments = (raw_span * 2.0_f64).ceil() as usize;
    assert_eq!(path.matches(" C ").count(), segments);
    assert_eq!(points.len(), segments * 4);
    let first = points.first().expect("path start");
    let last = points.last().expect("path end");
    assert_close((last.y - first.y).abs(), span);
    assert_close((last.x - first.x).abs(), 0.5);
}

#[test]
fn legacy_boolean_external_connection_migrates_at_json_boundary() {
    let mut document = fragment_document(
        json!([{
            "id": "n1",
            "element": "",
            "atomicNumber": 0,
            "position": [20.0, 20.0],
            "charge": 0,
            "numHydrogens": 0,
            "isExternalConnectionPoint": true
        }]),
        json!([]),
    );
    let value = serde_json::to_value(&document).expect("serialize");
    let mut legacy = value;
    let resources = legacy["resources"].as_object_mut().expect("resources");
    let node = resources
        .values_mut()
        .find_map(|resource| resource.pointer_mut("/data/nodes/0"))
        .expect("node");
    node.as_object_mut()
        .expect("node object")
        .remove("externalConnection");
    node.as_object_mut().expect("node object").insert(
        "isExternalConnectionPoint".to_string(),
        serde_json::Value::Bool(true),
    );
    document = parse_document_json(&legacy.to_string()).expect("legacy CCJS should migrate");
    let fragment = document
        .resources
        .values()
        .find_map(|resource| resource.data.as_fragment())
        .expect("fragment");
    assert_eq!(
        fragment.nodes[0]
            .external_connection
            .as_ref()
            .map(|connection| connection.connection_type),
        Some(ExternalConnectionType::Unspecified)
    );
}

#[test]
fn external_connection_editing_is_undoable_and_exposed_by_the_atom_menu() {
    let document = fragment_document(
        json!([{
            "id": "n1",
            "element": "C",
            "atomicNumber": 6,
            "position": [20.0, 20.0],
            "charge": 0,
            "numHydrogens": 0
        }]),
        json!([]),
    );
    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).expect("serialize"))
        .expect("load document");
    engine.select_at_point(Point::new(20.0, 20.0), false);
    assert_eq!(engine.state().selection.nodes, vec!["n1"]);

    let menu = engine.context_menu_json(r#"{"kind":"atom","nodeId":"n1"}"#, false);
    assert!(menu.contains("External Connection"), "{menu}");
    assert!(menu.contains("unlinked-branch"), "{menu}");

    assert!(
        engine.set_atom_property_for_selection("external-connection-type", Some("polymer-bead"))
    );
    assert!(engine.set_atom_property_for_selection("external-connection-number", Some("7")));
    let node = &engine
        .state()
        .document
        .editable_fragment()
        .expect("fragment")
        .fragment
        .nodes[0];
    assert_eq!(node.atomic_number, 0);
    assert_eq!(
        node.external_connection.as_ref(),
        Some(&chemsema_engine::ExternalConnection {
            connection_type: ExternalConnectionType::PolymerBead,
            number: Some(7),
        })
    );

    assert!(engine.undo());
    assert_eq!(
        engine
            .state()
            .document
            .editable_fragment()
            .expect("fragment")
            .fragment
            .nodes[0]
            .external_connection
            .as_ref()
            .and_then(|connection| connection.number),
        None
    );
    assert!(engine.undo());
    let node = &engine
        .state()
        .document
        .editable_fragment()
        .expect("fragment")
        .fragment
        .nodes[0];
    assert_eq!(node.atomic_number, 6);
    assert!(node.external_connection.is_none());
}
