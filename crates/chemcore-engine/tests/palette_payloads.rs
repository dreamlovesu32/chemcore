use chemcore_engine::Engine;

#[test]
fn engine_exposes_ui_palette_payloads() {
    let engine = Engine::new();

    let toolbar_colors: serde_json::Value =
        serde_json::from_str(&engine.toolbar_color_palette_json(r##"["#336699"]"##)).unwrap();
    assert_eq!(toolbar_colors["type"], "toolbar-color-palette");
    assert!(toolbar_colors["colors"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["value"] == "#336699"));

    let color_dialog: serde_json::Value =
        serde_json::from_str(&engine.color_dialog_palette_json("#ff0000", r##"["#336699"]"##))
            .unwrap();
    assert_eq!(color_dialog["type"], "color-dialog");
    assert_eq!(color_dialog["selected"], "#ff0000");
    assert!(color_dialog["basicColors"].as_array().unwrap().len() >= 48);

    let text_symbols: serde_json::Value =
        serde_json::from_str(&engine.text_symbol_palette_json()).unwrap();
    assert_eq!(text_symbols["type"], "text-symbol-palette");
    assert!(text_symbols["groups"].as_array().unwrap().len() >= 3);

    let elements: serde_json::Value = serde_json::from_str(&engine.element_palette_json()).unwrap();
    assert_eq!(elements["type"], "periodic-table");
    assert_eq!(elements["current"]["symbol"], "P");
    assert!(elements["elements"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["symbol"] == "C" && entry["color"]["background"] == "#000000"));
}

#[test]
fn engine_applies_element_palette_selection() {
    let mut engine = Engine::new();

    assert!(engine
        .apply_element_palette_json(r#"{"symbol":"O"}"#)
        .unwrap());

    let elements: serde_json::Value = serde_json::from_str(&engine.element_palette_json()).unwrap();
    assert_eq!(elements["current"]["symbol"], "O");
    assert_eq!(elements["current"]["atomicNumber"], 8);
}
