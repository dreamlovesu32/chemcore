use super::*;
use serde_json::{json, Value as JsonValue};

const TEXT_SYMBOLS_JSON: &str = include_str!("../../../../shared/text_symbols.json");

const TOOLBAR_COLORS: &[(&str, &str)] = &[
    ("#000000", "Black"),
    ("#ff0000", "Red"),
    ("#ffff00", "Yellow"),
    ("#00ff00", "Green"),
    ("#ffffff", "White"),
    ("#00ffff", "Cyan"),
    ("#0000ff", "Blue"),
    ("#ff00ff", "Magenta"),
];

const COLOR_DIALOG_BASIC_COLORS: &[&str] = &[
    "#ff7777", "#ffff77", "#77ff77", "#00e878", "#77e6e6", "#006bd6", "#f46bb4", "#ee66ee",
    "#ff0000", "#ffff00", "#66ff00", "#00ff3b", "#1fd6d6", "#0b75a8", "#ff00dd", "#ff0090",
    "#8b3d3d", "#ff7438", "#00e800", "#007a68", "#004b88", "#7a7de0", "#820047", "#f20073",
    "#900000", "#ff7900", "#007000", "#007748", "#0000ff", "#00007d", "#800080", "#7500ff",
    "#4b0000", "#8a4b00", "#004b00", "#004b4b", "#000075", "#00004b", "#3d003d", "#310075",
    "#000000", "#808000", "#808040", "#808080", "#408080", "#c0c0c0", "#3a003a", "#ffffff",
];

const CURATED_CUSTOM_COLORS: &[&str] = &[
    "#111827", "#374151", "#6b7280", "#9ca3af", "#e5e7eb", "#f8fafc", "#334155", "#0f172a",
    "#0f766e", "#0e7490", "#2563eb", "#4f46e5", "#7c3aed", "#be185d", "#dc2626", "#ea580c",
];

