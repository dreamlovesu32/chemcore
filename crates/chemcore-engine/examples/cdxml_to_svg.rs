use chemcore_engine::Engine;

fn main() {
    let mut args = std::env::args().skip(1);
    let input = args
        .next()
        .unwrap_or_else(|| "tmp/arrows.cdxml".to_string());
    let output = args
        .next()
        .unwrap_or_else(|| "tmp/arrows-backend.svg".to_string());

    let cdxml = std::fs::read_to_string(&input).expect("cdxml should be readable");
    let mut engine = Engine::new();
    engine
        .load_cdxml_document(&cdxml)
        .expect("cdxml should load into engine");
    let svg = engine.document_svg();
    std::fs::write(&output, svg).expect("svg should be writable");
    println!("wrote {output}");
}
