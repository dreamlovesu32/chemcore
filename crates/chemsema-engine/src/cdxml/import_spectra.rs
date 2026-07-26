use super::*;

pub(in crate::cdxml) fn append_spectrum_objects(
    root: &XmlNode,
    objects: &mut Vec<SceneObject>,
    styles: &mut BTreeMap<String, Value>,
    defaults: CdxmlDefaults,
    colors: &CdxmlColorTable,
    fonts: &BTreeMap<String, String>,
) -> Result<(), String> {
    for (index, node) in (1usize..).zip(
        descendants(root)
            .into_iter()
            .filter(|node| node.is("spectrum")),
    ) {
        let source_id = node.attr("id").unwrap_or("<missing id>");
        let bbox = parse_bbox(node.attr("BoundingBox")).ok_or_else(|| {
            format!("CDXML spectrum '{source_id}' is missing a valid BoundingBox")
        })?;
        let data_points = parse_spectrum_data_points(&node.full_text(), source_id)?;
        let spectrum = crate::SpectrumData {
            class: crate::SpectrumClass::from_cdxml(node.attr("Class"))?,
            x_low: required_spectrum_number(node, "XLow")?,
            x_spacing: required_spectrum_number(node, "XSpacing")?,
            x_type: crate::SpectrumXAxisType::from_cdxml(node.attr("XType"))?,
            x_axis_label: node.attr("XAxisLabel").unwrap_or("").to_string(),
            y_low: parse_f64(node.attr("YLow")).unwrap_or(0.0),
            y_scale: parse_f64(node.attr("YScale")).unwrap_or(1.0),
            y_type: crate::SpectrumYAxisType::from_cdxml(node.attr("YType"))?,
            y_axis_label: node.attr("YAxisLabel").unwrap_or("").to_string(),
            data_points,
        };
        spectrum
            .validate()
            .map_err(|error| format!("invalid CDXML spectrum '{source_id}': {error}"))?;

        let label_font = node
            .attr("LabelFont")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(defaults.label_font);
        let label_face = node
            .attr("LabelFace")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(defaults.label_face);
        let label_size = parse_f64(node.attr("LabelSize")).unwrap_or(defaults.label_size);
        let label_color = node
            .attr("LabelColor")
            .or_else(|| node.attr("color"))
            .map(ToString::to_string)
            .unwrap_or_else(|| defaults.color.to_string());
        let style_run = label_source_run(
            "",
            label_face,
            &label_font.to_string(),
            &label_color,
            label_size,
            colors,
            fonts,
        );
        let stroke = colors.resolve(node.attr("color"));
        let style_id = format!("style_spectrum_{index:03}");
        styles.insert(
            style_id.clone(),
            json!({
                "kind": "spectrum",
                "stroke": stroke,
                "strokeWidth": parse_f64(node.attr("LineWidth")).unwrap_or(defaults.line_width),
                "fontFamily": style_run.font_family.unwrap_or_else(|| "Arial".to_string()),
                "fontSize": style_run.font_size.unwrap_or(label_size),
                "fontWeight": style_run.font_weight.unwrap_or(400),
                "fontStyle": style_run.font_style.unwrap_or_else(|| "normal".to_string()),
                "underline": style_run.underline.unwrap_or(false),
                "outline": style_run.outline.unwrap_or(false),
                "shadow": style_run.shadow.unwrap_or(false),
                "script": style_run.script.unwrap_or_else(|| "normal".to_string()),
                "fill": style_run.fill.unwrap_or_else(|| colors.resolve(Some(&label_color))),
            }),
        );

        let mut payload = ObjectPayload {
            resource_ref: None,
            bbox: Some([
                0.0,
                0.0,
                round2(bbox[2] - bbox[0]),
                round2(bbox[3] - bbox[1]),
            ]),
            spectrum: None,
            geometry: None,
            constraint: None,
            table: None,
            stoichiometry_grid: None,
            gel_electrophoresis: None,
            extra: BTreeMap::new(),
        };
        payload.spectrum = Some(spectrum);
        objects.push(SceneObject {
            id: format!("obj_spectrum_{index:03}"),
            object_type: "spectrum".to_string(),
            name: format!("spectrum {index}"),
            visible: node
                .attr("Visible")
                .map(|value| value != "no")
                .unwrap_or(true),
            locked: false,
            z_index: parse_i32(node.attr("Z")).unwrap_or(18),
            transform: Transform {
                translate: [round2(bbox[0]), round2(bbox[1])],
                rotate: 0.0,
                scale: [1.0, 1.0],
            },
            style_ref: Some(style_id),
            link_policy: Default::default(),
            meta: json!({
                "source": "cdxml",
                "spectrumId": node.attr("id"),
            }),
            payload,
            children: Vec::new(),
        });
    }
    Ok(())
}

fn required_spectrum_number(node: &XmlNode, name: &str) -> Result<f64, String> {
    parse_f64(node.attr(name)).ok_or_else(|| {
        format!(
            "CDXML spectrum '{}' is missing required finite {name}",
            node.attr("id").unwrap_or("<missing id>")
        )
    })
}

fn parse_spectrum_data_points(text: &str, source_id: &str) -> Result<Vec<f64>, String> {
    let mut points = Vec::new();
    for token in text.split_whitespace() {
        let value = token.parse::<f64>().map_err(|_| {
            format!("CDXML spectrum '{source_id}' contains invalid data point '{token}'")
        })?;
        if !value.is_finite() {
            return Err(format!(
                "CDXML spectrum '{source_id}' contains non-finite data point '{token}'"
            ));
        }
        points.push(value);
        if points.len() > crate::SpectrumData::MAX_DATA_POINTS {
            return Err(format!(
                "CDXML spectrum '{source_id}' exceeds the {} point limit",
                crate::SpectrumData::MAX_DATA_POINTS
            ));
        }
    }
    Ok(points)
}
