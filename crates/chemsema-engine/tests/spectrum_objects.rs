use chemsema_engine::{
    document_to_cdx, document_to_cdxml, parse_cdx_document, parse_cdxml_document,
    parse_document_json, Engine, NmrAssignmentQuality, Point, RenderBoundsScope, RenderPrimitive,
    SpectrumClass, SpectrumXAxisType, SpectrumYAxisType,
};
use serde_json::{json, Value};

const SPECTRUM_CDXML: &str = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="30" LabelFont="3" LabelSize="10" LineWidth="0.6">
  <fonttable>
    <font id="3" charset="iso-8859-1" name="Arial"/>
  </fonttable>
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
    <color r="0.2" g="0.4" b="0.8"/>
  </colortable>
  <page id="1" BoundingBox="0 0 400 240">
    <spectrum id="2" BoundingBox="40 30 280 150" Z="3"
      Class="NMR" XType="PartsPerMillion" YType="ArbitraryUnits"
      XLow="-2" XSpacing="1" XAxisLabel="ppm" YAxisLabel="intensity"
      YLow="10" YScale="2" LabelFont="3" LabelSize="11" LabelFace="31"
      LineWidth="0.9" color="4">0 0.1 0.4 -0.2 1 0.3 0</spectrum>
  </page>
</CDXML>"#;

const ASSIGNED_MOLECULE_CDXML: &str = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="30" LabelFont="3" LabelSize="10" LineWidth="0.6">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
    <color r="0" g="0" b="1"/>
  </colortable>
  <page id="1" BoundingBox="0 0 300 200">
    <fragment id="2">
      <n id="3" p="60 70" Element="6" NumHydrogens="4">
        <objecttag id="4" TagType="String" Name="/CS/CD/assign" Value="0.878-0.943,">
          <t id="5" p="60 55" BoundingBox="52 46 68 55" LabelFont="3" LabelSize="7.5" color="4">
            <s font="3" size="7.5" color="4">0.91</s>
          </t>
        </objecttag>
      </n>
    </fragment>
  </page>
</CDXML>"#;

const HETEROATOM_MOLECULE_CDXML: &str = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="30" LabelFont="3" LabelSize="10" LineWidth="0.6">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable>
    <color r="1" g="1" b="1"/>
    <color r="0" g="0" b="0"/>
  </colortable>
  <page id="1" BoundingBox="0 0 300 200">
    <fragment id="2">
      <n id="3" p="60 70"/>
      <n id="4" p="90 70"/>
      <n id="5" p="120 70" Element="8">
        <t id="6" p="120 70"><s font="3" size="10">OH</s></t>
      </n>
      <b id="7" B="3" E="4" Order="1"/>
      <b id="8" B="4" E="5" Order="1"/>
    </fragment>
  </page>
</CDXML>"#;

const CHIRAL_ALCOHOL_CDXML: &str = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="30" LabelFont="3" LabelSize="10" LineWidth="0.6">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1" BoundingBox="0 0 300 200">
    <fragment id="2">
      <n id="8" p="40 120"/>
      <n id="9" p="70 100"/>
      <n id="10" p="100 120"/>
      <n id="11" p="130 100"/>
      <n id="12" p="160 120"/>
      <n id="13" p="130 60" Element="8"/>
      <b id="21" B="8" E="9" Order="1"/>
      <b id="22" B="9" E="10" Order="1"/>
      <b id="23" B="10" E="11" Order="1"/>
      <b id="24" B="11" E="12" Order="1" Display="WedgedHashBegin"/>
      <b id="25" B="11" E="13" Order="1"/>
    </fragment>
  </page>
</CDXML>"#;

