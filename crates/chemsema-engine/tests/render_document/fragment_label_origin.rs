use super::*;

fn imported_attached_label_source() -> &'static str {
    r##"<?xml version="1.0" encoding="UTF-8"?>
<CDXML LabelFont="4" LabelSize="7" MarginWidth="1.25">
  <fonttable><font id="4" charset="0" name="Times New Roman"/></fonttable>
  <page>
    <fragment>
      <n id="right" p="100 100" NodeType="Unspecified" LabelDisplay="Right">
        <t p="102 102.73" BoundingBox="82 96 102 103"
           LabelAlignment="Right" LabelJustification="Right">
          <s font="4" size="7" face="96">(Aax) </s><s font="4" size="7" face="34">n</s>
        </t>
      </n>
      <n id="neighbor" p="116 100"/>
      <b id="bond" B="right" E="neighbor"/>
    </fragment>
  </page>
</CDXML>"##
}

fn rendered_label_origin(document: &ChemSemaDocument, node_id: &str) -> (f64, Option<String>) {
    render_document(document)
        .into_iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::Text {
                node_id: rendered_node_id,
                x,
                text_anchor,
                ..
            } if rendered_node_id.as_deref() == Some(node_id) => Some((x, text_anchor)),
            _ => None,
        })
        .expect("attached label should render")
}

fn imported_authored_left_world(document: &ChemSemaDocument, node_id: &str) -> f64 {
    let entry = document
        .editable_fragments()
        .into_iter()
        .next()
        .expect("fragment");
    let label = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == node_id)
        .and_then(|node| node.label.as_ref())
        .expect("attached label");
    let local_bounding_box = label
        .meta
        .pointer("/import/cdxml/localBoundingBox")
        .and_then(serde_json::Value::as_array)
        .expect("authored local bounding box");
    entry.object.transform.translate[0]
        + local_bounding_box[0].as_f64().expect("authored local left")
}

#[test]
fn imported_single_line_attached_label_uses_authored_bounding_box_left() {
    let document = parse_cdxml_document(
        imported_attached_label_source(),
        Some("authored attached label origin"),
    )
    .expect("CDXML should parse");
    let entry = document
        .editable_fragments()
        .into_iter()
        .next()
        .expect("fragment");
    let label = entry.fragment.nodes[0]
        .label
        .as_ref()
        .expect("attached label");
    let natural_left = entry.object.transform.translate[0] + label.bbox().expect("resolved box")[0];
    let expected = imported_authored_left_world(&document, "right");
    assert!(
        (natural_left - expected).abs() > 0.5,
        "fixture must distinguish authored text origin from natural glyph outset"
    );

    let (render_x, render_anchor) = rendered_label_origin(&document, "right");
    assert_close(render_x, expected);
    assert_eq!(render_anchor.as_deref(), Some("start"));
}

#[test]
fn edited_single_line_attached_label_uses_current_resolved_geometry() {
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(imported_attached_label_source())
        .expect("CDXML should load");
    let result: serde_json::Value = serde_json::from_str(
        &engine
            .execute_command_json(
                &json!({
                    "type": "set-node-label-runs",
                    "nodeId": "right",
                    "runs": [{
                        "text": "(Bbx) n",
                        "fontFamily": "Times New Roman",
                        "fontSize": 7.0,
                        "script": "normal"
                    }]
                })
                .to_string(),
            )
            .expect("label edit should execute"),
    )
    .expect("command result should be JSON");
    assert_eq!(result["changed"], true);

    let entry = engine
        .state()
        .document
        .editable_fragment()
        .expect("edited fragment");
    let label = entry
        .fragment
        .nodes
        .iter()
        .find(|node| node.id == "right")
        .and_then(|node| node.label.as_ref())
        .expect("edited label");
    assert!(
        label.meta.pointer("/import/cdxml/boundingBox").is_none(),
        "committed text edits must explicitly discard authored geometry"
    );
    let expected = entry.object.transform.translate[0] + label.bbox().expect("resolved box")[0];
    let (render_x, render_anchor) = rendered_label_origin(&engine.state().document, "right");
    assert_close(render_x, expected);
    assert_eq!(render_anchor.as_deref(), Some("start"));
}

#[test]
fn right_display_with_left_edge_text_position_uses_resolved_geometry() {
    let source = imported_attached_label_source().replace(
        r#"<t p="102 102.73" BoundingBox="82 96 102 103""#,
        r#"<t p="82 102.73" BoundingBox="82 96 102 103""#,
    );
    let document = parse_cdxml_document(
        &source,
        Some("right display with left-origin text position"),
    )
    .expect("CDXML should parse");
    let entry = document
        .editable_fragments()
        .into_iter()
        .next()
        .expect("fragment");
    let label = entry.fragment.nodes[0]
        .label
        .as_ref()
        .expect("attached label");
    let expected = entry.object.transform.translate[0] + label.bbox().expect("resolved box")[0];

    let (render_x, render_anchor) = rendered_label_origin(&document, "right");
    assert_close(render_x, expected);
    assert_eq!(render_anchor.as_deref(), Some("start"));
}
