use super::*;

const SPECTRUM_Y_PADDING_FRACTION: f64 = 0.05;
const SPECTRUM_CURVE_POINTS_PER_PT: f64 = 4.0;

pub(super) fn render_spectrum_object(
    out: &mut Vec<RenderPrimitive>,
    document: &ChemSemaDocument,
    object: &SceneObject,
) {
    let Some(spectrum) = object.payload.spectrum.as_ref() else {
        return;
    };
    if spectrum.validate().is_err() {
        return;
    }
    let Some([local_x, local_y, width, height]) = object.payload.bbox else {
        return;
    };
    if width <= EPSILON || height <= EPSILON {
        return;
    }
    let outer_left = object.transform.translate[0] + local_x;
    let top = object.transform.translate[1] + local_y;
    let right = outer_left + width;
    let bottom = top + height;
    let style = object
        .style_ref
        .as_ref()
        .and_then(|style_ref| document.styles.get(style_ref));
    let stroke = style
        .and_then(|value| style_nullable_string(value, "stroke"))
        .unwrap_or_else(|| "#000000".to_string());
    let label_fill = style
        .and_then(|value| style_nullable_string(value, "fill"))
        .unwrap_or_else(|| stroke.clone());
    let stroke_width = style
        .and_then(|value| style_number(value, "strokeWidth"))
        .unwrap_or(DEFAULT_BOND_STROKE)
        .max(0.0);
    let font_size = style
        .and_then(|value| style_number(value, "fontSize"))
        .unwrap_or(DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT)
        .max(1.0);
    let font_family = style
        .and_then(|value| style_string(value, "fontFamily"))
        .unwrap_or_else(|| document.style.label_style.font_family.clone());
    let font_weight = style
        .and_then(|value| style_number(value, "fontWeight"))
        .map(|value| value.round() as u32)
        .unwrap_or(400);
    let font_style = style
        .and_then(|value| style_string(value, "fontStyle"))
        .unwrap_or_else(|| "normal".to_string());
    let underline = style
        .and_then(|value| value.get("underline"))
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);
    let outline = style
        .and_then(|value| value.get("outline"))
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);
    let shadow = style
        .and_then(|value| value.get("shadow"))
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);
    let script = style
        .and_then(|value| style_string(value, "script"))
        .unwrap_or_else(|| "normal".to_string());
    let left = if spectrum.y_axis_label.is_empty() {
        outer_left
    } else {
        (outer_left + font_size * 3.7).min(right - font_size * 2.0)
    };
    if right - left <= EPSILON {
        return;
    }
    let object_id = Some(object.id.clone());
    let line = |from: Point, to: Point, out: &mut Vec<RenderPrimitive>| {
        push_line(
            out,
            from,
            to,
            &stroke,
            stroke_width,
            Vec::new(),
            RenderRole::DocumentGraphic,
            object_id.clone(),
        );
    };
    line(Point::new(left, top), Point::new(right, top), out);
    line(Point::new(right, top), Point::new(right, bottom), out);
    line(Point::new(right, bottom), Point::new(left, bottom), out);
    line(Point::new(left, bottom), Point::new(left, top), out);

    let x_high = spectrum.x_high();
    let x_min = spectrum.x_low.min(x_high);
    let x_max = spectrum.x_low.max(x_high);
    let x_span = x_max - x_min;
    if x_span > EPSILON {
        let label_chars = format_tick(x_min)
            .chars()
            .count()
            .max(format_tick(x_max).chars().count()) as f64;
        let target_ticks = ((right - left) / (font_size * (label_chars + 1.5))).clamp(2.0, 6.0);
        let major_step = nice_step(x_span / target_ticks);
        let minor_step = major_step * 0.5;
        for value in tick_values(x_min, x_max, minor_step) {
            let x = spectrum_x_position(&spectrum, value, left, right);
            let is_major = is_multiple(value, major_step);
            line(
                Point::new(x, bottom),
                Point::new(x, bottom + font_size * if is_major { 0.6 } else { 0.36 }),
                out,
            );
            if is_major && (value - x_high).abs() > major_step * 1.0e-7 {
                push_spectrum_text(
                    out,
                    object,
                    x,
                    bottom + font_size * 1.4,
                    format_tick_for_step(value, major_step),
                    font_size,
                    &font_family,
                    font_weight,
                    &font_style,
                    underline,
                    outline,
                    shadow,
                    &script,
                    &label_fill,
                    Some("middle"),
                    0.0,
                    None,
                );
            }
        }
    }
    if !spectrum.x_axis_label.is_empty() {
        push_spectrum_text(
            out,
            object,
            (left + right) * 0.5,
            bottom + font_size * 2.43,
            spectrum.x_axis_label.clone(),
            font_size,
            &font_family,
            font_weight,
            &font_style,
            underline,
            outline,
            shadow,
            &script,
            &label_fill,
            Some("middle"),
            0.0,
            None,
        );
    }

    let decoded = spectrum.decoded_points().collect::<Vec<_>>();
    let (data_min, data_max) = finite_min_max(&decoded);
    let data_span = (data_max - data_min).abs();
    let padding = if data_span <= EPSILON {
        data_max.abs().max(1.0) * SPECTRUM_Y_PADDING_FRACTION
    } else {
        data_span * SPECTRUM_Y_PADDING_FRACTION
    };
    let y_min = data_min - padding;
    let y_max = data_max + padding;

    if !spectrum.y_axis_label.is_empty() {
        let major_step = nice_step((data_max - data_min).abs() / 3.0);
        if major_step > EPSILON {
            for value in tick_values(data_min, data_max, major_step * 0.5) {
                let y = spectrum_y_position(value, y_min, y_max, top, bottom);
                let is_major = is_multiple(value, major_step);
                line(
                    Point::new(left, y),
                    Point::new(left - font_size * if is_major { 0.31 } else { 0.186 }, y),
                    out,
                );
                if is_major {
                    push_spectrum_text(
                        out,
                        object,
                        left - font_size * 0.85,
                        y + font_size * 0.56,
                        format_tick_for_step(value, major_step),
                        font_size,
                        &font_family,
                        font_weight,
                        &font_style,
                        underline,
                        outline,
                        shadow,
                        &script,
                        &label_fill,
                        Some("end"),
                        0.0,
                        None,
                    );
                }
            }
        }
        let center = Point::new(outer_left + font_size * 0.8, (top + bottom) * 0.5);
        push_spectrum_text(
            out,
            object,
            center.x,
            center.y,
            spectrum.y_axis_label.clone(),
            font_size,
            &font_family,
            font_weight,
            &font_style,
            underline,
            outline,
            shadow,
            &script,
            &label_fill,
            Some("middle"),
            -90.0,
            Some(center),
        );
    }

    let sampled = extrema_preserving_samples(&decoded, (right - left).abs());
    let denominator = spectrum.data_points.len() as f64;
    let points = sampled
        .into_iter()
        .map(|(index, value)| {
            let x = right - (right - left) * index as f64 / denominator;
            Point::new(x, spectrum_y_position(value, y_min, y_max, top, bottom))
        })
        .collect::<Vec<_>>();
    if points.len() >= 2 {
        push_polyline(
            out,
            points,
            &stroke,
            stroke_width,
            Vec::new(),
            Some("butt".to_string()),
            Some("miter".to_string()),
            RenderRole::DocumentGraphic,
            object_id,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn push_spectrum_text(
    out: &mut Vec<RenderPrimitive>,
    object: &SceneObject,
    x: f64,
    y: f64,
    text: String,
    font_size: f64,
    font_family: &str,
    font_weight: u32,
    font_style: &str,
    underline: bool,
    outline: bool,
    shadow: bool,
    script: &str,
    fill: &str,
    text_anchor: Option<&str>,
    rotate: f64,
    rotate_center: Option<Point>,
) {
    let run = crate::LabelRun {
        text: text.clone(),
        font_family: Some(font_family.to_string()),
        font_size: Some(font_size),
        font_weight: Some(font_weight),
        font_style: Some(font_style.to_string()),
        underline: Some(underline),
        outline: Some(outline),
        shadow: Some(shadow),
        script: Some(script.to_string()),
        fill: Some(fill.to_string()),
        ..Default::default()
    };
    push_text_rotated(
        out,
        x,
        y,
        None,
        text,
        font_size,
        Some(font_family.to_string()),
        Some(fill.to_string()),
        text_anchor.map(ToString::to_string),
        vec![run],
        Some(object.id.clone()),
        rotate,
        rotate_center,
    );
}

fn spectrum_x_position(spectrum: &crate::SpectrumData, value: f64, left: f64, right: f64) -> f64 {
    let denominator = spectrum.x_high() - spectrum.x_low;
    if denominator.abs() <= EPSILON {
        return right;
    }
    right - (right - left) * (value - spectrum.x_low) / denominator
}

fn spectrum_y_position(value: f64, y_min: f64, y_max: f64, top: f64, bottom: f64) -> f64 {
    if (y_max - y_min).abs() <= EPSILON {
        return (top + bottom) * 0.5;
    }
    bottom - (bottom - top) * (value - y_min) / (y_max - y_min)
}

fn finite_min_max(values: &[f64]) -> (f64, f64) {
    values
        .iter()
        .copied()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), value| {
            (min.min(value), max.max(value))
        })
}