const MULTICENTER_CDXML: &str = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="30" LabelFont="3" LabelSize="10" LineWidth="0.6">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <colortable><color r="1" g="1" b="1"/><color r="0" g="0" b="0"/></colortable>
  <page id="1" BoundingBox="0 0 300 200">
    <fragment id="2">
      <n id="1" p="40 100"/>
      <n id="2" p="70 100"/>
      <n id="3" p="100 100"/>
      <n id="4" p="70 70" NodeType="MultiAttachment" Attachments="1 2 3"/>
      <n id="5" p="70 40" Element="26" NumHydrogens="0"/>
      <b id="11" B="1" E="2" Order="1.5" Display="Dash"/>
      <b id="12" B="2" E="3" Order="1.5" Display="Dash"/>
      <b id="14" B="3" E="1" Order="1.5" Display="Dash"/>
      <b id="13" B="4" E="5" Order="1"/>
    </fragment>
  </page>
</CDXML>"#;

const COORDINATION_STEREO_CDXML: &str = r#"<?xml version="1.0" encoding="UTF-8" ?>
<CDXML BondLength="30" LabelFont="3" LabelSize="10" LineWidth="0.6">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1"><fragment id="2">
    <n id="1" p="20 80"/>
    <n id="2" p="80 80"/>
    <n id="27" p="35 60" NodeType="MultiAttachment" Attachments="1"/>
    <n id="28" p="65 60" NodeType="MultiAttachment" Attachments="2"/>
    <n id="30" p="50 40" Element="22" NumHydrogens="0"
      Geometry="Tetrahedral" BondOrdering="34 31 36 32"/>
    <n id="33" p="20 20" Element="17"/>
    <n id="35" p="80 20" Element="9"/>
    <b id="31" B="28" E="30"/>
    <b id="32" B="27" E="30"/>
    <b id="34" B="30" E="33" Display="WedgeBegin"/>
    <b id="36" B="30" E="35" Display="WedgedHashBegin"/>
  </fragment></page>
</CDXML>"#;

fn spectrum_object(document: &chemsema_engine::ChemSemaDocument) -> &chemsema_engine::SceneObject {
    document
        .scene_objects()
        .into_iter()
        .find(|object| object.object_type == "spectrum")
        .expect("native spectrum object")
}

fn execute(engine: &mut Engine, command: Value) -> Value {
    serde_json::from_str(
        &engine
            .execute_command_json(&command.to_string())
            .expect("command executes"),
    )
    .expect("command result JSON")
}

#[test]
fn cdxml_import_creates_a_native_spectrum_and_render_primitives() {
    let document = parse_cdxml_document(SPECTRUM_CDXML, Some("spectrum")).expect("CDXML parses");
    let object = spectrum_object(&document);
    let spectrum = object.payload.spectrum.as_ref().expect("spectrum payload");

    assert_eq!(object.payload.bbox, Some([0.0, 0.0, 240.0, 120.0]));
    assert_eq!(object.transform.translate, [40.0, 30.0]);
    assert_eq!(spectrum.class, SpectrumClass::Nmr);
    assert_eq!(spectrum.x_type, SpectrumXAxisType::PartsPerMillion);
    assert_eq!(spectrum.y_type, SpectrumYAxisType::ArbitraryUnits);
    assert_eq!(spectrum.x_low, -2.0);
    assert_eq!(spectrum.x_high(), 5.0);
    assert_eq!(
        spectrum.decoded_points().collect::<Vec<_>>(),
        vec![10.0, 10.2, 10.8, 9.6, 12.0, 10.6, 10.0]
    );
    let style = document
        .styles
        .get(object.style_ref.as_deref().expect("spectrum style ref"))
        .expect("spectrum style");
    assert_eq!(style["fontWeight"], 700);
    assert_eq!(style["fontStyle"], "italic");
    assert_eq!(style["underline"], true);
    assert_eq!(style["outline"], true);
    assert_eq!(style["shadow"], true);

    let object_id = object.id.clone();
    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).expect("document serializes"))
        .expect("native document loads");
    let render = engine.render_list();
    assert!(render.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Polyline { object_id: Some(id), .. }
            if id.as_str() == object_id.as_str()
    )));
    assert!(render.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Text { text, object_id: Some(id), .. }
            if id.as_str() == object_id.as_str() && text == "ppm"
    )));
    assert!(render.iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Text { text, object_id: Some(id), .. }
            if id.as_str() == object_id.as_str() && text == "intensity"
    )));
}

