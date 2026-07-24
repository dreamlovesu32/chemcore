use super::*;

pub(super) fn read_i8(data: &[u8]) -> Option<i8> {
    data.first().map(|value| *value as i8)
}

pub(super) fn read_u8(data: &[u8]) -> Option<u8> {
    data.first().copied()
}

pub(super) fn read_i16(data: &[u8]) -> Option<i16> {
    (data.len() >= 2).then(|| i16::from_le_bytes([data[0], data[1]]))
}

pub(super) fn read_u16(data: &[u8]) -> Option<u16> {
    (data.len() >= 2).then(|| u16::from_le_bytes([data[0], data[1]]))
}

pub(super) fn read_i32(data: &[u8]) -> Option<i32> {
    (data.len() >= 4).then(|| i32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}

pub(super) fn read_u32(data: &[u8]) -> Option<u32> {
    (data.len() >= 4).then(|| u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}

pub(super) fn read_f64(data: &[u8]) -> Option<f64> {
    (data.len() >= 8).then(|| {
        f64::from_le_bytes([
            data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
        ])
    })
}

pub(super) fn read_i16_lossy(data: &[u8]) -> Option<i16> {
    if data.len() == 1 {
        Some(data[0] as i8 as i16)
    } else {
        read_i16(data)
    }
}

pub(super) fn read_u16_lossy(data: &[u8]) -> Option<u16> {
    if data.len() == 1 {
        Some(data[0] as u16)
    } else {
        read_u16(data)
    }
}

pub(super) fn decode_coordinate(data: &[u8]) -> Option<String> {
    Some(fmt_num(read_i32(data)? as f64 / CDX_COORD_FACTOR))
}

pub(super) fn decode_point2d(data: &[u8]) -> Option<String> {
    if data.len() < 8 {
        return None;
    }
    let y = i32::from_le_bytes([data[0], data[1], data[2], data[3]]) as f64 / CDX_COORD_FACTOR;
    let x = i32::from_le_bytes([data[4], data[5], data[6], data[7]]) as f64 / CDX_COORD_FACTOR;
    Some(format!("{} {}", fmt_num(x), fmt_num(y)))
}

pub(super) fn decode_point3d(data: &[u8]) -> Option<String> {
    if data.len() < 12 {
        return None;
    }
    let x = i32::from_le_bytes([data[0], data[1], data[2], data[3]]) as f64 / CDX_COORD_FACTOR;
    let y = i32::from_le_bytes([data[4], data[5], data[6], data[7]]) as f64 / CDX_COORD_FACTOR;
    let z = i32::from_le_bytes([data[8], data[9], data[10], data[11]]) as f64 / CDX_COORD_FACTOR;
    Some(format!("{} {} {}", fmt_num(x), fmt_num(y), fmt_num(z)))
}

pub(super) fn decode_rectangle(data: &[u8]) -> Option<String> {
    if data.len() < 16 {
        return None;
    }
    let top = i32::from_le_bytes([data[0], data[1], data[2], data[3]]) as f64 / CDX_COORD_FACTOR;
    let left = i32::from_le_bytes([data[4], data[5], data[6], data[7]]) as f64 / CDX_COORD_FACTOR;
    let bottom =
        i32::from_le_bytes([data[8], data[9], data[10], data[11]]) as f64 / CDX_COORD_FACTOR;
    let right =
        i32::from_le_bytes([data[12], data[13], data[14], data[15]]) as f64 / CDX_COORD_FACTOR;
    Some(format!(
        "{} {} {} {}",
        fmt_num(left),
        fmt_num(top),
        fmt_num(right),
        fmt_num(bottom)
    ))
}

pub(super) fn bool_from_bytes(data: &[u8]) -> String {
    if data.first().copied().unwrap_or(1) == 0 {
        "no".to_string()
    } else {
        "yes".to_string()
    }
}

pub(super) fn decode_bond_order(data: &[u8]) -> Option<String> {
    const ORDERS: [&str; 16] = [
        "1",
        "2",
        "3",
        "4",
        "5",
        "6",
        "0.5",
        "1.5",
        "2.5",
        "3.5",
        "4.5",
        "5.5",
        "dative",
        "ionic",
        "hydrogen",
        "threecenter",
    ];
    let value = read_u16(data)?;
    if value == 0 || value == 0xFFFF {
        return Some(String::new());
    }
    let parts: Vec<&str> = ORDERS
        .iter()
        .enumerate()
        .filter_map(|(index, order)| ((value & (1 << index)) != 0).then_some(*order))
        .collect();
    Some(parts.join(" "))
}

pub(super) fn decode_font_style(data: &[u8]) -> Option<(u16, u16, f64, u16)> {
    if data.len() < 8 {
        return None;
    }
    let font = u16::from_le_bytes([data[0], data[1]]);
    let face = u16::from_le_bytes([data[2], data[3]]);
    let size = u16::from_le_bytes([data[4], data[5]]) as f64 / 20.0;
    let color = u16::from_le_bytes([data[6], data[7]]);
    Some((font, face, size, color))
}

pub(super) fn decode_u32_array(data: &[u8]) -> Option<String> {
    if !data.len().is_multiple_of(4) {
        return None;
    }
    Some(
        data.chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]).to_string())
            .collect::<Vec<_>>()
            .join(" "),
    )
}

