use chemcore_engine::{Engine, Point};
use serde_json::json;

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let rect = args
        .windows(5)
        .position(|window| window[0] == "--rect")
        .map(|index| {
            let values = args.drain(index..index + 5).skip(1).collect::<Vec<_>>();
            [
                values[0].parse::<f64>().expect("rect x1 should be numeric"),
                values[1].parse::<f64>().expect("rect y1 should be numeric"),
                values[2].parse::<f64>().expect("rect x2 should be numeric"),
                values[3].parse::<f64>().expect("rect y2 should be numeric"),
            ]
        });
    let mut args = args.into_iter();
    let input = args.next().expect("input cdxml path is required");
    let output = args.next().expect("output payload path is required");
    let select_all = args.any(|arg| arg == "--select-all");

    let cdxml = std::fs::read_to_string(&input).expect("cdxml should be readable");
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(&cdxml)
        .expect("cdxml should load into engine");
    let document_json = if let Some([x1, y1, x2, y2]) = rect {
        engine.select_in_rect(
            Point {
                x: chemcore_engine::px_to_pt(x1),
                y: chemcore_engine::px_to_pt(y1),
            },
            Point {
                x: chemcore_engine::px_to_pt(x2),
                y: chemcore_engine::px_to_pt(y2),
            },
            false,
        );
        engine
            .clipboard_document_json()
            .expect("clipboard document should serialize")
            .expect("rect should produce a clipboard document")
    } else if select_all {
        engine.select_all();
        engine
            .clipboard_document_json()
            .expect("clipboard document should serialize")
            .expect("select all should produce a clipboard document")
    } else {
        engine.document_json().expect("document should serialize")
    };
    let fragment_json = if select_all || rect.is_some() {
        engine
            .clipboard_selection_json()
            .expect("clipboard selection should serialize")
    } else {
        None
    };
    let render_list_json =
        serde_json::to_string(&engine.render_list()).expect("render list should serialize");
    let payload = json!({
        "chemcoreFragmentJson": fragment_json,
        "chemcoreDocumentJson": document_json,
        "renderListJson": render_list_json,
        "cdxml": cdxml,
        "svg": null,
        "text": cdxml
    });
    std::fs::write(&output, serde_json::to_string_pretty(&payload).unwrap())
        .expect("payload should be writable");
}