fn nice_step(raw: f64) -> f64 {
    if !raw.is_finite() || raw <= EPSILON {
        return 0.0;
    }
    let power = 10.0_f64.powf(raw.log10().floor());
    let normalized = raw / power;
    let factor = if normalized <= 1.0 {
        1.0
    } else if normalized <= 2.0 {
        2.0
    } else if normalized <= 5.0 {
        5.0
    } else {
        10.0
    };
    factor * power
}

fn tick_values(min: f64, max: f64, step: f64) -> Vec<f64> {
    if !min.is_finite() || !max.is_finite() || !step.is_finite() || step <= EPSILON {
        return Vec::new();
    }
    let first = (min / step).ceil() as i64;
    let last = (max / step).floor() as i64;
    if last < first || last - first > 10_000 {
        return Vec::new();
    }
    (first..=last).map(|index| index as f64 * step).collect()
}

fn is_multiple(value: f64, step: f64) -> bool {
    if step <= EPSILON {
        return false;
    }
    let nearest = (value / step).round() * step;
    (value - nearest).abs() <= step.abs() * 1.0e-7
}

fn format_tick(value: f64) -> String {
    let value = if value.abs() <= 1.0e-12 { 0.0 } else { value };
    let magnitude = value.abs();
    if magnitude >= 1.0e6 || (magnitude > 0.0 && magnitude < 1.0e-4) {
        return format!("{value:.3e}");
    }
    let mut text = format!("{value:.6}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

fn format_tick_for_step(value: f64, step: f64) -> String {
    if !value.is_finite() || !step.is_finite() || step <= EPSILON {
        return format_tick(value);
    }
    let decimals = if step >= 1.0 {
        0
    } else {
        (-step.log10().floor()).clamp(0.0, 8.0) as usize
    };
    format!("{value:.decimals$}")
}

fn extrema_preserving_samples(values: &[f64], width: f64) -> Vec<(usize, f64)> {
    let bucket_count = (width * SPECTRUM_CURVE_POINTS_PER_PT).ceil().max(1.0) as usize;
    if values.len() <= bucket_count * 2 {
        return values.iter().copied().enumerate().collect();
    }
    let mut out = Vec::with_capacity(bucket_count * 2 + 2);
    for bucket in 0..bucket_count {
        let start = bucket * values.len() / bucket_count;
        let end = ((bucket + 1) * values.len() / bucket_count).max(start + 1);
        let slice = &values[start..end.min(values.len())];
        let mut min = (start, slice[0]);
        let mut max = min;
        for (offset, value) in slice.iter().copied().enumerate() {
            let sample = (start + offset, value);
            if value < min.1 {
                min = sample;
            }
            if value > max.1 {
                max = sample;
            }
        }
        if min.0 <= max.0 {
            out.push(min);
            if max.0 != min.0 {
                out.push(max);
            }
        } else {
            out.push(max);
            out.push(min);
        }
    }
    out.sort_by_key(|sample| sample.0);
    out.dedup_by_key(|sample| sample.0);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downsampling_preserves_single_sample_extrema_and_order() {
        let mut values = vec![0.0; 20_000];
        values[137] = 100.0;
        values[9_003] = -50.0;
        let sampled = extrema_preserving_samples(&values, 100.0);
        assert!(sampled.windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert!(sampled.contains(&(137, 100.0)));
        assert!(sampled.contains(&(9_003, -50.0)));
        assert!(sampled.len() <= 802);
    }

    #[test]
    fn nice_ticks_cover_common_nmr_and_ir_ranges() {
        assert_eq!(nice_step(2.0), 2.0);
        assert_eq!(nice_step(1_084.0), 2_000.0);
        assert_eq!(format_tick(-0.0), "0");
        assert_eq!(format_tick_for_step(0.0, 0.5), "0.0");
        assert_eq!(format_tick_for_step(0.5, 0.5), "0.5");
        assert_eq!(format_tick_for_step(2_000.0, 2_000.0), "2000");
    }
}