pub(super) fn decode_u32_counted_array(data: &[u8]) -> Option<String> {
    let count = read_u16(data)? as usize;
    let body = data.get(2..2 + count * 4)?;
    decode_u32_array(body)
}

pub(super) fn decode_element_list(data: &[u8]) -> Option<String> {
    let signed_count = read_i16(data)?;
    let count = signed_count.unsigned_abs() as usize;
    let body = data.get(2..2 + count * 2)?;
    let values = body
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]).to_string())
        .collect::<Vec<_>>()
        .join(" ");
    Some(if signed_count < 0 {
        format!("NOT {values}")
    } else {
        values
    })
}

pub(super) fn decode_curve_points(data: &[u8], dimensions: usize) -> Option<String> {
    let count = read_u16(data)? as usize;
    let body = data.get(2..2 + count * dimensions * 4)?;
    let mut values = Vec::with_capacity(count * dimensions);
    for point in body.chunks_exact(dimensions * 4) {
        if dimensions == 2 {
            values.extend(
                decode_point2d(point)?
                    .split_whitespace()
                    .map(ToString::to_string),
            );
        } else {
            values.extend(
                decode_point3d(point)?
                    .split_whitespace()
                    .map(ToString::to_string),
            );
        }
    }
    Some(values.join(" "))
}

pub(super) fn decode_cdx_date(data: &[u8]) -> Option<String> {
    if data.len() < 14 {
        return None;
    }
    Some(
        data[..14]
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]).to_string())
            .collect::<Vec<_>>()
            .join(" "),
    )
}

pub(super) fn decode_represents_property(data: &[u8]) -> Option<String> {
    if data.len() < 6 {
        return None;
    }
    Some(format!(
        "{} 0x{:04X}",
        read_u32(data)?,
        u16::from_le_bytes([data[4], data[5]])
    ))
}

pub(super) fn decode_generic_list(data: &[u8]) -> Option<String> {
    let signed_count = read_i16(data)?;
    let count = signed_count.unsigned_abs() as usize;
    let mut offset = 2usize;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        let len = read_u16(data.get(offset..)?)? as usize;
        offset += 2;
        let item = data.get(offset..offset + len)?;
        offset += len;
        values.push(parse_cdx_string(item, None).text);
    }
    let joined = values.join(" ");
    Some(if signed_count < 0 {
        format!("NOT {joined}")
    } else {
        joined
    })
}

pub(super) fn decode_i16_counted_list(data: &[u8]) -> Option<String> {
    if data.len() < 2 {
        return None;
    }
    let count = u16::from_le_bytes([data[0], data[1]]) as usize;
    if data.len() < 2 + count * 2 {
        return None;
    }
    Some(
        data[2..2 + count * 2]
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]).to_string())
            .collect::<Vec<_>>()
            .join(" "),
    )
}