#[test]
fn cdxml_and_cdx_roundtrips_preserve_their_defined_spectrum_storage_rules() {
    let document = parse_cdxml_document(SPECTRUM_CDXML, None).expect("CDXML parses");

    let cdxml = document_to_cdxml(&document);
    assert!(cdxml.contains("LabelFace=\"31\""));
    let from_cdxml = parse_cdxml_document(&cdxml, None).expect("generated CDXML parses");
    let cdxml_spectrum = spectrum_object(&from_cdxml)
        .payload
        .spectrum
        .as_ref()
        .expect("spectrum payload");
    assert_eq!(cdxml_spectrum.y_low, 10.0);
    assert_eq!(cdxml_spectrum.y_scale, 2.0);
    assert_eq!(
        cdxml_spectrum.data_points,
        vec![0.0, 0.1, 0.4, -0.2, 1.0, 0.3, 0.0]
    );

    let cdx = document_to_cdx(&document).expect("CDX writes");
    let from_cdx = parse_cdx_document(&cdx, None).expect("generated CDX parses");
    let cdx_spectrum = spectrum_object(&from_cdx)
        .payload
        .spectrum
        .as_ref()
        .expect("spectrum payload");
    assert_eq!(cdx_spectrum.class, SpectrumClass::Nmr);
    assert_eq!(cdx_spectrum.x_type, SpectrumXAxisType::PartsPerMillion);
    assert_eq!(cdx_spectrum.y_type, SpectrumYAxisType::ArbitraryUnits);
    assert_eq!(cdx_spectrum.y_low, 0.0);
    assert_eq!(cdx_spectrum.y_scale, 1.0);
    assert_eq!(
        cdx_spectrum.data_points,
        vec![10.0, 10.2, 10.8, 9.6, 12.0, 10.6, 10.0]
    );
}

#[test]
fn native_spectrum_supports_data_edit_move_resize_color_line_width_copy_and_delete() {
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(SPECTRUM_CDXML)
        .expect("spectrum CDXML loads");
    assert!(engine.select_all());
    let object_id = engine.state().selection.arrow_objects[0].clone();

    let update = execute(
        &mut engine,
        json!({
            "type": "set-spectrum-data",
            "objectId": object_id,
            "spectrum": {
                "class": "infrared",
                "xLow": 400,
                "xSpacing": 20,
                "xType": "wavenumbers",
                "xAxisLabel": "cm-1",
                "yLow": 0,
                "yScale": 1,
                "yType": "percent-transmittance",
                "yAxisLabel": "%T",
                "dataPoints": [95, 80, 45, 90]
            }
        }),
    );
    assert_eq!(update["changed"], true);
    assert_eq!(update["command"]["type"], "set-spectrum-data");

    let before = engine
        .render_bounds(RenderBoundsScope::Selection)
        .expect("selection bounds");
    let center = Point::new((before[0] + before[2]) * 0.5, (before[1] + before[3]) * 0.5);
    assert!(engine.begin_selection_move_at_point(center, false, false));
    assert!(engine.finish_selection_move(Point::new(center.x + 12.0, center.y + 8.0), true));

    let moved = engine
        .render_bounds(RenderBoundsScope::Selection)
        .expect("moved bounds");
    assert!(engine.begin_selection_resize("se", Point::new(moved[2], moved[3])));
    assert!(engine.finish_selection_resize(Point::new(moved[2] + 30.0, moved[3] + 20.0)));

    assert!(engine.apply_color_to_selection("#c02040"));
    let settings = execute(
        &mut engine,
        json!({
            "type": "apply-object-settings-to-selection",
            "objectIds": [object_id],
            "settings": { "lineWidth": 1.25 }
        }),
    );
    assert_eq!(settings["changed"], true);

    let rotate = execute(
        &mut engine,
        json!({
            "type": "rotate-targets",
            "targets": { "objects": [object_id] },
            "center": { "x": center.x, "y": center.y },
            "degrees": 30
        }),
    );
    assert_eq!(rotate["changed"], false);
    assert_eq!(
        spectrum_object(&engine.state().document).transform.rotate,
        0.0
    );

    assert!(engine.copy_selection());
    assert!(engine.paste_clipboard());
    assert_eq!(
        engine
            .state()
            .document
            .scene_objects()
            .into_iter()
            .filter(|object| object.object_type == "spectrum")
            .count(),
        2
    );
    assert!(engine.delete_selection());
    assert_eq!(
        engine
            .state()
            .document
            .scene_objects()
            .into_iter()
            .filter(|object| object.object_type == "spectrum")
            .count(),
        1
    );
    let exported = document_to_cdxml(&engine.state().document);
    let reparsed = parse_cdxml_document(&exported, None).expect("edited export parses");
    assert_eq!(
        reparsed
            .scene_objects()
            .into_iter()
            .filter(|object| object.object_type == "spectrum")
            .count(),
        1,
        "deleted spectrum must not be resurrected from interchange metadata"
    );
}

