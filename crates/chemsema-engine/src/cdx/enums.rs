pub(super) fn enum_name(value: i16, values: &'static [(i16, &'static str)]) -> &'static str {
    values
        .iter()
        .find_map(|(candidate, name)| (*candidate == value).then_some(*name))
        .unwrap_or("Unspecified")
}

pub(super) fn enum_value(name: &str, values: &'static [(i16, &'static str)]) -> Option<i16> {
    if let Ok(value) = name.parse::<i16>() {
        return Some(value);
    }
    values
        .iter()
        .find_map(|(value, candidate)| candidate.eq_ignore_ascii_case(name).then_some(*value))
}

pub(super) fn decode_bit_flags(value: i16, values: &'static [(i16, &'static str)]) -> String {
    if value == 0 {
        return values
            .iter()
            .find_map(|(flag, name)| (*flag == 0).then_some(*name))
            .unwrap_or("0")
            .to_string();
    }
    let names = values
        .iter()
        .filter_map(|(flag, name)| (*flag != 0 && value & *flag == *flag).then_some(*name))
        .collect::<Vec<_>>();
    if names.is_empty() {
        value.to_string()
    } else {
        names.join(" ")
    }
}

pub(super) fn encode_bit_flags(value: &str, values: &'static [(i16, &'static str)]) -> Option<i16> {
    if let Ok(numeric) = value.parse::<i16>() {
        return Some(numeric);
    }
    let mut encoded = 0_i16;
    let mut matched = false;
    for token in value.split_whitespace() {
        let flag = enum_value(token, values)?;
        encoded |= flag;
        matched = true;
    }
    matched.then_some(encoded)
}
