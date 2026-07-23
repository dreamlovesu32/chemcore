use super::*;

pub(super) fn page_from_objects(objects: &[SceneObject], background: &str) -> Page {
    let mut max_x: f64 = 640.0;
    let mut max_y: f64 = 480.0;
    for object in objects {
        let tx = object.transform.translate[0];
        let ty = object.transform.translate[1];
        if let Some([x, y, w, h]) = object.payload.bbox {
            max_x = max_x.max(tx + x + w);
            max_y = max_y.max(ty + y + h);
        }
        if let Some(points) = object.payload.extra.get("points").and_then(Value::as_array) {
            for point in points {
                if let Some(coords) = point.as_array() {
                    if let (Some(x), Some(y)) = (
                        coords.first().and_then(Value::as_f64),
                        coords.get(1).and_then(Value::as_f64),
                    ) {
                        max_x = max_x.max(tx + x);
                        max_y = max_y.max(ty + y);
                    }
                }
            }
        }
    }
    Page {
        width: round2(max_x + 24.0),
        height: round2(max_y + 24.0),
        background: background.to_string(),
    }
}

pub(super) fn parse_xy(value: Option<&str>) -> Option<[f64; 2]> {
    let mut parts = value?.split_whitespace();
    Some([parts.next()?.parse().ok()?, parts.next()?.parse().ok()?])
}

pub(super) fn parse_xyz2(value: Option<&str>) -> Option<[f64; 2]> {
    parse_xy(value)
}

pub(super) fn parse_bbox(value: Option<&str>) -> Option<[f64; 4]> {
    let nums: Vec<f64> = value?
        .split_whitespace()
        .take(4)
        .filter_map(|part| part.parse().ok())
        .collect();
    (nums.len() == 4).then(|| {
        [
            nums[0].min(nums[2]),
            nums[1].min(nums[3]),
            nums[0].max(nums[2]),
            nums[1].max(nums[3]),
        ]
    })
}

pub(super) fn parse_f64(value: Option<&str>) -> Option<f64> {
    value?.parse().ok()
}

pub(super) fn parse_i32(value: Option<&str>) -> Option<i32> {
    value?.parse().ok()
}

pub(super) fn parse_i16(value: Option<&str>) -> Option<i16> {
    value?.parse().ok()
}

pub(super) fn parse_u8(value: Option<&str>) -> Option<u8> {
    value?.parse().ok()
}

pub(super) fn parse_u32(value: Option<&str>) -> Option<u32> {
    value?.parse().ok()
}

pub(super) fn parse_scaled_100(value: Option<&str>) -> Option<f64> {
    parse_f64(value).map(|value| value / 100.0)
}

pub(super) fn round2(value: f64) -> f64 {
    crate::round2(value)
}

pub(super) fn has_arrow_attrs(node: &XmlNode) -> bool {
    [
        "ArrowheadHead",
        "ArrowheadTail",
        "ArrowType",
        "ArrowheadType",
    ]
    .into_iter()
    .any(|key| arrow_endpoint_enabled(node.attr(key)))
}

pub(super) fn arrow_endpoint_enabled(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        let normalized = value.trim().to_ascii_lowercase();
        !normalized.is_empty() && !matches!(normalized.as_str(), "none" | "0" | "false")
    })
}

pub(super) fn empty_as_null(value: Option<&str>) -> Value {
    match value.filter(|value| !value.is_empty()) {
        Some(value) => json!(value),
        None => Value::Null,
    }
}

pub(super) fn nonempty_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub(crate) fn element_symbol(atomic_number: u8) -> &'static str {
    const SYMBOLS: [&str; 119] = [
        "", "H", "He", "Li", "Be", "B", "C", "N", "O", "F", "Ne", "Na", "Mg", "Al", "Si", "P", "S",
        "Cl", "Ar", "K", "Ca", "Sc", "Ti", "V", "Cr", "Mn", "Fe", "Co", "Ni", "Cu", "Zn", "Ga",
        "Ge", "As", "Se", "Br", "Kr", "Rb", "Sr", "Y", "Zr", "Nb", "Mo", "Tc", "Ru", "Rh", "Pd",
        "Ag", "Cd", "In", "Sn", "Sb", "Te", "I", "Xe", "Cs", "Ba", "La", "Ce", "Pr", "Nd", "Pm",
        "Sm", "Eu", "Gd", "Tb", "Dy", "Ho", "Er", "Tm", "Yb", "Lu", "Hf", "Ta", "W", "Re", "Os",
        "Ir", "Pt", "Au", "Hg", "Tl", "Pb", "Bi", "Po", "At", "Rn", "Fr", "Ra", "Ac", "Th", "Pa",
        "U", "Np", "Pu", "Am", "Cm", "Bk", "Cf", "Es", "Fm", "Md", "No", "Lr", "Rf", "Db", "Sg",
        "Bh", "Hs", "Mt", "Ds", "Rg", "Cn", "Nh", "Fl", "Mc", "Lv", "Ts", "Og",
    ];
    SYMBOLS.get(atomic_number as usize).copied().unwrap_or("C")
}