#[test]
fn ccjs_validation_rejects_ambiguous_or_invalid_spectrum_state() {
    let document = parse_cdxml_document(SPECTRUM_CDXML, None).expect("CDXML parses");
    let mut value = serde_json::to_value(document).expect("document serializes");
    parse_document_json(&value.to_string()).expect("valid spectrum CCJS parses");

    value["entities"]["scene"][0]["payload"]["spectrum"]["dataPoints"] = json!([]);
    assert!(parse_document_json(&value.to_string())
        .expect_err("empty spectrum must fail")
        .contains("must not be empty"));

    value["entities"]["scene"][0]["payload"]["spectrum"]["dataPoints"] = json!([1, 2, 3]);
    value["entities"]["scene"][0]["transform"]["rotate"] = json!(15);
    assert!(parse_document_json(&value.to_string())
        .expect_err("rotated spectrum must fail")
        .contains("cannot be rotated"));

    value["entities"]["scene"][0]["transform"]["rotate"] = json!(0);
    value["entities"]["scene"][0]["type"] = json!("shape");
    assert!(parse_document_json(&value.to_string())
        .expect_err("spectrum payload on another object type must fail")
        .contains("non-spectrum object"));
}

#[test]
fn chemdraw_atom_assignment_is_one_native_editable_field_without_duplicate_text() {
    let document =
        parse_cdxml_document(ASSIGNED_MOLECULE_CDXML, None).expect("assignment CDXML parses");
    let fragment = document
        .editable_fragment()
        .expect("molecule fragment")
        .fragment;
    let assignment = &fragment.nodes[0].nmr_assignments[0];
    assert_eq!(assignment.shift_ppm, 0.91);
    assert_eq!(assignment.range_low_ppm, 0.878);
    assert_eq!(assignment.range_high_ppm, 0.943);
    assert_eq!(assignment.quality, NmrAssignmentQuality::Good);
    assert_eq!(assignment.label.text, "0.91");
    assert!(!document
        .scene_objects()
        .iter()
        .any(|object| object.object_type == "text"));

    let mut engine = Engine::new();
    engine
        .load_document_json(&serde_json::to_string(&document).expect("document serializes"))
        .expect("native document loads");
    assert!(engine.render_list().iter().any(|primitive| matches!(
        primitive,
        RenderPrimitive::Text { runs, node_id: Some(id), .. }
            if runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>().concat() == "0.91" && id == "3"
    )));

    let cdxml = document_to_cdxml(&document);
    assert!(cdxml.contains("Name=\"/CS/CD/assign\""));
    assert!(cdxml.contains("Value=\"0.878-0.943,\""));
    let reopened = parse_cdxml_document(&cdxml, None).expect("generated CDXML parses");
    let reopened_assignment = &reopened
        .editable_fragment()
        .expect("molecule fragment")
        .fragment
        .nodes[0]
        .nmr_assignments[0];
    assert_eq!(reopened_assignment.shift_ppm, assignment.shift_ppm);
    assert_eq!(reopened_assignment.range_low_ppm, assignment.range_low_ppm);
    assert_eq!(
        reopened_assignment.range_high_ppm,
        assignment.range_high_ppm
    );
    assert_eq!(reopened_assignment.quality, assignment.quality);
    assert_eq!(reopened_assignment.label.text, assignment.label.text);
    assert_eq!(
        reopened_assignment.label.position,
        assignment.label.position
    );
    assert_eq!(reopened_assignment.label.bbox(), assignment.label.bbox());
    let cdx = document_to_cdx(&document).expect("generated CDX writes");
    let reopened_cdx = parse_cdx_document(&cdx, None).expect("generated CDX parses");
    let cdx_assignment = &reopened_cdx
        .editable_fragment()
        .expect("CDX molecule fragment")
        .fragment
        .nodes[0]
        .nmr_assignments[0];
    assert_eq!(cdx_assignment.shift_ppm, 0.91);
    assert_eq!(cdx_assignment.range_low_ppm, 0.91);
    assert_eq!(cdx_assignment.range_high_ppm, 0.91);
    assert_eq!(cdx_assignment.quality, NmrAssignmentQuality::Good);
}

