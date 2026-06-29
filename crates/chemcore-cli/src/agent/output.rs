use super::*;

pub(super) struct CaptureWriteResult {
    pub(super) pixel_size: Option<PixelSize>,
    pub(super) bytes: u64,
}

pub(super) fn resolve_capture_output(
    output: Option<String>,
    format: Option<CaptureFormat>,
) -> Result<(String, CaptureFormat, bool), String> {
    if let Some(output) = output {
        if output == "-" {
            return Err(
                "capture writes image data to a file; stdout is reserved for the JSON manifest."
                    .to_string(),
            );
        }
        let format = format
            .or_else(|| infer_capture_format_from_path(&output))
            .ok_or_else(|| {
                "Capture output format is ambiguous; use --out <path.svg|path.png> or --format svg|png."
                    .to_string()
            })?;
        return Ok((output, format, false));
    }

    let format = format.unwrap_or(CaptureFormat::Png);
    Ok((
        default_capture_output_path(format).display().to_string(),
        format,
        true,
    ))
}

pub(super) fn default_capture_output_path(format: CaptureFormat) -> PathBuf {
    default_output_dir().join(format!(
        "capture-{}-{}.{}",
        std::process::id(),
        timestamp_millis(),
        format.as_str()
    ))
}

pub(super) fn default_output_dir() -> PathBuf {
    std::env::temp_dir().join(DEFAULT_OUTPUT_DIR_NAME)
}

pub(super) fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

pub(super) fn default_capture_warnings(defaulted: bool, path: &str) -> Vec<Value> {
    if defaulted {
        vec![json!({
            "kind": "default_output_path",
            "message": "--out was not provided; capture wrote a PNG to the default temp path. Pass --out <path> to choose a persistent location.",
            "path": path,
        })]
    } else {
        Vec::new()
    }
}

pub(super) fn default_payload_warnings(defaulted: bool, path: &Path) -> Vec<Value> {
    if defaulted {
        vec![json!({
            "kind": "default_payload_path",
            "message": "--payload was not provided; copy wrote the clipboard payload JSON to the default temp path. Pass --payload <path> to choose a persistent location.",
            "path": path.display().to_string(),
        })]
    } else {
        Vec::new()
    }
}

pub(super) fn write_capture_output(
    primitives: &[RenderPrimitive],
    view_box: [f64; 4],
    output: &str,
    format: CaptureFormat,
    raster: RasterOptions,
) -> Result<CaptureWriteResult, String> {
    let svg = primitives_to_svg_viewbox(primitives, view_box, None);
    match format {
        CaptureFormat::Svg => {
            let bytes = write_text_output(Some(output), &svg)?;
            Ok(CaptureWriteResult {
                pixel_size: None,
                bytes,
            })
        }
        CaptureFormat::Png => {
            let pixel_size = pixel_size_for_view_box(view_box, raster)?;
            let bytes = write_svg_png_output(&svg, view_box, output, pixel_size)?;
            Ok(CaptureWriteResult {
                pixel_size: Some(pixel_size),
                bytes,
            })
        }
    }
}

pub(super) fn pixel_size_for_view_box(
    view_box: [f64; 4],
    raster: RasterOptions,
) -> Result<PixelSize, String> {
    let source_width = view_box[2].max(1.0);
    let source_height = view_box[3].max(1.0);
    let (width, height) = match (raster.width, raster.height) {
        (Some(width), Some(height)) => (width, height),
        (Some(width), None) => {
            let height = ((width as f64) * source_height / source_width)
                .round()
                .max(1.0) as u32;
            (width, height)
        }
        (None, Some(height)) => {
            let width = ((height as f64) * source_width / source_height)
                .round()
                .max(1.0) as u32;
            (width, height)
        }
        (None, None) => (
            (source_width * raster.scale).round().max(1.0) as u32,
            (source_height * raster.scale).round().max(1.0) as u32,
        ),
    };
    validate_png_size(width, height)?;
    Ok(PixelSize {
        width,
        height,
        scale_x: width as f64 / source_width,
        scale_y: height as f64 / source_height,
    })
}

pub(super) fn validate_png_size(width: u32, height: u32) -> Result<(), String> {
    if width > MAX_CAPTURE_SIDE_PX || height > MAX_CAPTURE_SIDE_PX {
        return Err(format!(
            "PNG capture dimensions {width}x{height} exceed the side limit {MAX_CAPTURE_SIDE_PX}px. Use --scale, --width, or --height to request a smaller image."
        ));
    }
    let pixels = width as u64 * height as u64;
    if pixels > MAX_CAPTURE_PIXELS {
        return Err(format!(
            "PNG capture dimensions {width}x{height} require {pixels} pixels, above the limit {MAX_CAPTURE_PIXELS}. Use --scale, --width, or --height to request a smaller image."
        ));
    }
    Ok(())
}

pub(super) fn write_svg_png_output(
    svg: &str,
    view_box: [f64; 4],
    output: &str,
    pixel_size: PixelSize,
) -> Result<u64, String> {
    let pixmap = render_svg_png_pixmap(svg, view_box, pixel_size)?;
    ensure_output_parent_path(Path::new(output))?;
    pixmap
        .save_png(output)
        .map_err(|error| format!("Failed to write PNG {output}: {error}"))?;
    verify_file_written(Path::new(output), 8, "PNG capture")
}

pub(super) fn render_svg_png_pixmap(
    svg: &str,
    view_box: [f64; 4],
    pixel_size: PixelSize,
) -> Result<tiny_skia::Pixmap, String> {
    let svg = svg_with_explicit_size(svg, view_box);
    let options = usvg_options_with_system_fonts();
    let tree = usvg::Tree::from_str(&svg, &options)
        .map_err(|error| format!("Failed to parse capture SVG for PNG output: {error}"))?;
    let mut pixmap = tiny_skia::Pixmap::new(pixel_size.width, pixel_size.height)
        .ok_or_else(|| "Failed to allocate PNG pixmap.".to_string())?;
    pixmap.fill(tiny_skia::Color::WHITE);
    let mut pixmap_mut = pixmap.as_mut();
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(pixel_size.scale_x as f32, pixel_size.scale_y as f32),
        &mut pixmap_mut,
    );
    Ok(pixmap)
}

pub(super) fn usvg_options_with_system_fonts() -> usvg::Options<'static> {
    let mut options = usvg::Options::default();
    options.fontdb = capture_font_database();
    options.font_family = "Arial".to_string();
    options
}

pub(super) fn capture_font_database() -> Arc<fontdb::Database> {
    static FONT_DB: OnceLock<Arc<fontdb::Database>> = OnceLock::new();
    FONT_DB
        .get_or_init(|| {
            let mut database = fontdb::Database::new();
            database.load_system_fonts();
            Arc::new(database)
        })
        .clone()
}

pub(super) fn svg_with_explicit_size(svg: &str, view_box: [f64; 4]) -> String {
    svg.replacen(
        "<svg xmlns=\"http://www.w3.org/2000/svg\"",
        &format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\"",
            view_box[2].max(1.0),
            view_box[3].max(1.0)
        ),
        1,
    )
}