const PERIODIC_ELEMENTS: &[(&str, u8, &str, u8, u8)] = &[
    ("H", 1, "Hydrogen", 1, 1),
    ("He", 2, "Helium", 18, 1),
    ("Li", 3, "Lithium", 1, 2),
    ("Be", 4, "Beryllium", 2, 2),
    ("B", 5, "Boron", 13, 2),
    ("C", 6, "Carbon", 14, 2),
    ("N", 7, "Nitrogen", 15, 2),
    ("O", 8, "Oxygen", 16, 2),
    ("F", 9, "Fluorine", 17, 2),
    ("Ne", 10, "Neon", 18, 2),
    ("Na", 11, "Sodium", 1, 3),
    ("Mg", 12, "Magnesium", 2, 3),
    ("Al", 13, "Aluminum", 13, 3),
    ("Si", 14, "Silicon", 14, 3),
    ("P", 15, "Phosphorus", 15, 3),
    ("S", 16, "Sulfur", 16, 3),
    ("Cl", 17, "Chlorine", 17, 3),
    ("Ar", 18, "Argon", 18, 3),
    ("K", 19, "Potassium", 1, 4),
    ("Ca", 20, "Calcium", 2, 4),
    ("Sc", 21, "Scandium", 3, 4),
    ("Ti", 22, "Titanium", 4, 4),
    ("V", 23, "Vanadium", 5, 4),
    ("Cr", 24, "Chromium", 6, 4),
    ("Mn", 25, "Manganese", 7, 4),
    ("Fe", 26, "Iron", 8, 4),
    ("Co", 27, "Cobalt", 9, 4),
    ("Ni", 28, "Nickel", 10, 4),
    ("Cu", 29, "Copper", 11, 4),
    ("Zn", 30, "Zinc", 12, 4),
    ("Ga", 31, "Gallium", 13, 4),
    ("Ge", 32, "Germanium", 14, 4),
    ("As", 33, "Arsenic", 15, 4),
    ("Se", 34, "Selenium", 16, 4),
    ("Br", 35, "Bromine", 17, 4),
    ("Kr", 36, "Krypton", 18, 4),
    ("Rb", 37, "Rubidium", 1, 5),
    ("Sr", 38, "Strontium", 2, 5),
    ("Y", 39, "Yttrium", 3, 5),
    ("Zr", 40, "Zirconium", 4, 5),
    ("Nb", 41, "Niobium", 5, 5),
    ("Mo", 42, "Molybdenum", 6, 5),
    ("Tc", 43, "Technetium", 7, 5),
    ("Ru", 44, "Ruthenium", 8, 5),
    ("Rh", 45, "Rhodium", 9, 5),
    ("Pd", 46, "Palladium", 10, 5),
    ("Ag", 47, "Silver", 11, 5),
    ("Cd", 48, "Cadmium", 12, 5),
    ("In", 49, "Indium", 13, 5),
    ("Sn", 50, "Tin", 14, 5),
    ("Sb", 51, "Antimony", 15, 5),
    ("Te", 52, "Tellurium", 16, 5),
    ("I", 53, "Iodine", 17, 5),
    ("Xe", 54, "Xenon", 18, 5),
    ("Cs", 55, "Cesium", 1, 6),
    ("Ba", 56, "Barium", 2, 6),
    ("La", 57, "Lanthanum", 3, 6),
    ("Hf", 72, "Hafnium", 4, 6),
    ("Ta", 73, "Tantalum", 5, 6),
    ("W", 74, "Tungsten", 6, 6),
    ("Re", 75, "Rhenium", 7, 6),
    ("Os", 76, "Osmium", 8, 6),
    ("Ir", 77, "Iridium", 9, 6),
    ("Pt", 78, "Platinum", 10, 6),
    ("Au", 79, "Gold", 11, 6),
    ("Hg", 80, "Mercury", 12, 6),
    ("Tl", 81, "Thallium", 13, 6),
    ("Pb", 82, "Lead", 14, 6),
    ("Bi", 83, "Bismuth", 15, 6),
    ("Po", 84, "Polonium", 16, 6),
    ("At", 85, "Astatine", 17, 6),
    ("Rn", 86, "Radon", 18, 6),
    ("Fr", 87, "Francium", 1, 7),
    ("Ra", 88, "Radium", 2, 7),
    ("Ac", 89, "Actinium", 3, 7),
    ("Rf", 104, "Rutherfordium", 4, 7),
    ("Db", 105, "Dubnium", 5, 7),
    ("Sg", 106, "Seaborgium", 6, 7),
    ("Bh", 107, "Bohrium", 7, 7),
    ("Hs", 108, "Hassium", 8, 7),
    ("Mt", 109, "Meitnerium", 9, 7),
    ("Ds", 110, "Darmstadtium", 10, 7),
    ("Rg", 111, "Roentgenium", 11, 7),
    ("Cn", 112, "Copernicium", 12, 7),
    ("Nh", 113, "Nihonium", 13, 7),
    ("Fl", 114, "Flerovium", 14, 7),
    ("Mc", 115, "Moscovium", 15, 7),
    ("Lv", 116, "Livermorium", 16, 7),
    ("Ts", 117, "Tennessine", 17, 7),
    ("Og", 118, "Oganesson", 18, 7),
    ("Ce", 58, "Cerium", 4, 8),
    ("Pr", 59, "Praseodymium", 5, 8),
    ("Nd", 60, "Neodymium", 6, 8),
    ("Pm", 61, "Promethium", 7, 8),
    ("Sm", 62, "Samarium", 8, 8),
    ("Eu", 63, "Europium", 9, 8),
    ("Gd", 64, "Gadolinium", 10, 8),
    ("Tb", 65, "Terbium", 11, 8),
    ("Dy", 66, "Dysprosium", 12, 8),
    ("Ho", 67, "Holmium", 13, 8),
    ("Er", 68, "Erbium", 14, 8),
    ("Tm", 69, "Thulium", 15, 8),
    ("Yb", 70, "Ytterbium", 16, 8),
    ("Lu", 71, "Lutetium", 17, 8),
    ("Th", 90, "Thorium", 4, 9),
    ("Pa", 91, "Protactinium", 5, 9),
    ("U", 92, "Uranium", 6, 9),
    ("Np", 93, "Neptunium", 7, 9),
    ("Pu", 94, "Plutonium", 8, 9),
    ("Am", 95, "Americium", 9, 9),
    ("Cm", 96, "Curium", 10, 9),
    ("Bk", 97, "Berkelium", 11, 9),
    ("Cf", 98, "Californium", 12, 9),
    ("Es", 99, "Einsteinium", 13, 9),
    ("Fm", 100, "Fermium", 14, 9),
    ("Md", 101, "Mendelevium", 15, 9),
    ("No", 102, "Nobelium", 16, 9),
    ("Lr", 103, "Lawrencium", 17, 9),
];