#[test]
fn ccjs_validation_rejects_invalid_nmr_assignments() {
    let document =
        parse_cdxml_document(ASSIGNED_MOLECULE_CDXML, None).expect("assignment CDXML parses");
    let mut value = serde_json::to_value(document).expect("document serializes");
    let resource = value["resources"]
        .as_object_mut()
        .expect("resources")
        .values_mut()
        .find(|resource| resource["data"]["nodes"].is_array())
        .expect("molecule resource");
    resource["data"]["nodes"][0]["nmrAssignments"][0]["rangeLowPpm"] = json!(2.0);
    resource["data"]["nodes"][0]["nmrAssignments"][0]["rangeHighPpm"] = json!(1.0);
    assert!(parse_document_json(&value.to_string())
        .expect_err("reversed range must fail")
        .contains("rangeLowPpm"));
}

#[test]
fn chemdraw_result_title_resolves_assignment_nucleus() {
    let cdxml = ASSIGNED_MOLECULE_CDXML.replace(
        "<fragment id=\"2\">",
        "<t id=\"20\" p=\"25 30\" BoundingBox=\"25 16 165 33\"><s font=\"3\" size=\"12\">ChemNMR 13C Estimation</s></t><fragment id=\"2\">",
    );
    let document = parse_cdxml_document(&cdxml, None).expect("result CDXML parses");
    assert_eq!(
        document
            .editable_fragment()
            .expect("molecule")
            .fragment
            .nodes[0]
            .nmr_assignments[0]
            .nucleus,
        chemsema_engine::NmrNucleus::Carbon13
    );
}

