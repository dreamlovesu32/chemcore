use chemcore_engine::Engine;
use serde_json::json;

fn main() {
    let mut args = std::env::args().skip(1);
    let input = args.next().expect("input cdxml path is required");
    let output = args.next().expect("output payload path is required");

    let cdxml = std::fs::read_to_string(&input).expect("cdxml should be readable");
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(&cdxml)
        .expect("cdxml should load into engine");
    let document_json = engine.document_json().expect("document should serialize");
    let svg = engine.document_svg();
    let payload = json!({
        "chemcoreDocumentJson": document_json,
        "cdxml": cdxml,
        "svg": svg,
        "text": "Chemcore Document"
    });
    std::fs::write(&output, serde_json::to_string_pretty(&payload).unwrap())
        .expect("payload should be writable");
}