const ELEMENT_COLORS: &[(&str, &str, &str)] = &[
    ("C", "#000000", "#ffffff"),
    ("N", "#0000d8", "#ffffff"),
    ("O", "#ff0000", "#ffffff"),
    ("F", "#62ee75", "#000000"),
    ("Na", "#ff00d8", "#ffffff"),
    ("P", "#ff72df", "#000000"),
    ("S", "#ecff24", "#000000"),
    ("Cl", "#00f32e", "#ffffff"),
    ("Fe", "#0b6415", "#ffffff"),
    ("Ni", "#6c6d6d", "#000000"),
    ("Cu", "#d58428", "#ffffff"),
    ("Ag", "#dcefff", "#000000"),
    ("Au", "#fff438", "#000000"),
    ("Br", "#8b4c42", "#ffffff"),
];

impl Engine {
    pub fn toolbar_color_palette_json(&self, custom_colors_json: &str) -> String {
        let colors = toolbar_color_entries(custom_colors_json);
        json!({
            "type": "toolbar-color-palette",
            "colors": colors,
            "otherLabel": "Other...",
        })
        .to_string()
    }

    pub fn color_dialog_palette_json(
        &self,
        current_color: &str,
        custom_colors_json: &str,
    ) -> String {
        json!({
            "type": "color-dialog",
            "title": "Color",
            "selected": normalize_hex_color(current_color).unwrap_or_else(|| "#000000".to_string()),
            "labels": {
                "basic": "Basic colors:",
                "custom": "Custom colors:",
                "preview": "Color | Solid",
                "addCustom": "Add to custom colors",
                "ok": "OK",
                "cancel": "Cancel",
                "close": "Close",
            },
            "fields": [
                { "kind": "hsv", "key": "h", "label": "Hue", "min": 0, "max": 359 },
                { "kind": "rgb", "key": "r", "label": "Red", "min": 0, "max": 255 },
                { "kind": "hsv", "key": "s", "label": "Saturation", "min": 0, "max": 100 },
                { "kind": "rgb", "key": "g", "label": "Green", "min": 0, "max": 255 },
                { "kind": "hsv", "key": "v", "label": "Brightness", "min": 0, "max": 100 },
                { "kind": "rgb", "key": "b", "label": "Blue", "min": 0, "max": 255 },
                { "kind": "hex", "key": "hex", "label": "Hex" },
            ],
            "basicColors": COLOR_DIALOG_BASIC_COLORS,
            "customColors": dialog_custom_colors(custom_colors_json),
        })
        .to_string()
    }

    pub fn text_symbol_palette_json(&self) -> String {
        let mut payload: JsonValue = serde_json::from_str(TEXT_SYMBOLS_JSON).unwrap_or_else(|_| {
            json!({
                "version": 1,
                "groups": [],
            })
        });
        if let Some(object) = payload.as_object_mut() {
            object.insert("type".to_string(), json!("text-symbol-palette"));
            object.insert("title".to_string(), json!("Symbol"));
            object.insert("toggleLabel".to_string(), json!("Text symbols"));
            object.insert("pinLabel".to_string(), json!("Pin"));
        }
        payload.to_string()
    }