#[test]
fn prediction_response_builds_the_chemdraw_style_page_from_native_objects() {
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(ASSIGNED_MOLECULE_CDXML)
        .expect("molecule loads");
    assert!(engine.select_all());
    let request: Value = serde_json::from_str(
        &engine
            .nmr_prediction_request_json("1H")
            .expect("request builds"),
    )
    .expect("request JSON");
    assert_eq!(request["schema"], "chemsema.nmr-prediction-request.v2");
    assert_eq!(
        request["graph"]["schema"],
        "chemsema-nomenclature/chemical-graph/2"
    );
    assert_eq!(request["nucleus"], "1H");
    assert_eq!(request["graph"]["atoms"][0]["id"], "3");

    let response = json!({
        "schema": "chemsema.nmr-prediction-response.v2",
        "engineVersion": "0.1.0",
        "ruleSetVersion": "test.v1",
        "status": "complete",
        "moleculeId": request["moleculeId"],
        "nucleus": "1H",
        "conditions": {
            "solvent": "CDCl3",
            "frequencyMHz": 400.0,
            "temperatureKelvin": 298.15
        },
        "assignments": [{
            "siteIds": ["h-3"],
            "atomIds": ["3"],
            "shiftPpm": 0.91,
            "integral": 4.0,
            "confidence": "good",
            "confidenceReason": "Exact source-reviewed branch.",
            "equivalenceClass": "eq-1",
            "contributions": [{
                "ruleId": "base-methyl",
                "valuePpm": 0.91,
                "role": "base",
                "sourceId": "test-source"
            }]
        }],
        "couplings": [{
            "siteIds": ["h-3", "p-3"],
            "atomIds": ["3", "3"],
            "nuclei": ["1H", "31P"],
            "valueHz": 10.9,
            "ruleId": "test-phosphorus-coupling"
        }],
        "peaks": [{
            "assignmentIndexes": [0],
            "centerPpm": 0.91,
            "intensity": 4.0,
            "linePositionsPpm": [0.91],
            "lineIntensities": [4.0]
        }],
        "diagnostics": []
    });
    let result_json = engine
        .nmr_result_document_json(&response.to_string())
        .expect("result page builds");
    let result = parse_document_json(&result_json).expect("result page is valid CCJS");
    assert_eq!(result.document.title, "ChemNMR 1H Estimation");
    assert_eq!(result.document.page.width, 523.32);
    assert_eq!(result.document.page.height, 769.92);
    let result_fragment = result.editable_fragment().expect("result molecule");
    assert_eq!(result_fragment.object.transform.translate, [28.8, 58.0]);
    assert_eq!(result_fragment.fragment.nodes[0].nmr_assignments.len(), 1);
    assert_eq!(
        result_fragment.fragment.nodes[0].nmr_assignments[0].nucleus,
        chemsema_engine::NmrNucleus::Hydrogen1
    );
    let spectrum = spectrum_object(&result);
    assert_eq!(spectrum.transform.translate, [14.4, 119.85]);
    assert_eq!(spectrum.payload.bbox, Some([0.0, 0.0, 450.0, 200.0]));
    assert_eq!(
        spectrum.meta["nmrPrediction"]["peakLinks"][0]["atomIds"],
        json!(["3"])
    );
    assert_eq!(
        spectrum.payload.spectrum.as_ref().expect("spectrum").class,
        SpectrumClass::Nmr
    );
    assert!(result.scene_objects().iter().any(|object| {
        object.object_type == "text"
            && object.payload.extra.get("text").and_then(Value::as_str)
                == Some("Estimation quality is indicated by color: good, medium, rough")
    }));
}

