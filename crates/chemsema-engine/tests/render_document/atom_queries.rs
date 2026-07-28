use super::*;

#[test]
fn atom_query_fixture_has_native_fields_labels_and_query_decorations() {
    let source = read_cdxml_fixture("atom-query-properties.cdxml");
    let document = parse_cdxml_document(&source, None).expect("atom-query fixture parses");
    let document_json = serde_json::to_value(&document).expect("document json");
    let nodes = document_json["resources"]
        .as_object()
        .expect("resources")
        .values()
        .flat_map(|resource| resource["data"]["nodes"].as_array().into_iter().flatten())
        .collect::<Vec<_>>();
    let query = nodes
        .iter()
        .find(|node| node["id"] == "101")
        .expect("query node");
    assert_eq!(query["atomProperties"]["freeSites"], 2);
    assert_eq!(query["atomProperties"]["ringBondCount"], "simple-ring");
    assert_eq!(
        query["atomProperties"]["unsaturatedBonds"],
        "must-be-present"
    );
    assert_eq!(query["atomProperties"]["substituentsExactly"], 3);
    assert_eq!(query["atomProperties"]["translation"], "narrow");

    let list = nodes
        .iter()
        .find(|node| node["id"] == "103")
        .expect("list node");
    assert_eq!(list["atomProperties"]["elementList"], json!([7, 8, 15]));
    assert_eq!(list["atomProperties"]["elementListExcluded"], true);
    assert_eq!(list["atomProperties"]["genericList"], json!(["R", "X"]));
    assert_eq!(list["label"]["text"], "NOT N, O, P, R, X");

    let rendered = render_document(&document);
    let texts = rendered
        .iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text { text, runs, .. } => Some(if text.is_empty() {
                runs.iter().map(|run| run.text.as_str()).collect::<String>()
            } else {
                text.clone()
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        texts.iter().filter(|text| text.as_str() == "X3SRL").count(),
        1,
        "{texts:?}"
    );
    assert!(
        texts.iter().any(|text| text == "NOT N, O, P, R, X"),
        "{texts:?}"
    );

    let exported = document_to_cdxml(&document);
    for expected in [
        "ElementList=\"NOT 7 8 15\"",
        "GenericList=\"R X\"",
        "RingBondCount=\"SimpleRing\"",
        "UnsaturatedBonds=\"MustBePresent\"",
        "SubstituentsExactly=\"3\"",
        "Translation=\"Narrow\"",
        "AbnormalValence=\"yes\"",
        "ShowTerminalCarbonLabels=\"yes\"",
    ] {
        assert!(
            exported.contains(expected),
            "missing {expected}\n{exported}"
        );
    }
}

#[test]
fn explicit_atom_query_display_uses_its_object_tag_instead_of_a_derived_duplicate() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LabelFont="3" LabelSize="10" ShowAtomQuery="yes">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1"><fragment id="2">
    <n id="3" p="20 20" FreeSites="2">
      <objecttag Name="query">
        <t p="24 28" BoundingBox="24 22 34 28"><s font="3" size="7.5">*2</s></t>
      </objecttag>
    </n>
  </fragment></page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("authored atom query display"))
        .expect("atom query display should parse");
    let query_objects = document
        .objects
        .iter()
        .filter(|object| {
            object.meta.get("role").and_then(|value| value.as_str()) == Some("query")
                && object
                    .meta
                    .get("attachedNodeId")
                    .and_then(|value| value.as_str())
                    == Some("3")
        })
        .collect::<Vec<_>>();
    assert_eq!(query_objects.len(), 1);

    let query_texts = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text { text, runs, .. } => {
                let rendered = if text.is_empty() {
                    runs.into_iter().map(|run| run.text).collect::<String>()
                } else {
                    text
                };
                (rendered == "*2").then_some(rendered)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(query_texts, vec!["*2"]);
}

#[test]
fn deuterium_and_tritium_labels_encode_their_isotope_mass_once() {
    let cdxml = r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LabelFont="3" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1">
    <fragment id="fragment">
      <n id="deuterium" p="20 20" Element="1" Isotope="2">
        <t p="20 20"><s font="3" size="10">D</s></t>
      </n>
      <n id="tritium" p="50 20" Element="1" Isotope="3">
        <t p="50 20"><s font="3" size="10">T</s></t>
      </n>
      <n id="explicit-hydrogen" p="80 20" Element="1" Isotope="2">
        <t p="80 20"><s font="3" size="10">H</s></t>
      </n>
    </fragment>
  </page>
</CDXML>"##;
    let document = parse_cdxml_document(cdxml, Some("hydrogen isotope shorthand"))
        .expect("hydrogen isotope shorthand parses");
    let texts = render_document(&document)
        .into_iter()
        .filter_map(|primitive| match primitive {
            RenderPrimitive::Text { text, runs, .. } => Some(if text.is_empty() {
                runs.into_iter().map(|run| run.text).collect::<String>()
            } else {
                text
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(texts.iter().filter(|text| text.as_str() == "D").count(), 1);
    assert_eq!(texts.iter().filter(|text| text.as_str() == "T").count(), 1);
    assert_eq!(texts.iter().filter(|text| text.as_str() == "H").count(), 1);
    assert_eq!(
        texts.iter().filter(|text| text.as_str() == "2").count(),
        1,
        "the isotope mass remains visible for an H label but not for D: {texts:?}"
    );
    assert!(
        texts.iter().all(|text| text != "3"),
        "T already encodes isotope mass 3: {texts:?}"
    );
}

#[test]
fn atom_query_uses_symbol_star_size_and_connection_opposite_placement() {
    for (neighbor, expected_side) in [
        ([110.0, 80.0], "left"),
        ([50.0, 80.0], "right"),
        ([80.0, 50.0], "below"),
        ([80.0, 110.0], "above"),
    ] {
        let document = fragment_document(
            json!([
                {
                    "id": "query",
                    "element": "N",
                    "atomicNumber": 7,
                    "position": [80.0, 80.0],
                    "charge": 0,
                    "numHydrogens": 0,
                    "atomProperties": { "freeSites": 2 }
                },
                {
                    "id": "neighbor",
                    "element": "C",
                    "atomicNumber": 6,
                    "position": neighbor,
                    "charge": 0,
                    "numHydrogens": 0
                }
            ]),
            json!([{
                "id": "bond",
                "begin": "query",
                "end": "neighbor",
                "order": 1
            }]),
        );
        let rendered = render_document(&document);
        let query = rendered
            .iter()
            .find_map(|primitive| match primitive {
                RenderPrimitive::Text {
                    node_id,
                    x,
                    y,
                    runs,
                    ..
                } if node_id.as_deref() == Some("query")
                    && runs.iter().map(|run| run.text.as_str()).collect::<String>() == "*2" =>
                {
                    Some((*x, *y, runs))
                }
                _ => None,
            })
            .expect("query decoration");
        match expected_side {
            "left" => assert!(query.0 < 80.0),
            "right" => assert!(query.0 > 80.0),
            "below" => assert!(query.1 > 80.0),
            "above" => assert!(query.1 < 80.0),
            _ => unreachable!(),
        }
        assert_eq!(query.2[0].font_family.as_deref(), Some("Symbol"));
        assert!((query.2[0].font_size.unwrap() - 8.3).abs() < 1.0e-6);
        assert!((query.2[1].font_size.unwrap() - 7.5).abs() < 1.0e-6);
    }
}
