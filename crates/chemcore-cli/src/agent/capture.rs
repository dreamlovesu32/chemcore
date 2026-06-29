use super::*;

pub(crate) fn capture_command(args: &[String]) -> Result<(), String> {
    let mut input = None;
    let mut target = None;
    let mut output = None;
    let mut format = None;
    let mut expansion = CropExpansion::uniform_abs(8.0);
    let mut raster = RasterOptions::default();
    let mut pretty = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--target" | "-t" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    parse_target_selector(
                        args.get(index)
                            .ok_or_else(|| "--target requires a selector.".to_string())?,
                    )?,
                )?;
            }
            "--targets" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    parse_target_selection_arg(args.get(index).ok_or_else(|| {
                        "--targets requires selectors separated by semicolons.".to_string()
                    })?)?,
                )?;
            }
            "--object" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    TargetSelector::Object(
                        args.get(index)
                            .ok_or_else(|| "--object requires an object id.".to_string())?
                            .clone(),
                    ),
                )?;
            }
            "--molecule" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    TargetSelector::Molecule(parse_usize_arg("--molecule", args.get(index))?),
                )?;
            }
            "--node" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    TargetSelector::Node(
                        args.get(index)
                            .ok_or_else(|| "--node requires a node id.".to_string())?
                            .clone(),
                    ),
                )?;
            }
            "--bond" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    TargetSelector::Bond(
                        args.get(index)
                            .ok_or_else(|| "--bond requires a bond id.".to_string())?
                            .clone(),
                    ),
                )?;
            }
            "--bounds" => {
                index += 1;
                add_target_arg(
                    &mut target,
                    TargetSelector::Bounds(parse_bounds_arg(
                        args.get(index)
                            .ok_or_else(|| "--bounds requires minX,minY,maxX,maxY.".to_string())?,
                    )?),
                )?;
            }
            "--out" | "-o" => {
                index += 1;
                output = Some(
                    args.get(index)
                        .ok_or_else(|| "--out requires a path.".to_string())?
                        .clone(),
                );
            }
            "--format" | "-f" => {
                index += 1;
                format =
                    Some(parse_capture_format(args.get(index).ok_or_else(|| {
                        "--format requires svg or png.".to_string()
                    })?)?);
            }
            "--padding" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--padding",
                    args.get(index)
                        .ok_or_else(|| "--padding requires a number.".to_string())?,
                )?;
                expansion.abs_left = value;
                expansion.abs_top = value;
                expansion.abs_right = value;
                expansion.abs_bottom = value;
            }
            "--expand" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand",
                    args.get(index)
                        .ok_or_else(|| "--expand requires a number.".to_string())?,
                )?;
                expansion.abs_left = value;
                expansion.abs_top = value;
                expansion.abs_right = value;
                expansion.abs_bottom = value;
            }
            "--expand-x" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand-x",
                    args.get(index)
                        .ok_or_else(|| "--expand-x requires a number.".to_string())?,
                )?;
                expansion.abs_left = value;
                expansion.abs_right = value;
            }
            "--expand-y" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand-y",
                    args.get(index)
                        .ok_or_else(|| "--expand-y requires a number.".to_string())?,
                )?;
                expansion.abs_top = value;
                expansion.abs_bottom = value;
            }
            "--expand-left" => {
                index += 1;
                expansion.abs_left = parse_non_negative_f64(
                    "--expand-left",
                    args.get(index)
                        .ok_or_else(|| "--expand-left requires a number.".to_string())?,
                )?;
            }
            "--expand-right" => {
                index += 1;
                expansion.abs_right = parse_non_negative_f64(
                    "--expand-right",
                    args.get(index)
                        .ok_or_else(|| "--expand-right requires a number.".to_string())?,
                )?;
            }
            "--expand-top" => {
                index += 1;
                expansion.abs_top = parse_non_negative_f64(
                    "--expand-top",
                    args.get(index)
                        .ok_or_else(|| "--expand-top requires a number.".to_string())?,
                )?;
            }
            "--expand-bottom" => {
                index += 1;
                expansion.abs_bottom = parse_non_negative_f64(
                    "--expand-bottom",
                    args.get(index)
                        .ok_or_else(|| "--expand-bottom requires a number.".to_string())?,
                )?;
            }
            "--expand-rel" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand-rel",
                    args.get(index)
                        .ok_or_else(|| "--expand-rel requires a fraction.".to_string())?,
                )?;
                expansion.rel_left = value;
                expansion.rel_top = value;
                expansion.rel_right = value;
                expansion.rel_bottom = value;
            }
            "--expand-rel-x" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand-rel-x",
                    args.get(index)
                        .ok_or_else(|| "--expand-rel-x requires a fraction.".to_string())?,
                )?;
                expansion.rel_left = value;
                expansion.rel_right = value;
            }
            "--expand-rel-y" => {
                index += 1;
                let value = parse_non_negative_f64(
                    "--expand-rel-y",
                    args.get(index)
                        .ok_or_else(|| "--expand-rel-y requires a fraction.".to_string())?,
                )?;
                expansion.rel_top = value;
                expansion.rel_bottom = value;
            }
            "--expand-rel-left" => {
                index += 1;
                expansion.rel_left = parse_non_negative_f64(
                    "--expand-rel-left",
                    args.get(index)
                        .ok_or_else(|| "--expand-rel-left requires a fraction.".to_string())?,
                )?;
            }
            "--expand-rel-right" => {
                index += 1;
                expansion.rel_right = parse_non_negative_f64(
                    "--expand-rel-right",
                    args.get(index)
                        .ok_or_else(|| "--expand-rel-right requires a fraction.".to_string())?,
                )?;
            }
            "--expand-rel-top" => {
                index += 1;
                expansion.rel_top = parse_non_negative_f64(
                    "--expand-rel-top",
                    args.get(index)
                        .ok_or_else(|| "--expand-rel-top requires a fraction.".to_string())?,
                )?;
            }
            "--expand-rel-bottom" => {
                index += 1;
                expansion.rel_bottom = parse_non_negative_f64(
                    "--expand-rel-bottom",
                    args.get(index)
                        .ok_or_else(|| "--expand-rel-bottom requires a fraction.".to_string())?,
                )?;
            }
            "--scale" => {
                index += 1;
                raster.scale = parse_positive_f64(
                    "--scale",
                    args.get(index)
                        .ok_or_else(|| "--scale requires a positive number.".to_string())?,
                )?;
            }
            "--width" => {
                index += 1;
                raster.width = Some(parse_positive_u32(
                    "--width",
                    args.get(index)
                        .ok_or_else(|| "--width requires a positive integer.".to_string())?,
                )?);
            }
            "--height" => {
                index += 1;
                raster.height = Some(parse_positive_u32(
                    "--height",
                    args.get(index)
                        .ok_or_else(|| "--height requires a positive integer.".to_string())?,
                )?);
            }
            "--pretty" => pretty = true,
            value if input.is_none() => input = Some(value.to_string()),
            value => return Err(format!("Unexpected capture argument '{value}'.")),
        }
        index += 1;
    }
    let input = input.ok_or_else(|| "capture requires an input file.".to_string())?;
    let target = target.ok_or_else(|| {
        "capture requires --target <object:id|molecule:index|node:id|bond:id|all>, repeated --target values, --targets, or --bounds."
            .to_string()
    })?;
    let (output, format, output_defaulted) = resolve_capture_output(output, format)?;

    let engine = load_engine_from_file(&input)?;
    let document = engine_document(&engine)?;
    let bounds = target_bounds(&document, &target)?;
    let view_box = expanded_view_box(bounds, expansion);
    let render = capture_render_primitives(&document, &target, view_box);
    let render_output =
        write_capture_output(&render.primitives, view_box, &output, format, raster)?;
    let primitive_count = render.primitives.len();
    write_json_value(
        json!({
            "ok": true,
            "input": input,
            "target": target.to_json(),
            "warnings": default_capture_warnings(output_defaulted, &output),
            "output": {
                "path": output,
                "format": format.as_str(),
                "defaulted": output_defaulted,
                "verified": true,
                "bytes": render_output.bytes,
                "pixelSize": render_output.pixel_size.map(PixelSize::to_json),
            },
            "bounds": bounds_json(bounds),
            "viewBox": view_box_json(view_box),
            "expansion": expansion.to_json(),
            "render": {
                "mode": render.mode,
                "primitiveCount": primitive_count,
                "targets": render.targets.to_json(),
            },
        }),
        None,
        pretty,
    )
}

