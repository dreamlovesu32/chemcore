pub(super) fn parse_numbers(value: &str) -> Option<Vec<f64>> {
    let values: Option<Vec<f64>> = value
        .split_whitespace()
        .map(|part| part.parse::<f64>().ok())
        .collect();
    values
}

pub(super) fn write_property(out: &mut Vec<u8>, tag: u16, data: &[u8]) {
    write_u16(out, tag);
    if data.len() > 65_534 {
        write_u16(out, 0xFFFF);
        write_u32(out, data.len() as u32);
    } else {
        write_u16(out, data.len() as u16);
    }
    out.extend_from_slice(data);
}

pub(super) fn write_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(super) fn write_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}

pub(super) fn yes(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "yes" | "true" | "1"
    )
}

pub(super) fn fmt_num(value: f64) -> String {
    let rounded = (value * 1000.0).round() / 1000.0;
    let mut out = format!("{rounded:.3}");
    while out.contains('.') && out.ends_with('0') {
        out.pop();
    }
    if out.ends_with('.') {
        out.pop();
    }
    if out == "-0" {
        "0".to_string()
    } else {
        out
    }
}

pub(super) fn escape_attr(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub(super) fn escape_text(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub(super) fn attr_order(key: &str) -> usize {
    match key {
        "id" => 0,
        "BoundingBox" => 1,
        "p" => 2,
        "Z" => 3,
        _ => 10,
    }
}

pub(super) fn charset_name(id: u16) -> &'static str {
    match id {
        0 => "iso-8859-1",
        1 => "iso-8859-1",
        65001 => "UTF-8",
        1252 => "iso-8859-1",
        _ => "iso-8859-1",
    }
}

pub(super) fn charset_id(name: &str) -> u16 {
    match name.to_ascii_lowercase().as_str() {
        "utf-8" | "utf8" => 65001,
        "iso-8859-1" | "latin1" | "windows-1252" | "cp1252" => 1252,
        _ => 1252,
    }
}
