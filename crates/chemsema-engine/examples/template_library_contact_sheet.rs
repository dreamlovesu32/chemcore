use chemsema_engine::{parse_cdxml_template_documents, template_document_icon_svg};
use std::{env, fs};

const CELL_WIDTH: usize = 64;
const CELL_HEIGHT: usize = 60;
const COLUMNS: usize = 5;

fn main() -> Result<(), String> {
    let input = env::args().nth(1).ok_or_else(|| {
        "usage: template_library_contact_sheet <input.cdxml> <output.svg>".to_string()
    })?;
    let output = env::args().nth(2).ok_or_else(|| {
        "usage: template_library_contact_sheet <input.cdxml> <output.svg>".to_string()
    })?;
    let cdxml = fs::read_to_string(&input).map_err(|error| format!("read {input}: {error}"))?;
    let documents = parse_cdxml_template_documents(&cdxml, Some("Template"))
        .map_err(|error| format!("parse {input}: {error}"))?;
    let rows = documents.len().div_ceil(COLUMNS);
    let mut out = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}"><rect width="100%" height="100%" fill="#fff"/>"##,
        CELL_WIDTH * COLUMNS,
        CELL_HEIGHT * rows,
        CELL_WIDTH * COLUMNS,
        CELL_HEIGHT * rows,
    );
    for (index, document) in documents.iter().enumerate() {
        let x = index % COLUMNS * CELL_WIDTH;
        let y = index / COLUMNS * CELL_HEIGHT;
        let icon = template_document_icon_svg(document);
        let positioned = icon.replacen(
            "<svg ",
            &format!(r#"<svg x="{x}" y="{y}" width="{CELL_WIDTH}" height="{CELL_HEIGHT}" "#),
            1,
        );
        out.push_str(&positioned);
        out.push_str(&format!(
            r##"<path d="M{} {}H{}V{}" fill="none" stroke="#b7b7b7" stroke-width=".5"/>"##,
            x,
            y + CELL_HEIGHT,
            x + CELL_WIDTH,
            y,
        ));
    }
    out.push_str("</svg>");
    fs::write(&output, out).map_err(|error| format!("write {output}: {error}"))?;
    println!(
        "[TEMPLATE-SHEET] templates={} output={output}",
        documents.len()
    );
    Ok(())
}
