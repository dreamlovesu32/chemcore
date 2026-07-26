use chemsema_engine::{
    document_to_cdx, document_to_cdxml, parse_cdx_document, parse_cdxml_document, Engine, Point,
    PointerEvent, RenderBoundsScope, TableLineStyle, Tool,
};
use serde_json::Value;

const TABLE_CONTENT_CDXML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<CDXML BoundingBox="0 0 300 200" LineWidth="0.75" LabelFont="3" LabelSize="10">
  <fonttable><font id="3" charset="iso-8859-1" name="Arial"/></fonttable>
  <page id="1" BoundingBox="0 0 300 200">
    <table id="40" BoundingBox="40 40 240 140" Z="1">
      <page id="41" BoundingBox="40 40 140 140" BoundsInParent="40 40 140 140">
        <t id="411" p="65 85" BoundingBox="60 75 100 95"><s font="3" size="10" face="0">Cell A</s></t>
        <border id="412" Side="top" LineWidth="0"/>
        <border id="413" Side="left" LineType="Bold" LineWidth="1.25"/>
      </page>
      <page id="42" BoundingBox="140 40 240 140" BoundsInParent="140 40 240 140">
        <fragment id="420">
          <n id="421" p="165 90" Element="6"/>
          <n id="422" p="195 90" Element="8"/>
          <b id="423" B="421" E="422" Order="1"/>
        </fragment>
        <border id="424" Side="bottom" LineType="Wavy" LineWidth="0.8"/>
      </page>
    </table>
  </page>
</CDXML>"#;

fn execute(engine: &mut Engine, command: Value) -> Value {
    serde_json::from_str(
        &engine
            .execute_command_json(&command.to_string())
            .expect("table command should execute"),
    )
    .expect("command result should be JSON")
}

#[test]
fn add_table_is_native_and_round_trips_cell_borders() {
    let mut engine = Engine::new();
    let result = execute(
        &mut engine,
        serde_json::json!({
            "type": "add-table",
            "begin": {"x": 20.0, "y": 30.0},
            "end": {"x": 220.0, "y": 130.0},
            "rows": 2,
            "columns": 4
        }),
    );
    assert_eq!(result["changed"], true);
    let table = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "table")
        .expect("native table object");
    let data = table.payload.table.as_ref().expect("typed table payload");
    assert_eq!((data.rows, data.columns), (2, 4));
    assert_eq!(data.cells.len(), 8);
    assert_eq!(data.row_guides, vec![0.0, 50.0, 100.0]);
    assert_eq!(data.column_guides, vec![0.0, 50.0, 100.0, 150.0, 200.0]);

    let table_id = table.id.clone();
    execute(
        &mut engine,
        serde_json::json!({
            "type": "edit-table",
            "objectId": table_id,
            "row": 0,
            "column": 1,
            "action": "border-dashed"
        }),
    );
    let cdxml = engine.document_cdxml();
    assert!(cdxml.contains("<table "), "{cdxml}");
    assert!(cdxml.contains("<border "), "{cdxml}");
    assert!(cdxml.contains("LineType=\"Dashed\""), "{cdxml}");

    let reopened = parse_cdxml_document(&cdxml, Some("table")).expect("table CDXML reopens");
    let data = reopened
        .objects
        .iter()
        .find(|object| object.object_type == "table")
        .and_then(|object| object.payload.table.as_ref())
        .expect("reopened typed table");
    assert_eq!((data.rows, data.columns), (2, 4));
    let cell = data
        .cells
        .iter()
        .find(|cell| cell.row == 0 && cell.column == 1)
        .expect("edited cell");
    assert_eq!(
        cell.borders.top.as_ref().map(|border| border.line_style),
        Some(TableLineStyle::Dashed)
    );
}

#[test]
fn table_tool_drag_requests_kernel_insert_dialog() {
    let mut engine = Engine::new();
    let mut tool = engine.state().tool.clone();
    tool.active_tool = Tool::Table;
    engine.set_tool_state(tool);
    engine.pointer_down(PointerEvent {
        x: 10.0,
        y: 20.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 110.0,
        y: 80.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 110.0,
        y: 80.0,
        button: Some(0),
        alt_key: false,
    });
    let dialog: Value =
        serde_json::from_str(&engine.take_pending_dialog_json()).expect("dialog JSON");
    assert_eq!(dialog["kind"], "insert-table");
    assert_eq!(dialog["bounds"]["begin"], serde_json::json!([10.0, 20.0]));
    assert_eq!(dialog["bounds"]["end"], serde_json::json!([110.0, 80.0]));
}

