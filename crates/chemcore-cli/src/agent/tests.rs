use super::*;

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 0.000_001,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn infers_png_capture_format_from_output_path() {
    assert_eq!(
        infer_capture_format_from_path("capture.png"),
        Some(CaptureFormat::Png)
    );
    assert_eq!(
        infer_capture_format_from_path("capture.SVG"),
        Some(CaptureFormat::Svg)
    );
    assert!(parse_capture_format("jpeg").is_err());
}

#[test]
fn capture_output_defaults_to_temp_png() {
    let (path, format, defaulted) = resolve_capture_output(None, None).unwrap();
    assert_eq!(format, CaptureFormat::Png);
    assert!(defaulted);
    assert!(Path::new(&path).starts_with(default_output_dir()));
    assert_eq!(
        Path::new(&path)
            .extension()
            .and_then(|value| value.to_str()),
        Some("png")
    );
}

#[test]
fn explicit_capture_output_without_extension_requires_format() {
    assert!(resolve_capture_output(Some("capture".to_string()), None).is_err());
    let (path, format, defaulted) =
        resolve_capture_output(Some("capture".to_string()), Some(CaptureFormat::Svg)).unwrap();
    assert_eq!(path, "capture");
    assert_eq!(format, CaptureFormat::Svg);
    assert!(!defaulted);
}

#[test]
fn default_output_warnings_are_machine_readable() {
    let capture_warnings = default_capture_warnings(true, "capture.png");
    assert_eq!(capture_warnings[0]["kind"], "default_output_path");
    assert_eq!(capture_warnings[0]["path"], "capture.png");
    assert!(default_capture_warnings(false, "capture.png").is_empty());

    let payload_warnings = default_payload_warnings(true, Path::new("payload.json"));
    assert_eq!(payload_warnings[0]["kind"], "default_payload_path");
    assert_eq!(payload_warnings[0]["path"], "payload.json");
    assert!(default_payload_warnings(false, Path::new("payload.json")).is_empty());
}

#[test]
fn expands_view_box_with_absolute_and_relative_sides() {
    let view_box = expanded_view_box(
        [10.0, 20.0, 30.0, 60.0],
        CropExpansion {
            abs_left: 1.0,
            abs_top: 2.0,
            abs_right: 3.0,
            abs_bottom: 4.0,
            rel_left: 0.1,
            rel_top: 0.25,
            rel_right: 0.2,
            rel_bottom: 0.0,
        },
    );
    assert_close(view_box[0], 7.0);
    assert_close(view_box[1], 8.0);
    assert_close(view_box[2], 30.0);
    assert_close(view_box[3], 56.0);
}

#[test]
fn derives_png_height_from_fixed_width() {
    let pixel_size = pixel_size_for_view_box(
        [0.0, 0.0, 100.0, 50.0],
        RasterOptions {
            scale: 4.0,
            width: Some(1000),
            height: None,
        },
    )
    .unwrap();
    assert_eq!(pixel_size.width, 1000);
    assert_eq!(pixel_size.height, 500);
    assert_close(pixel_size.scale_x, 10.0);
    assert_close(pixel_size.scale_y, 10.0);
}

#[test]
fn png_capture_renders_svg_text() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 80 30"><text x="5" y="22" font-size="22" font-family="sans-serif" fill="#000000">CN</text></svg>"##;
    let pixmap = render_svg_png_pixmap(
        svg,
        [0.0, 0.0, 80.0, 30.0],
        PixelSize {
            width: 800,
            height: 300,
            scale_x: 10.0,
            scale_y: 10.0,
        },
    )
    .unwrap();
    let dark_pixels = pixmap
        .data()
        .chunks_exact(4)
        .filter(|pixel| pixel[0] < 240 || pixel[1] < 240 || pixel[2] < 240)
        .count();
    assert!(
        dark_pixels > 500,
        "text-only SVG should produce visible non-white PNG pixels, got {dark_pixels}"
    );
}

#[test]
fn detail_report_returns_raw_object_without_expanding_resource_by_default() {
    let document = ChemcoreDocument::blank();
    let report = detail_report(
        "blank.ccjs",
        &document,
        &TargetSelector::Object("obj_editor_molecule".to_string()),
        DetailOptions {
            include_raw: true,
            include_resource: false,
        },
    )
    .unwrap();
    assert_eq!(report["ok"], true);
    assert_eq!(report["detail"]["id"], "obj_editor_molecule");
    assert_eq!(
        report["detail"]["references"]["resource"]["id"],
        "mol_editor"
    );
    assert_eq!(
        report["detail"]["raw"]["object"]["id"],
        "obj_editor_molecule"
    );
    assert!(report["detail"]["raw"].get("resource").is_none());
}

#[test]
fn detail_report_can_suppress_raw_or_include_molecule_fragment() {
    let document = ChemcoreDocument::blank();
    let summary = detail_report(
        "blank.ccjs",
        &document,
        &TargetSelector::Object("obj_editor_molecule".to_string()),
        DetailOptions {
            include_raw: false,
            include_resource: false,
        },
    )
    .unwrap();
    assert!(summary["detail"].get("raw").is_none());

    let molecule = detail_report(
        "blank.ccjs",
        &document,
        &TargetSelector::Molecule(0),
        DetailOptions {
            include_raw: true,
            include_resource: false,
        },
    )
    .unwrap();
    assert_eq!(molecule["detail"]["kind"], "molecule");
    assert_eq!(
        molecule["detail"]["raw"]["fragment"]["schema"],
        "chemcore.molecule.fragment2d"
    );
}
