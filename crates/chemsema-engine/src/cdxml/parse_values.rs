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
    let mut remaining = value?;
    Some([
        parse_chemdraw_coordinate_component(&mut remaining),
        parse_chemdraw_coordinate_component(&mut remaining),
    ])
}

fn parse_chemdraw_coordinate_component(remaining: &mut &str) -> f64 {
    *remaining = remaining.trim_start_matches([' ', '\t', '\r', '\n']);
    let Some((consumed, value)) = chemdraw_coordinate_number_prefix(remaining) else {
        // ChemDraw's CDXPoint2D reader leaves the scanner at an invalid token
        // and returns zero. Consequently, if the first component is invalid,
        // the second component sees the same token and is also zero.
        return 0.0;
    };
    *remaining = &remaining[consumed..];
    value
}

fn chemdraw_coordinate_number_prefix(value: &str) -> Option<(usize, f64)> {
    let bytes = value.as_bytes();
    let mut index = usize::from(matches!(bytes.first(), Some(b'+') | Some(b'-')));
    if index >= bytes.len() {
        return None;
    }

    let unsigned = &bytes[index..];
    let non_finite_length = if unsigned
        .get(..8)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"infinity"))
    {
        Some(8)
    } else if unsigned
        .get(..3)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"inf"))
    {
        Some(3)
    } else if unsigned
        .get(..3)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"nan"))
    {
        Some(3)
    } else {
        None
    };
    if let Some(length) = non_finite_length {
        // ChemDraw accepts C-style non-finite spellings, then serializes the
        // non-finite fixed-point coordinate as +0.25 pt.
        return Some((index + length, 0.25));
    }

    let mut digits = 0;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
        digits += 1;
    }
    if index < bytes.len() && bytes[index] == b'.' {
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return None;
    }

    let exponent_start = index;
    if index < bytes.len() && matches!(bytes[index], b'e' | b'E') {
        index += 1;
        if index < bytes.len() && matches!(bytes[index], b'+' | b'-') {
            index += 1;
        }
        let exponent_digits_start = index;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
        if index == exponent_digits_start {
            index = exponent_start;
        }
    }

    let parsed = value[..index].parse::<f64>().ok()?;
    Some((index, if parsed.is_finite() { parsed } else { 0.25 }))
}

pub(super) fn parse_xyz2(value: Option<&str>) -> Option<[f64; 2]> {
    parse_xy(value)
}

#[cfg(test)]
mod point_lexical_tests {
    use super::parse_xy;

    #[test]
    fn point_components_follow_chemdraws_sequential_numeric_scanner() {
        for (source, expected) in [
            ("150 100", [150.0, 100.0]),
            ("150 foofoo", [150.0, 0.0]),
            ("foofoo 100", [0.0, 0.0]),
            ("150", [150.0, 0.0]),
            ("", [0.0, 0.0]),
            ("150 100 extra", [150.0, 100.0]),
            ("150foo 100", [150.0, 0.0]),
            ("150 100foo", [150.0, 100.0]),
            ("1.5e2 1e2", [150.0, 100.0]),
            ("+150 -20", [150.0, -20.0]),
            ("1e 100", [1.0, 0.0]),
            (".5 100", [0.5, 100.0]),
            ("150,100", [150.0, 0.0]),
        ] {
            assert_eq!(parse_xy(Some(source)), Some(expected), "{source:?}");
        }
        assert_eq!(parse_xy(None), None);
    }

    #[test]
    fn point_non_finite_and_overflow_components_use_chemdraws_quarter_point_value() {
        for source in [
            "NaN 100",
            "nan 100",
            "Infinity 100",
            "-Infinity 100",
            "inf 100",
            "1e999 100",
        ] {
            assert_eq!(parse_xy(Some(source)), Some([0.25, 100.0]), "{source:?}");
        }
        assert_eq!(parse_xy(Some("150 NaN")), Some([150.0, 0.25]));
        assert_eq!(parse_xy(Some("1e-999 100")), Some([0.0, 100.0]));
    }
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