#[test]
fn table_row_and_column_edits_are_structural_and_undoable() {
    let mut engine = Engine::new();
    execute(
        &mut engine,
        serde_json::json!({
            "type": "add-table",
            "begin": {"x": 0.0, "y": 0.0},
            "end": {"x": 120.0, "y": 80.0},
            "rows": 2,
            "columns": 2
        }),
    );
    let id = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "table")
        .expect("table")
        .id
        .clone();
    for action in ["add-row-after", "add-column-before"] {
        execute(
            &mut engine,
            serde_json::json!({
                "type": "edit-table",
                "objectId": id.clone(),
                "row": 0,
                "column": 0,
                "action": action
            }),
        );
    }
    let object = engine
        .state()
        .document
        .find_scene_object(&id)
        .expect("table object");
    let table = object.payload.table.as_ref().expect("table");
    assert_eq!((table.rows, table.columns), (3, 3));
    assert_eq!(table.cells.len(), 9);
    assert_eq!(object.payload.bbox, Some([0.0, 0.0, 180.0, 120.0]));

    execute(&mut engine, serde_json::json!({"type": "undo"}));
    let table = engine
        .state()
        .document
        .find_scene_object(&id)
        .and_then(|object| object.payload.table.as_ref())
        .expect("table after undo");
    assert_eq!((table.rows, table.columns), (3, 2));
    assert!(document_to_cdxml(&engine.state().document).contains("<table "));
}

#[test]
fn table_import_associates_nested_contents_and_preserves_official_border_styles() {
    let document =
        parse_cdxml_document(TABLE_CONTENT_CDXML, Some("table-content")).expect("table imports");
    let table = document
        .objects
        .iter()
        .find(|object| object.object_type == "table")
        .and_then(|object| object.payload.table.as_ref())
        .expect("native table");
    let first = table
        .cells
        .iter()
        .find(|cell| (cell.row, cell.column) == (0, 0))
        .expect("first cell");
    let second = table
        .cells
        .iter()
        .find(|cell| (cell.row, cell.column) == (0, 1))
        .expect("second cell");
    assert_eq!(first.content_object_ids.len(), 1, "{first:?}");
    assert_eq!(second.content_object_ids.len(), 1, "{second:?}");
    assert_eq!(
        first.borders.top.as_ref().map(|border| border.visible),
        Some(false)
    );
    assert_eq!(
        first.borders.left.as_ref().map(|border| border.line_style),
        Some(TableLineStyle::Bold)
    );
    assert_eq!(
        second
            .borders
            .bottom
            .as_ref()
            .map(|border| border.line_style),
        Some(TableLineStyle::Wavy)
    );

    let exported = document_to_cdxml(&document);
    let table_start = exported.find("<table ").expect("table tag");
    let table_end = exported.find("</table>").expect("table close");
    let text_position = exported.find("<t ").expect("nested text");
    let fragment_position = exported.find("<fragment ").expect("nested fragment");
    assert!(
        table_start < text_position && text_position < table_end,
        "{exported}"
    );
    assert!(
        table_start < fragment_position && fragment_position < table_end,
        "{exported}"
    );
    assert!(exported.contains("LineWidth=\"0\""), "{exported}");
    assert!(exported.contains("LineType=\"Bold\""), "{exported}");
    assert!(exported.contains("LineType=\"Wavy\""), "{exported}");
}

#[test]
fn native_table_survives_cdx_round_trip() {
    let document =
        parse_cdxml_document(TABLE_CONTENT_CDXML, Some("table-cdx")).expect("table imports");
    let cdx = document_to_cdx(&document).expect("table CDX writes");
    let reopened = parse_cdx_document(&cdx, Some("table-cdx")).expect("table CDX reopens");
    let table = reopened
        .objects
        .iter()
        .find(|object| object.object_type == "table")
        .and_then(|object| object.payload.table.as_ref())
        .expect("native table after CDX");
    assert_eq!((table.rows, table.columns), (1, 2));
    assert_eq!(table.cells.len(), 2);
    assert_eq!(
        table.cells[0]
            .borders
            .left
            .as_ref()
            .map(|border| border.line_style),
        Some(TableLineStyle::Bold)
    );
}