#[test]
fn nmr_request_does_not_treat_an_unresolved_atom_label_as_v2_stereochemistry() {
    let cdxml = ASSIGNED_MOLECULE_CDXML.replace("<n id=\"3\"", "<n id=\"3\" AS=\"R\"");
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(&cdxml)
        .expect("stereochemical molecule loads");
    assert!(engine.select_all());
    let request: Value = serde_json::from_str(
        &engine
            .nmr_prediction_request_json("1H")
            .expect("request builds"),
    )
    .expect("request JSON");

    assert!(request["graph"]["stereo"].as_array().unwrap().is_empty());
    assert!(request["assignedCipDescriptors"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn nmr_request_emits_true_chemical_graph_v2_tetrahedral_stereo() {
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(CHIRAL_ALCOHOL_CDXML)
        .expect("chiral alcohol loads");
    assert!(engine.select_all());
    let request: Value = serde_json::from_str(
        &engine
            .nmr_prediction_request_json("1H")
            .expect("request builds"),
    )
    .expect("request JSON");

    assert_eq!(request["schema"], "chemsema.nmr-prediction-request.v2");
    assert_eq!(
        request["graph"]["schema"],
        "chemsema-nomenclature/chemical-graph/2"
    );
    let stereo = request["graph"]["stereo"].as_array().expect("stereo array");
    let tetrahedral = stereo
        .iter()
        .find(|element| element["kind"] == "tetrahedral")
        .expect("tetrahedral element");
    assert_eq!(tetrahedral["center"], "11");
    assert_eq!(
        tetrahedral["references"]
            .as_array()
            .expect("references")
            .len(),
        4
    );
    assert!(matches!(
        tetrahedral["parity"].as_str(),
        Some("clockwise" | "anticlockwise")
    ));
    assert_eq!(
        request["assignedCipDescriptors"].as_array().unwrap().len(),
        1
    );
}

#[test]
fn cdxml_enhanced_stereo_is_native_and_roundtrips() {
    let source = CHIRAL_ALCOHOL_CDXML.replace(
        "<n id=\"11\"",
        "<n id=\"11\" EnhancedStereoType=\"Or\" EnhancedStereoGroupNum=\"2\"",
    );
    let mut engine = Engine::new();
    engine.load_cdxml_document(&source).expect("source loads");
    assert!(engine.select_all());
    let graph: Value =
        serde_json::from_str(&engine.chemical_graph_v2_json().expect("graph exports"))
            .expect("graph JSON");
    let group = graph["stereo"]
        .as_array()
        .expect("stereo")
        .iter()
        .find(|element| element["kind"] == "enhancedGroup")
        .expect("native enhanced group");
    assert_eq!(group["groupKind"], "or");
    assert_eq!(group["members"], json!(["tetrahedral-11"]));

    let exported = engine.document_cdxml();
    assert!(exported.contains("EnhancedStereoType=\"Or\""));
    assert!(exported.contains("EnhancedStereoGroupNum=\"2\""));
}

#[test]
fn cdxml_multiattachment_becomes_native_interaction_without_proxy_atoms() {
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(MULTICENTER_CDXML)
        .expect("multi-center source loads");
    assert!(engine.select_all());
    let graph: Value =
        serde_json::from_str(&engine.chemical_graph_v2_json().expect("graph exports"))
            .expect("graph JSON");
    assert_eq!(graph["atoms"].as_array().unwrap().len(), 4);
    assert!(graph["atoms"]
        .as_array()
        .unwrap()
        .iter()
        .all(|atom| atom["id"] != "4"));
    assert_eq!(graph["bonds"].as_array().unwrap().len(), 3);
    assert!(graph["bonds"]
        .as_array()
        .unwrap()
        .iter()
        .all(|bond| bond["kind"] == "aromatic"));
    assert!(graph["atoms"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|atom| atom["atomicNumber"] == 6)
        .all(|atom| atom["implicitHydrogens"] == 1));
    assert_eq!(graph["interactions"].as_array().unwrap().len(), 1);
    let centers = graph["interactions"][0]["centers"].as_array().unwrap();
    assert_eq!(
        centers
            .iter()
            .find(|center| center["role"] == "donor")
            .unwrap()["atoms"],
        json!(["1", "2", "3"])
    );
    assert_eq!(
        centers
            .iter()
            .find(|center| center["role"] == "acceptor")
            .unwrap()["atoms"],
        json!(["5"])
    );
    assert_eq!(graph["components"].as_array().unwrap().len(), 1);
}

#[test]
fn multicenter_ligands_turn_metal_wedges_into_coordination_stereo() {
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(COORDINATION_STEREO_CDXML)
        .expect("coordination stereo loads");
    assert!(engine.select_all());
    let graph: Value =
        serde_json::from_str(&engine.chemical_graph_v2_json().expect("graph exports"))
            .expect("graph JSON");
    assert_eq!(graph["interactions"].as_array().unwrap().len(), 2);
    let stereo = graph["stereo"].as_array().unwrap();
    assert_eq!(stereo.len(), 1);
    assert_eq!(stereo[0]["kind"], "extended");
    assert_eq!(stereo[0]["class"], "nontetrahedral-center");
    assert!(stereo[0]["descriptor"]
        .as_str()
        .unwrap()
        .starts_with("tetrahedral-"));
    assert_eq!(stereo[0]["carriers"].as_array().unwrap().len(), 5);
    assert!(stereo.iter().all(|item| item["kind"] != "tetrahedral"));
}

#[test]
fn malformed_cdxml_multiattachment_is_rejected_instead_of_dropped() {
    let missing = MULTICENTER_CDXML.replace("Attachments=\"1 2 3\"", "Attachments=\"1 2 missing\"");
    let error = parse_cdxml_document(&missing, None).expect_err("missing atom must fail");
    assert!(error.contains("missing attachment"));

    let ambiguous = MULTICENTER_CDXML.replace(
        "<b id=\"13\" B=\"4\" E=\"5\" Order=\"1\"/>",
        "<b id=\"13\" B=\"4\" E=\"5\" Order=\"1\"/><b id=\"15\" B=\"4\" E=\"5\" Order=\"1\"/>",
    );
    let error = parse_cdxml_document(&ambiguous, None).expect_err("repeated acceptor must fail");
    assert!(error.contains("invalid or repeated acceptor"));
}

#[test]
fn native_molecule_semantics_survive_cross_document_copy_and_paste() {
    let source = CHIRAL_ALCOHOL_CDXML.replace(
        "<n id=\"11\"",
        "<n id=\"11\" EnhancedStereoType=\"And\" EnhancedStereoGroupNum=\"3\"",
    );
    let mut origin = Engine::new();
    origin.load_cdxml_document(&source).expect("source loads");
    assert!(origin.select_all());
    let clipboard = origin
        .clipboard_document_json()
        .expect("clipboard serializes")
        .expect("selected document");

    let mut destination = Engine::new();
    destination
        .load_document_json(&clipboard)
        .expect("cross-document clipboard opens");
    assert!(destination.select_all());
    let graph: Value = serde_json::from_str(
        &destination
            .chemical_graph_v2_json()
            .expect("pasted graph exports"),
    )
    .expect("graph JSON");
    let group = graph["stereo"]
        .as_array()
        .unwrap()
        .iter()
        .find(|element| element["kind"] == "enhancedGroup")
        .expect("enhanced group survives");
    assert_eq!(group["groupKind"], "and");
    assert_eq!(group["members"], json!(["tetrahedral-11"]));
}

#[test]
fn nmr_request_emits_semantic_v2_double_bond_stereo_from_smiles() {
    let mut engine = Engine::new();
    execute(
        &mut engine,
        json!({
            "type": "insert-smiles",
            "smiles": "F/C=C/F",
            "x": 80.0,
            "y": 80.0
        }),
    );
    assert!(engine.select_all());
    let request: Value = serde_json::from_str(
        &engine
            .nmr_prediction_request_json("1H")
            .expect("request builds"),
    )
    .expect("request JSON");
    let double_bond = request["graph"]["stereo"]
        .as_array()
        .expect("stereo array")
        .iter()
        .find(|element| element["kind"] == "doubleBond")
        .expect("double-bond stereo element");

    assert_eq!(double_bond["relation"], "opposite");
    assert!(double_bond["leftReference"].is_string());
    assert!(double_bond["rightReference"].is_string());
}

#[test]
fn nmr_request_allows_labels_belonging_to_the_selected_complete_molecule() {
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(HETEROATOM_MOLECULE_CDXML)
        .expect("heteroatom molecule loads");
    assert!(engine.select_all());

    let state: Value =
        serde_json::from_str(&engine.state_json().expect("state serializes")).expect("state JSON");
    assert_eq!(state["selection"]["labelNodes"], json!(["5"]));
    let request: Value = serde_json::from_str(
        &engine
            .nmr_prediction_request_json("1H")
            .expect("complete labeled molecule request builds"),
    )
    .expect("request JSON");

    assert_eq!(request["graph"]["atoms"].as_array().unwrap().len(), 3);
    assert_eq!(request["graph"]["atoms"][2]["atomicNumber"], 8);
}
