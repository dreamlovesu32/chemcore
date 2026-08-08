use super::session::{handle_session_request, session_help_json, session_ready_json};
use super::*;

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("chemsema-cli-test-{}-{}", std::process::id(), name))
}

fn write_engine_temp(engine: &Engine, name: &str) -> PathBuf {
    let path = temp_path(name);
    fs::write(&path, engine.document_json().expect("document json")).expect("write document");
    path
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("repo root")
        .to_path_buf()
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 0.000_001,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn bundle_for_object_writes_verified_artifacts_and_identity_map() {
    let mut engine = Engine::new();
    let added: Value = serde_json::from_str(
        &engine
            .execute_command_json(
                &json!({
                    "type": "add-text",
                    "position": { "x": 100.0, "y": 120.0 },
                    "text": "target",
                    "box": [0.0, 0.0, 40.0, 12.0]
                })
                .to_string(),
            )
            .expect("add text"),
    )
    .expect("result");
    engine
        .execute_command_json(
            &json!({
                "type": "add-text",
                "position": { "x": 112.0, "y": 122.0 },
                "text": "neighbor",
                "box": [0.0, 0.0, 42.0, 12.0]
            })
            .to_string(),
        )
        .expect("add neighbor");
    let target_id = added["created"]["objects"][0].as_str().unwrap().to_string();
    let input = write_engine_temp(&engine, "bundle-object-input.ccjs");
    let out_dir = temp_path("bundle-object");
    let _ = fs::remove_dir_all(&out_dir);
    let document = engine_document(&engine).expect("document");

    let manifest = bundle_document(
        &engine,
        &document,
        &BundleOptions {
            input: input.display().to_string(),
            target: TargetSelector::Object(target_id.clone()),
            out_dir: out_dir.clone(),
            context_radius: 8.0,
            capture_format: CaptureFormat::Svg,
            raster: RasterOptions::default(),
            subset_format: "ccjs".to_string(),
            pretty: true,
        },
    )
    .expect("bundle");

    assert_eq!(manifest["schema"], "chemsema.agent.bundle.v1");
    assert_eq!(manifest["integrity"]["allResourcesResolved"], true);
    assert_eq!(manifest["integrity"]["allStylesResolved"], true);
    for file in [
        "manifest.json",
        "target.json",
        "context.json",
        "editable-subset.ccjs",
        "capture.svg",
        "identity-map.json",
        "provenance.json",
    ] {
        assert!(out_dir.join(file).is_file(), "missing {file}");
    }
    assert_eq!(manifest["artifacts"]["provenance"], "provenance.json");
    assert_eq!(
        manifest["provenance"]["derivedFrom"]["selector"],
        format!("object:{target_id}")
    );
    let identity: Value =
        serde_json::from_str(&fs::read_to_string(out_dir.join("identity-map.json")).unwrap())
            .unwrap();
    assert!(identity["entries"].as_array().unwrap().iter().any(|entry| {
        entry["sourceSelector"] == format!("object:{target_id}")
            && entry["bundleSelector"] == format!("object:{target_id}")
            && entry["kind"] == "object"
            && entry["includedBecause"] == "editable-target-or-required-ancestor"
    }));
    assert_eq!(identity["selectorIdentityStable"], true);
    let provenance: Value =
        serde_json::from_str(&fs::read_to_string(out_dir.join("provenance.json")).unwrap())
            .unwrap();
    assert_eq!(provenance["schema"], "chemsema.agent.provenance.v1");
    assert_eq!(provenance["source"]["privacy"]["absolutePathStored"], false);
    assert_eq!(
        provenance["source"]["path"],
        input.file_name().and_then(|value| value.to_str()).unwrap()
    );
    assert_eq!(
        provenance["derivedFrom"]["selector"],
        format!("object:{target_id}")
    );
    assert_eq!(
        provenance["identityMap"]["entryCount"],
        identity["entries"].as_array().unwrap().len()
    );
    let subset: Value =
        serde_json::from_str(&fs::read_to_string(out_dir.join("editable-subset.ccjs")).unwrap())
            .unwrap();
    let object_ids = subset["entities"]["scene"]
        .as_array()
        .unwrap()
        .iter()
        .map(|object| object["id"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(object_ids, vec![target_id.as_str()]);
    let context: Value =
        serde_json::from_str(&fs::read_to_string(out_dir.join("context.json")).unwrap()).unwrap();
    assert!(context["context"]["objects"]
        .as_array()
        .unwrap()
        .iter()
        .any(
            |entry| entry["isTarget"] == false && entry["selectionBoxRelation"].as_str().is_some()
        ));

    let _ = fs::remove_file(input);
    let _ = fs::remove_dir_all(out_dir);
}

#[test]
fn bundle_supports_molecule_multi_object_and_group_targets() {
    let mut engine = Engine::new();
    engine
        .execute_command_json(
            &json!({
                "type": "add-bond",
                "begin": { "x": 20.0, "y": 20.0 },
                "end": { "x": 60.0, "y": 20.0 },
                "order": 1,
                "variant": "single"
            })
            .to_string(),
        )
        .expect("add bond");
    let first: Value = serde_json::from_str(
        &engine
            .execute_command_json(
                &json!({
                    "type": "add-text",
                    "position": { "x": 100.0, "y": 100.0 },
                    "text": "A",
                    "box": [0.0, 0.0, 10.0, 10.0]
                })
                .to_string(),
            )
            .expect("add first"),
    )
    .unwrap();
    let second: Value = serde_json::from_str(
        &engine
            .execute_command_json(
                &json!({
                    "type": "add-text",
                    "position": { "x": 130.0, "y": 100.0 },
                    "text": "B",
                    "box": [0.0, 0.0, 10.0, 10.0]
                })
                .to_string(),
            )
            .expect("add second"),
    )
    .unwrap();
    let first_id = first["created"]["objects"][0].as_str().unwrap().to_string();
    let second_id = second["created"]["objects"][0]
        .as_str()
        .unwrap()
        .to_string();
    let group: Value = serde_json::from_str(
        &engine
            .execute_command_json(
                &json!({
                    "type": "group-selection",
                    "object_ids": [first_id, second_id]
                })
                .to_string(),
            )
            .expect("group"),
    )
    .unwrap();
    let group_id = group["created"]["objects"][0].as_str().unwrap().to_string();
    let input = write_engine_temp(&engine, "bundle-targets-input.ccjs");
    let document = engine_document(&engine).expect("document");
    let cases = [
        ("molecule", TargetSelector::Molecule(0)),
        (
            "multi",
            TargetSelector::Selection(vec![
                TargetSelector::Object(first_id.clone()),
                TargetSelector::Object(second_id.clone()),
            ]),
        ),
        ("group", TargetSelector::Object(group_id)),
    ];
    for (name, target) in cases {
        let out_dir = temp_path(&format!("bundle-{name}"));
        let _ = fs::remove_dir_all(&out_dir);
        let manifest = bundle_document(
            &engine,
            &document,
            &BundleOptions {
                input: input.display().to_string(),
                target,
                out_dir: out_dir.clone(),
                context_radius: 4.0,
                capture_format: CaptureFormat::Svg,
                raster: RasterOptions::default(),
                subset_format: "ccjs".to_string(),
                pretty: false,
            },
        )
        .expect("bundle case");
        assert_eq!(manifest["ok"], true, "{name}");
        assert!(out_dir.join("editable-subset.ccjs").is_file());
        let _ = fs::remove_dir_all(out_dir);
    }
    let _ = fs::remove_file(input);
}

#[test]
fn bundle_fails_when_editable_subset_has_missing_references() {
    let mut document = ChemSemaDocument::blank();
    document.objects[0].style_ref = Some("missing_style".to_string());
    document.objects[0].payload.resource_ref = Some("missing_resource".to_string());
    document.resources.clear();
    let engine = Engine::new();
    let out_dir = temp_path("bundle-missing-reference");
    let error = bundle_document(
        &engine,
        &document,
        &BundleOptions {
            input: "missing-input.ccjs".to_string(),
            target: TargetSelector::Object("obj_editor_molecule".to_string()),
            out_dir,
            context_radius: 4.0,
            capture_format: CaptureFormat::Svg,
            raster: RasterOptions::default(),
            subset_format: "ccjs".to_string(),
            pretty: false,
        },
    )
    .unwrap_err();
    assert!(error.contains("unresolved references"));
}

#[test]
fn bundle_json_outputs_are_deterministic_across_runs() {
    let mut engine = Engine::new();
    engine
        .execute_command_json(
            &json!({
                "type": "add-text",
                "position": { "x": 100.0, "y": 120.0 },
                "text": "stable",
                "box": [0.0, 0.0, 30.0, 12.0]
            })
            .to_string(),
        )
        .expect("add text");
    let input = write_engine_temp(&engine, "bundle-deterministic-input.ccjs");
    let document = engine_document(&engine).expect("document");
    let mut manifests = Vec::new();
    for index in 0..2 {
        let out_dir = temp_path(&format!("bundle-deterministic-{index}"));
        let _ = fs::remove_dir_all(&out_dir);
        manifests.push(
            bundle_document(
                &engine,
                &document,
                &BundleOptions {
                    input: input.display().to_string(),
                    target: TargetSelector::Object("obj_text_1".to_string()),
                    out_dir: out_dir.clone(),
                    context_radius: 4.0,
                    capture_format: CaptureFormat::Svg,
                    raster: RasterOptions::default(),
                    subset_format: "ccjs".to_string(),
                    pretty: false,
                },
            )
            .expect("bundle"),
        );
        let _ = fs::remove_dir_all(out_dir);
    }
    assert_eq!(
        manifests[0]["artifactVerification"],
        manifests[1]["artifactVerification"]
    );
    assert_eq!(manifests[0]["target"], manifests[1]["target"]);
    assert_eq!(manifests[0]["editableScope"], manifests[1]["editableScope"]);
    let _ = fs::remove_file(input);
}

#[test]
fn document_diff_reports_structural_changes_by_selector() {
    let mut before = Engine::new();
    before
        .execute_command_json(
            &json!({
                "type": "add-bond",
                "begin": { "x": 20.0, "y": 20.0 },
                "end": { "x": 60.0, "y": 20.0 },
                "order": 1,
                "variant": "single"
            })
            .to_string(),
        )
        .expect("add bond");
    let mut after = Engine::new();
    after
        .load_document_json(&before.document_json().expect("before json"))
        .expect("load after");
    after
        .execute_command_json(
            &json!({
                "type": "replace-node-label",
                "node_id": "n_1",
                "label": "OMe"
            })
            .to_string(),
        )
        .expect("label");
    after
        .execute_command_json(
            &json!({
                "type": "apply-bond-style",
                "bondIds": ["b_3"],
                "style": "double-center"
            })
            .to_string(),
        )
        .expect("bond style");
    after
        .execute_command_json(
            &json!({
                "type": "move-targets",
                "targets": { "objects": ["obj_editor_molecule"] },
                "delta": { "dx": 5.0, "dy": 0.0 }
            })
            .to_string(),
        )
        .expect("move");
    let diff = diff::document_diff(
        &engine_document(&before).unwrap(),
        &engine_document(&after).unwrap(),
    )
    .unwrap();
    assert!(!diff.equal());
    assert!(diff.value["nodes"]["updated"]
        .as_array()
        .unwrap()
        .contains(&json!("node:n_1")));
    assert!(diff.value["bonds"]["updated"]
        .as_array()
        .unwrap()
        .contains(&json!("bond:b_3")));
    assert!(diff.value["objects"]["updated"]
        .as_array()
        .unwrap()
        .contains(&json!("object:obj_editor_molecule")));
    assert!(diff.value["changes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|change| {
            change["selector"] == "node:n_1" && change["path"].as_str().unwrap().contains("label")
        }));
}

#[test]
fn document_diff_reports_creation_deletion_and_unchanged_documents() {
    let before = ChemSemaDocument::blank();
    let mut after = ChemSemaDocument::blank();
    after.objects.push(SceneObject {
        id: "obj_extra".to_string(),
        object_type: "text".to_string(),
        name: "text".to_string(),
        visible: true,
        locked: false,
        z_index: 20,
        transform: chemsema_engine::Transform::identity(),
        style_ref: None,
        link_policy: Default::default(),
        meta: Value::Null,
        payload: chemsema_engine::ObjectPayload {
            resource_ref: None,
            bbox: Some([0.0, 0.0, 10.0, 10.0]),
            spectrum: None,
            geometry: None,
            constraint: None,
            table: None,
            stoichiometry_grid: None,
            gel_electrophoresis: None,
            plasmid_map: None,
            bio_shape: None,
            extra: BTreeMap::new(),
        },
        children: Vec::new(),
    });
    let created = diff::document_diff(&before, &after).unwrap();
    assert!(created.value["objects"]["created"]
        .as_array()
        .unwrap()
        .contains(&json!("object:obj_extra")));
    let deleted = diff::document_diff(&after, &before).unwrap();
    assert!(deleted.value["objects"]["deleted"]
        .as_array()
        .unwrap()
        .contains(&json!("object:obj_extra")));
    let unchanged = diff::document_diff(&before, &before).unwrap();
    assert!(unchanged.equal());
    assert!(unchanged.value["changes"].as_array().unwrap().is_empty());
}

#[test]
fn figure_cdxml_files_support_bundle_integration() {
    for file in ["figure1.cdxml", "figure2.cdxml"] {
        let path = repo_root().join(file);
        let engine = load_engine_from_file(&path.display().to_string()).expect("load figure");
        let document = engine_document(&engine).expect("document");
        let object = document
            .scene_objects()
            .into_iter()
            .find(|object| object.visible && object.object_type != "group")
            .expect("visible object");
        let out_dir = temp_path(&format!("bundle-{file}"));
        let _ = fs::remove_dir_all(&out_dir);
        let manifest = bundle_document(
            &engine,
            &document,
            &BundleOptions {
                input: path.display().to_string(),
                target: TargetSelector::Object(object.id.clone()),
                out_dir: out_dir.clone(),
                context_radius: 4.0,
                capture_format: CaptureFormat::Svg,
                raster: RasterOptions::default(),
                subset_format: "ccjs".to_string(),
                pretty: false,
            },
        )
        .expect("figure bundle");
        assert_eq!(manifest["ok"], true);
        assert_eq!(manifest["schema"], "chemsema.agent.bundle.v1");
        assert!(out_dir.join("editable-subset.ccjs").is_file());
        let _ = fs::remove_dir_all(out_dir);
    }
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
fn export_document_for_object_target_compacts_page() {
    let mut engine = Engine::new();
    let first: Value = serde_json::from_str(
        &engine
            .execute_command_json(
                &json!({
                    "type": "add-text",
                    "position": { "x": 100.0, "y": 120.0 },
                    "text": "A",
                    "box": [0.0, 0.0, 10.0, 10.0]
                })
                .to_string(),
            )
            .expect("add first text"),
    )
    .expect("first result");
    let first_id = first["created"]["objects"][0]
        .as_str()
        .expect("first id")
        .to_string();

    let second: Value = serde_json::from_str(
        &engine
            .execute_command_json(
                &json!({
                    "type": "add-text",
                    "position": { "x": 300.0, "y": 120.0 },
                    "text": "B",
                    "box": [0.0, 0.0, 10.0, 10.0]
                })
                .to_string(),
            )
            .expect("add second text"),
    )
    .expect("second result");
    let second_id = second["created"]["objects"][0]
        .as_str()
        .expect("second id")
        .to_string();

    let mut document = engine_document(&engine).expect("document");
    document.links.push(chemsema_engine::LinkRelation {
        id: "link_cross_object".to_string(),
        kind: "analysis-caption".to_string(),
        endpoints: vec![
            chemsema_engine::LinkEndpoint {
                entity_id: first_id.clone(),
                role: "source".to_string(),
            },
            chemsema_engine::LinkEndpoint {
                entity_id: second_id,
                role: "caption".to_string(),
            },
        ],
        data: Value::Null,
    });
    let target = TargetSelector::Object(first_id.clone());
    let bounds = target_bounds(&document, &target).expect("target bounds");
    let exported = export_document_for_target(&document, &target).expect("export object");

    assert_eq!(exported.objects.len(), 1);
    assert_eq!(exported.objects[0].id, first_id);
    assert!(
        exported.links.is_empty(),
        "relations whose endpoints are outside the editable subset must be pruned"
    );
    assert_close(exported.objects[0].transform.translate[0], 20.0);
    assert_close(exported.objects[0].transform.translate[1], 20.0);
    assert_close(exported.document.page.width, bounds[2] - bounds[0] + 40.0);
    assert_close(exported.document.page.height, bounds[3] - bounds[1] + 40.0);
    assert_eq!(
        exported.document.meta["export"]["selectionTarget"]["kind"],
        "object"
    );
}

#[test]
fn export_document_for_multi_object_selection_keeps_only_selected_objects() {
    let mut engine = Engine::new();
    let first: Value = serde_json::from_str(
        &engine
            .execute_command_json(
                &json!({
                    "type": "add-text",
                    "position": { "x": 100.0, "y": 120.0 },
                    "text": "A",
                    "box": [0.0, 0.0, 10.0, 10.0]
                })
                .to_string(),
            )
            .expect("add first text"),
    )
    .expect("first result");
    let second: Value = serde_json::from_str(
        &engine
            .execute_command_json(
                &json!({
                    "type": "add-text",
                    "position": { "x": 300.0, "y": 120.0 },
                    "text": "B",
                    "box": [0.0, 0.0, 10.0, 10.0]
                })
                .to_string(),
            )
            .expect("add second text"),
    )
    .expect("second result");
    engine
        .execute_command_json(
            &json!({
                "type": "add-text",
                "position": { "x": 500.0, "y": 120.0 },
                "text": "C",
                "box": [0.0, 0.0, 10.0, 10.0]
            })
            .to_string(),
        )
        .expect("add third text");
    let first_id = first["created"]["objects"][0]
        .as_str()
        .expect("first id")
        .to_string();
    let second_id = second["created"]["objects"][0]
        .as_str()
        .expect("second id")
        .to_string();

    let document = engine_document(&engine).expect("document");
    let target = TargetSelector::Selection(vec![
        TargetSelector::Object(first_id.clone()),
        TargetSelector::Object(second_id.clone()),
    ]);
    let bounds = target_bounds(&document, &target).expect("target bounds");
    let exported = export_document_for_target(&document, &target).expect("export selection");

    let ids = exported
        .objects
        .iter()
        .map(|object| object.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![first_id.as_str(), second_id.as_str()]);
    assert_close(exported.objects[0].transform.translate[0], 20.0);
    assert_close(exported.objects[1].transform.translate[0], 220.0);
    assert_close(exported.document.page.width, bounds[2] - bounds[0] + 40.0);
    assert_close(exported.document.page.height, bounds[3] - bounds[1] + 40.0);
    assert_eq!(
        exported.document.meta["export"]["selectionTarget"]["targetCount"],
        2
    );
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
            scale: 10.0,
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
    let document = ChemSemaDocument::blank();
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
    let document = ChemSemaDocument::blank();
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
        "chemsema.molecule.fragment2d"
    );
}

#[test]
fn session_reports_canonical_protocol_id() {
    let ready = session_ready_json(None);
    assert_eq!(ready["protocol"], crate::protocol::SESSION_PROTOCOL_VERSION);

    let help = session_help_json();
    assert_eq!(help["protocol"], crate::protocol::SESSION_PROTOCOL_VERSION);
    assert!(help["operations"]["execute"]["description"]
        .as_str()
        .expect("execute description")
        .contains("select-targets"));
}

#[test]
fn session_execute_selection_commands_drive_arrange() {
    let mut engine = Engine::new();
    let first: Value = serde_json::from_str(
        &engine
            .execute_command_json(
                &json!({
                    "type": "add-text",
                    "position": { "x": 10.0, "y": 10.0 },
                    "text": "A",
                    "box": [0.0, 0.0, 10.0, 10.0]
                })
                .to_string(),
            )
            .expect("add first text"),
    )
    .expect("first result");
    let second: Value = serde_json::from_str(
        &engine
            .execute_command_json(
                &json!({
                    "type": "add-text",
                    "position": { "x": 40.0, "y": 30.0 },
                    "text": "B",
                    "box": [0.0, 0.0, 10.0, 10.0]
                })
                .to_string(),
            )
            .expect("add second text"),
    )
    .expect("second result");
    let first_id = first["created"]["objects"][0].as_str().expect("first id");
    let second_id = second["created"]["objects"][0].as_str().expect("second id");

    let input = std::env::temp_dir().join(format!(
        "chemsema-cli-session-selection-input-{}.ccjs",
        std::process::id()
    ));
    let output = std::env::temp_dir().join(format!(
        "chemsema-cli-session-selection-output-{}.ccjs",
        std::process::id()
    ));
    fs::write(&input, engine.document_json().expect("document json")).expect("write input");

    let mut session = None;
    let (opened, exit) = handle_session_request(
        &mut session,
        json!({
            "id": 1,
            "op": "open",
            "input": input.to_string_lossy()
        }),
    );
    assert!(!exit);
    assert_eq!(opened["ok"], true);

    let (executed, exit) = handle_session_request(
        &mut session,
        json!({
            "id": 2,
            "op": "execute",
            "commands": [
                {
                    "type": "select-targets",
                    "targets": { "objects": [first_id, second_id] }
                },
                {
                    "type": "apply-selection-arrange",
                    "command": "align-left"
                }
            ]
        }),
    );
    assert!(!exit);
    assert_eq!(executed["ok"], true);
    assert_eq!(
        executed["result"]["results"][0]["commandType"],
        "select-targets"
    );
    assert_eq!(
        executed["result"]["results"][0]["result"]["output"]["counts"]["textObjects"],
        2
    );
    assert_eq!(
        executed["result"]["results"][1]["commandType"],
        "apply-selection-arrange"
    );
    assert_eq!(executed["result"]["results"][1]["changed"], true);

    let (saved, exit) = handle_session_request(
        &mut session,
        json!({
            "id": 3,
            "op": "save",
            "out": output.to_string_lossy()
        }),
    );
    assert!(!exit);
    assert_eq!(saved["ok"], true);

    let document: Value =
        serde_json::from_str(&fs::read_to_string(&output).expect("saved document"))
            .expect("saved json");
    let text_x = document["entities"]["scene"]
        .as_array()
        .expect("scene entities")
        .iter()
        .filter(|object| object["type"].as_str() == Some("text"))
        .map(|object| object["transform"]["translate"][0].as_f64().expect("x"))
        .collect::<Vec<_>>();
    assert_eq!(text_x, vec![10.0, 10.0]);

    let _ = fs::remove_file(input);
    let _ = fs::remove_file(output);
}

#[test]
fn session_execute_accepts_transaction_envelope() {
    let mut engine = Engine::new();
    engine
        .execute_command_json(
            &json!({
                "type": "add-bond",
                "begin": { "x": 20.0, "y": 20.0 },
                "end": { "x": 60.0, "y": 20.0 },
                "order": 1,
                "variant": "single"
            })
            .to_string(),
        )
        .expect("add bond");
    let input = std::env::temp_dir().join(format!(
        "chemsema-cli-session-transaction-input-{}.ccjs",
        std::process::id()
    ));
    fs::write(&input, engine.document_json().expect("document json")).expect("write input");

    let mut session = None;
    let (opened, exit) = handle_session_request(
        &mut session,
        json!({
            "id": 1,
            "op": "open",
            "input": input.to_string_lossy()
        }),
    );
    assert!(!exit);
    assert_eq!(opened["ok"], true);
    let before_revision = opened["result"]["revision"].as_u64().expect("revision");

    let (executed, exit) = handle_session_request(
        &mut session,
        json!({
            "id": 2,
            "op": "execute",
            "schema": "chemsema.command-transaction.v1",
            "preconditions": {
                "expectedRevision": before_revision,
                "requiredSelectors": ["object:obj_editor_molecule", "node:n_1"]
            },
            "scope": {
                "editableTargets": ["object:obj_editor_molecule"],
                "includeReferencedResources": true,
                "allowCreate": false,
                "allowDelete": false,
                "forbidChangesOutsideScope": true
            },
            "commands": [
                { "type": "replace-node-label", "node_id": "n_1", "label": "OMe" }
            ],
            "postconditions": [
                { "type": "document-valid" },
                { "type": "no-unexpected-changes" }
            ]
        }),
    );
    assert!(!exit);
    assert_eq!(executed["ok"], true);
    assert_eq!(executed["result"]["transaction"]["applied"], true);
    assert_eq!(
        executed["result"]["diff"]["nodes"]["updated"],
        json!(["node:n_1"])
    );
    assert_eq!(executed["result"]["scope"]["unexpectedChanges"], json!([]));

    let _ = fs::remove_file(input);
}
