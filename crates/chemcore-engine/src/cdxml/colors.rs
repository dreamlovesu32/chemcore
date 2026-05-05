use std::collections::BTreeMap;

use super::{descendants, parse_f64, XmlNode};

const DEFAULT_FOREGROUND: &str = "#000000";
const DEFAULT_BACKGROUND: &str = "#ffffff";
const COLOR_TABLE_ID_OFFSET: usize = 2;

const DEFAULT_TABLE_COLORS: [&str; 12] = [
    "#ffffff", "#000000", "#ff0000", "#ffff00", "#00ff00", "#00ffff", "#0000ff", "#ff00ff",
    "#804040", "#008000", "#0000a0", "#808080",
];

#[derive(Debug, Clone)]
pub(super) struct CdxmlColorTable {
    colors: Vec<String>,
    ids: BTreeMap<String, String>,
    foreground: String,
    background: String,
}

impl Default for CdxmlColorTable {
    fn default() -> Self {
        let mut table = Self {
            colors: Vec::new(),
            ids: BTreeMap::new(),
            foreground: DEFAULT_FOREGROUND.to_string(),
            background: DEFAULT_BACKGROUND.to_string(),
        };
        for color in DEFAULT_TABLE_COLORS {
            table.insert_table_color(color);
        }
        table
    }
}

impl CdxmlColorTable {
    pub(super) fn for_export(background: &str) -> Self {
        let mut table = Self::default();
        table.background =
            normalize_hex_color(background).unwrap_or_else(|| DEFAULT_BACKGROUND.to_string());
        if table.background != DEFAULT_BACKGROUND {
            let background = table.background.clone();
            table.insert_table_color(&background);
        }
        table
    }

    pub(super) fn from_cdxml(root: &XmlNode) -> Self {
        let mut table = Self {
            colors: Vec::new(),
            ids: BTreeMap::new(),
            foreground: DEFAULT_FOREGROUND.to_string(),
            background: DEFAULT_BACKGROUND.to_string(),
        };
        if let Some(color_table) = descendants(root)
            .into_iter()
            .find(|node| node.is("colortable"))
        {
            for color in color_table.direct_children("color") {
                let parsed = match (
                    parse_f64(color.attr("r")),
                    parse_f64(color.attr("g")),
                    parse_f64(color.attr("b")),
                ) {
                    (Some(r), Some(g), Some(b)) => rgb_fraction_to_hex(r, g, b),
                    _ => DEFAULT_FOREGROUND.to_string(),
                };
                table.append_table_color(&parsed);
            }
        }
        if table.colors.is_empty() {
            for color in DEFAULT_TABLE_COLORS {
                table.insert_table_color(color);
            }
        }
        let foreground = root.attr("color").unwrap_or("0");
        if foreground != "0" {
            table.foreground = table.resolve_id(foreground);
        }
        let background = root.attr("bgcolor").unwrap_or("1");
        if background != "1" {
            table.background = table.resolve_id(background);
        }
        table
    }

    pub(super) fn resolve(&self, color_id: Option<&str>) -> String {
        self.resolve_id(color_id.unwrap_or("0"))
    }

    pub(super) fn ensure(&mut self, color: &str) -> String {
        let normalized =
            normalize_hex_color(color).unwrap_or_else(|| DEFAULT_FOREGROUND.to_string());
        if normalized == self.foreground {
            return "0".to_string();
        }
        if normalized == self.background {
            return "1".to_string();
        }
        if let Some(id) = self.ids.get(&normalized) {
            return id.clone();
        }
        self.insert_table_color(&normalized)
    }

    pub(super) fn id_for(&self, color: &str) -> String {
        let normalized =
            normalize_hex_color(color).unwrap_or_else(|| DEFAULT_FOREGROUND.to_string());
        if normalized == self.foreground {
            return "0".to_string();
        }
        if normalized == self.background {
            return "1".to_string();
        }
        self.ids
            .get(&normalized)
            .cloned()
            .unwrap_or_else(|| "0".to_string())
    }

    pub(super) fn colors(&self) -> &[String] {
        &self.colors
    }

    pub(super) fn background(&self) -> &str {
        &self.background
    }

    pub(super) fn background_id(&self) -> String {
        if self.background == DEFAULT_BACKGROUND {
            "1".to_string()
        } else {
            self.ids
                .get(&self.background)
                .cloned()
                .unwrap_or_else(|| "1".to_string())
        }
    }

    fn resolve_id(&self, color_id: &str) -> String {
        let trimmed = color_id.trim();
        if trimmed == "0" || trimmed.is_empty() {
            return self.foreground.clone();
        }
        if trimmed == "1" {
            return self.background.clone();
        }
        let Some(id) = trimmed.parse::<usize>().ok() else {
            return self.foreground.clone();
        };
        let Some(index) = id.checked_sub(COLOR_TABLE_ID_OFFSET) else {
            return self.foreground.clone();
        };
        self.colors
            .get(index)
            .cloned()
            .unwrap_or_else(|| self.foreground.clone())
    }

    fn insert_table_color(&mut self, color: &str) -> String {
        let normalized =
            normalize_hex_color(color).unwrap_or_else(|| DEFAULT_FOREGROUND.to_string());
        if let Some(id) = self.ids.get(&normalized) {
            return id.clone();
        }
        let id = (self.colors.len() + COLOR_TABLE_ID_OFFSET).to_string();
        self.colors.push(normalized.clone());
        self.ids.insert(normalized, id.clone());
        id
    }

    fn append_table_color(&mut self, color: &str) -> String {
        let normalized =
            normalize_hex_color(color).unwrap_or_else(|| DEFAULT_FOREGROUND.to_string());
        let id = (self.colors.len() + COLOR_TABLE_ID_OFFSET).to_string();
        self.colors.push(normalized.clone());
        self.ids.entry(normalized).or_insert_with(|| id.clone());
        id
    }
}

pub(super) fn normalize_hex_color(color: &str) -> Option<String> {
    let color = color.trim();
    if !color.starts_with('#') {
        return None;
    }
    let hex = &color[1..];
    if hex.len() == 3 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        let mut out = String::from("#");
        for ch in hex.chars() {
            out.push(ch.to_ascii_lowercase());
            out.push(ch.to_ascii_lowercase());
        }
        return Some(out);
    }
    if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(format!("#{}", hex.to_ascii_lowercase()));
    }
    None
}

pub(super) fn rgb_fractions(color: &str) -> (f64, f64, f64) {
    let normalized = normalize_hex_color(color).unwrap_or_else(|| DEFAULT_FOREGROUND.to_string());
    let r = u8::from_str_radix(&normalized[1..3], 16).unwrap_or(0) as f64 / 255.0;
    let g = u8::from_str_radix(&normalized[3..5], 16).unwrap_or(0) as f64 / 255.0;
    let b = u8::from_str_radix(&normalized[5..7], 16).unwrap_or(0) as f64 / 255.0;
    (r, g, b)
}

fn rgb_fraction_to_hex(r: f64, g: f64, b: f64) -> String {
    format!(
        "#{:02x}{:02x}{:02x}",
        (r * 255.0).round().clamp(0.0, 255.0) as u8,
        (g * 255.0).round().clamp(0.0, 255.0) as u8,
        (b * 255.0).round().clamp(0.0, 255.0) as u8
    )
}