pub(super) fn capture_render_primitives(
    document: &ChemcoreDocument,
    target: &TargetSelector,
    view_box: [f64; 4],
) -> CaptureRender {
    if matches!(target, TargetSelector::All) {
        return CaptureRender {
            primitives: render_document(document),
            mode: "full-document",
            targets: RegionRenderTargets::default(),
        };
    }

    let targets = region_render_targets(document, view_box_to_bounds(view_box));
    if targets.is_empty() {
        return CaptureRender {
            primitives: Vec::new(),
            mode: "region-empty",
            targets,
        };
    }

    let primitives =
        render_document_targets(document, &targets.nodes, &targets.bonds, &targets.objects);
    CaptureRender {
        primitives,
        mode: "region-targets",
        targets,
    }
}

pub(super) fn region_render_targets(
    document: &ChemcoreDocument,
    query_bounds: [f64; 4],
) -> RegionRenderTargets {
    let mut targets = RegionRenderTargets::default();
    collect_region_scene_object_targets(document, &document.objects, query_bounds, &mut targets);
    targets
}

pub(super) fn collect_region_scene_object_targets(
    document: &ChemcoreDocument,
    objects: &[SceneObject],
    query_bounds: [f64; 4],
    targets: &mut RegionRenderTargets,
) {
    for object in objects {
        if !object.visible {
            continue;
        }

        if object.object_type == "molecule"
            && collect_region_molecule_targets(document, object, query_bounds, targets)
        {
            continue;
        }

        if object.object_type == "group" {
            collect_region_scene_object_targets(document, &object.children, query_bounds, targets);
            if scene_object_fast_bounds(document, object)
                .is_some_and(|bounds| bounds_intersect(bounds, query_bounds))
            {
                targets.objects.insert(object.id.clone());
            }
            continue;
        }

        let bounds = scene_object_fast_bounds(document, object)
            .or_else(|| target_bounds(document, &TargetSelector::Object(object.id.clone())).ok());
        if bounds.is_some_and(|bounds| bounds_intersect(bounds, query_bounds)) {
            targets.objects.insert(object.id.clone());
        }
    }
}

pub(super) fn collect_region_molecule_targets(
    document: &ChemcoreDocument,
    object: &SceneObject,
    query_bounds: [f64; 4],
    targets: &mut RegionRenderTargets,
) -> bool {
    let Some(resource_ref) = object.payload.resource_ref.as_ref() else {
        return false;
    };
    let Some(fragment) = document
        .resources
        .get(resource_ref)
        .and_then(|resource| resource.data.as_fragment())
    else {
        return false;
    };

    for node in &fragment.nodes {
        if bounds_intersect(node_fast_bounds(object, node), query_bounds) {
            targets.nodes.insert(node.id.clone());
        }
    }
    for bond in &fragment.bonds {
        let Some(bounds) = bond_fast_bounds(object, &fragment.nodes, bond) else {
            continue;
        };
        if bounds_intersect(bounds, query_bounds) {
            targets.bonds.insert(bond.id.clone());
        }
    }
    true
}