#[test]
fn table_selection_moves_resizes_and_copies_as_one_native_object() {
    let mut engine = Engine::new();
    execute(
        &mut engine,
        serde_json::json!({
            "type": "add-table",
            "begin": {"x": 20.0, "y": 30.0},
            "end": {"x": 140.0, "y": 110.0},
            "rows": 2,
            "columns": 3
        }),
    );
    assert!(engine.select_all());
    let before = engine
        .render_bounds(RenderBoundsScope::Selection)
        .expect("table selection bounds");
    let center = Point::new((before[0] + before[2]) * 0.5, (before[1] + before[3]) * 0.5);
    assert!(engine.begin_selection_move_at_point(center, false, false));
    assert!(engine.finish_selection_move(Point::new(center.x + 12.0, center.y + 8.0), true));
    let moved = engine
        .render_bounds(RenderBoundsScope::Selection)
        .expect("moved table bounds");
    assert!(engine.begin_selection_resize("se", Point::new(moved[2], moved[3])));
    assert!(engine.finish_selection_resize(Point::new(moved[2] + 30.0, moved[3] + 20.0)));
    let table = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "table")
        .and_then(|object| object.payload.table.as_ref())
        .expect("resized native table");
    assert_eq!(table.column_guides.last().copied(), Some(150.0));
    assert_eq!(table.row_guides.last().copied(), Some(100.0));

    assert!(engine.copy_selection());
    assert!(engine.paste_clipboard());
    let tables = engine
        .state()
        .document
        .objects
        .iter()
        .filter(|object| object.object_type == "table")
        .collect::<Vec<_>>();
    assert_eq!(tables.len(), 2);
    let first_ids = tables[0]
        .payload
        .table
        .as_ref()
        .unwrap()
        .cells
        .iter()
        .map(|cell| cell.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let second_ids = tables[1]
        .payload
        .table
        .as_ref()
        .unwrap()
        .cells
        .iter()
        .map(|cell| cell.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(first_ids.is_disjoint(&second_ids));
}

#[test]
fn moving_and_copying_a_table_carries_its_cell_contents() {
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(TABLE_CONTENT_CDXML)
        .expect("table content loads");
    let table_id = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.object_type == "table")
        .expect("table")
        .id
        .clone();
    let content_ids = engine
        .state()
        .document
        .find_scene_object(&table_id)
        .and_then(|object| object.payload.table.as_ref())
        .unwrap()
        .cells
        .iter()
        .flat_map(|cell| cell.content_object_ids.iter().cloned())
        .collect::<Vec<_>>();
    let before = content_ids
        .iter()
        .map(|id| {
            engine
                .state()
                .document
                .find_scene_object(id)
                .unwrap()
                .transform
                .translate
        })
        .collect::<Vec<_>>();

    assert!(engine.begin_selection_move_at_point(Point::new(130.0, 130.0), false, false));
    assert_eq!(
        engine.state().selection.arrow_objects,
        vec![table_id.clone()]
    );
    assert!(engine.finish_selection_move(Point::new(140.0, 145.0), true));
    for (id, original) in content_ids.iter().zip(before) {
        let moved = engine
            .state()
            .document
            .find_scene_object(id)
            .unwrap()
            .transform
            .translate;
        assert_eq!(moved, [original[0] + 10.0, original[1] + 15.0]);
    }

    assert!(engine.copy_selection());
    assert!(engine.paste_clipboard());
    let pasted = engine
        .state()
        .document
        .objects
        .iter()
        .filter(|object| object.object_type == "table")
        .find(|object| object.id != table_id)
        .expect("pasted table");
    let pasted_content = pasted
        .payload
        .table
        .as_ref()
        .unwrap()
        .cells
        .iter()
        .flat_map(|cell| cell.content_object_ids.iter())
        .collect::<Vec<_>>();
    assert_eq!(pasted_content.len(), content_ids.len());
    assert!(pasted_content.iter().all(|id| !content_ids.contains(id)));
    assert!(pasted_content.iter().all(|id| engine
        .state()
        .document
        .find_scene_object(id)
        .is_some()));
}