    pub fn element_palette_json(&self) -> String {
        let current = periodic_element_by_symbol(&self.state.tool.element_symbol)
            .or_else(|| periodic_element_by_symbol("P"))
            .expect("periodic table should contain phosphorus");
        json!({
            "type": "periodic-table",
            "title": "Periodic Table",
            "current": element_json(current),
            "columns": 18,
            "rows": 9,
            "elements": PERIODIC_ELEMENTS.iter().map(|element| element_json(*element)).collect::<Vec<_>>(),
        })
        .to_string()
    }

    pub fn apply_element_palette_json(&mut self, selection_json: &str) -> Result<bool, String> {
        let payload: JsonValue =
            serde_json::from_str(selection_json).map_err(|error| error.to_string())?;
        let symbol = payload
            .get("symbol")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| "Element selection requires a symbol.".to_string())?;
        let Some((symbol, atomic_number, _, _, _)) = periodic_element_by_symbol(symbol) else {
            return Err(format!("Unknown element symbol: {symbol}"));
        };
        let mut tool = self.state.tool.clone();
        let changed = tool.element_symbol != symbol || tool.element_atomic_number != atomic_number;
        tool.element_symbol = symbol.to_string();
        tool.element_atomic_number = atomic_number;
        self.set_tool_state(tool);
        Ok(changed)
    }
}

fn toolbar_color_entries(custom_colors_json: &str) -> Vec<JsonValue> {
    let mut entries = TOOLBAR_COLORS
        .iter()
        .map(|(value, title)| json!({ "value": value, "title": title }))
        .collect::<Vec<_>>();
    let basics = TOOLBAR_COLORS
        .iter()
        .map(|(value, _)| *value)
        .collect::<Vec<_>>();
    for color in parsed_colors(custom_colors_json) {
        if basics
            .iter()
            .any(|basic| basic.eq_ignore_ascii_case(color.as_str()))
        {
            continue;
        }
        entries.push(json!({ "value": color, "title": color.to_ascii_uppercase() }));
    }
    entries
}

fn dialog_custom_colors(custom_colors_json: &str) -> Vec<String> {
    let mut colors = parsed_colors(custom_colors_json);
    colors.extend(
        CURATED_CUSTOM_COLORS
            .iter()
            .filter_map(|color| normalize_hex_color(color)),
    );
    unique_colors(colors).into_iter().take(16).collect()
}

fn parsed_colors(custom_colors_json: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<JsonValue>(custom_colors_json) else {
        return Vec::new();
    };
    let Some(items) = value.as_array() else {
        return Vec::new();
    };
    unique_colors(
        items
            .iter()
            .filter_map(JsonValue::as_str)
            .filter_map(normalize_hex_color)
            .collect(),
    )
}

fn unique_colors(colors: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for color in colors {
        if !out.iter().any(|existing: &String| existing == &color) {
            out.push(color);
        }
    }
    out
}

fn normalize_hex_color(value: &str) -> Option<String> {
    let raw = value.trim().to_ascii_lowercase();
    if raw.len() == 7
        && raw.starts_with('#')
        && raw[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Some(raw);
    }
    if raw.len() == 4
        && raw.starts_with('#')
        && raw[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        let chars = raw.chars().collect::<Vec<_>>();
        return Some(format!(
            "#{}{}{}{}{}{}",
            chars[1], chars[1], chars[2], chars[2], chars[3], chars[3]
        ));
    }
    None
}

fn periodic_element_by_symbol(symbol: &str) -> Option<(&'static str, u8, &'static str, u8, u8)> {
    PERIODIC_ELEMENTS
        .iter()
        .copied()
        .find(|element| element.0 == symbol)
}

fn element_color(symbol: &str) -> Option<JsonValue> {
    ELEMENT_COLORS
        .iter()
        .find(|(entry_symbol, _, _)| *entry_symbol == symbol)
        .map(|(_, background, foreground)| {
            json!({
                "background": background,
                "foreground": foreground,
            })
        })
}

fn element_json((symbol, atomic_number, name, column, row): (&str, u8, &str, u8, u8)) -> JsonValue {
    json!({
        "symbol": symbol,
        "atomicNumber": atomic_number,
        "name": name,
        "column": column,
        "row": row,
        "color": element_color(symbol),
    })
}
