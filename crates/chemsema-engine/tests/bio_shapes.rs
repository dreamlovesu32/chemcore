use chemsema_engine::{
    document_to_cdx, parse_cdx_document, BioDrawKind, BioShapeFillType, BioShapeKind,
    BioShapeLineType, Engine, PointerEvent, RenderPrimitive, Tool,
};
use serde_json::json;

const BIO_SHAPES: &[(BioDrawKind, BioShapeKind, &str)] = &[
    (
        BioDrawKind::OneSubstrateEnzyme,
        BioShapeKind::OneSubstrateEnzyme,
        "1SubstrateEnzyme",
    ),
    (
        BioDrawKind::TwoSubstrateEnzyme,
        BioShapeKind::TwoSubstrateEnzyme,
        "2SubstrateEnzyme",
    ),
    (BioDrawKind::Receptor, BioShapeKind::Receptor, "Receptor"),
    (
        BioDrawKind::GProteinAlpha,
        BioShapeKind::GProteinAlpha,
        "GProteinAlpha",
    ),
    (
        BioDrawKind::GProteinBeta,
        BioShapeKind::GProteinBeta,
        "GProteinBeta",
    ),
    (
        BioDrawKind::GProteinGamma,
        BioShapeKind::GProteinGamma,
        "GProteinGamma",
    ),
    (
        BioDrawKind::Immunoglobulin,
        BioShapeKind::Immunoglobulin,
        "Immunoglobin",
    ),
    (
        BioDrawKind::IonChannel,
        BioShapeKind::IonChannel,
        "IonChannel",
    ),
    (
        BioDrawKind::EndoplasmicReticulum,
        BioShapeKind::EndoplasmicReticulum,
        "EndoplasmicReticulum",
    ),
    (BioDrawKind::Golgi, BioShapeKind::Golgi, "Golgi"),
    (
        BioDrawKind::MembraneLine,
        BioShapeKind::MembraneLine,
        "MembraneLine",
    ),
    (
        BioDrawKind::MembraneArc,
        BioShapeKind::MembraneArc,
        "MembraneArc",
    ),
    (
        BioDrawKind::MembraneEllipse,
        BioShapeKind::MembraneEllipse,
        "MembraneEllipse",
    ),
    (
        BioDrawKind::MembraneMicelle,
        BioShapeKind::MembraneMicelle,
        "MembraneMicelle",
    ),
    (BioDrawKind::Dna, BioShapeKind::Dna, "DNA"),
    (
        BioDrawKind::HelixProtein,
        BioShapeKind::HelixProtein,
        "HelixProtein",
    ),
    (
        BioDrawKind::Mitochondrion,
        BioShapeKind::Mitochondrion,
        "Mitochondrion",
    ),
    (BioDrawKind::Cloud, BioShapeKind::Cloud, "Cloud"),
    (BioDrawKind::TRna, BioShapeKind::TRna, "tRNA"),
    (BioDrawKind::RibosomeA, BioShapeKind::RibosomeA, "RibosomeA"),
    (BioDrawKind::RibosomeB, BioShapeKind::RibosomeB, "RibosomeB"),
];

fn draw_bio_shape(kind: BioDrawKind) -> Engine {
    let mut engine = Engine::new();
    let mut tool = engine.state().tool.clone();
    tool.active_tool = Tool::BioDraw;
    tool.bio_draw_kind = kind;
    tool.bio_shape_fill_type = BioShapeFillType::Shaded;
    tool.bio_shape_line_type = BioShapeLineType::Dashed;
    tool.shape_color = "#336699".to_string();
    engine.set_tool_state(tool);
    engine.pointer_down(PointerEvent {
        x: 100.0,
        y: 120.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_move(PointerEvent {
        x: 172.0,
        y: 144.0,
        button: Some(0),
        alt_key: false,
    });
    engine.pointer_up(PointerEvent {
        x: 172.0,
        y: 144.0,
        button: Some(0),
        alt_key: false,
    });
    engine
}

fn assert_bio_handle_is_stationary(
    kind: BioDrawKind,
    expected_action: &str,
    normalized: impl Fn(&chemsema_engine::BioShapeData) -> (f64, f64),
) {
    let mut engine = draw_bio_shape(kind);
    let object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.payload.bio_shape.is_some())
        .expect("BioShape")
        .clone();
    let data = object.payload.bio_shape.as_ref().expect("BioShape data");
    let before = data.parameters.clone();
    let (u, v) = normalized(data);
    let local_x = data.center[0]
        + (data.major_axis_end[0] - data.center[0]) * u
        + (data.minor_axis_end[0] - data.center[0]) * v;
    let local_y = data.center[1]
        + (data.major_axis_end[1] - data.center[1]) * u
        + (data.minor_axis_end[1] - data.center[1]) * v;
    let scaled_x = local_x * object.transform.scale[0];
    let scaled_y = local_y * object.transform.scale[1];
    let angle = object.transform.rotate.to_radians();
    let handle = chemsema_engine::Point::new(
        object.transform.translate[0] + scaled_x * angle.cos() - scaled_y * angle.sin(),
        object.transform.translate[1] + scaled_x * angle.sin() + scaled_y * angle.cos(),
    );
    assert_eq!(engine.hover_shape_action_at_point(handle), expected_action);
    assert_eq!(engine.begin_hover_shape_edit(handle), expected_action);
    assert!(
        !engine.finish_hover_shape_edit(handle, false),
        "{expected_action} without pointer movement must remain a no-op"
    );
    assert_eq!(
        engine
            .state()
            .document
            .find_scene_object(&object.id)
            .expect("edited BioShape")
            .payload
            .bio_shape
            .as_ref()
            .expect("edited BioShape data")
            .parameters,
        before,
        "{expected_action} must not change its parameter without pointer movement"
    );
}

#[test]
fn every_official_bio_shape_is_typed_rendered_and_cdxml_roundtrippable() {
    for (draw_kind, shape_kind, cdxml_type) in BIO_SHAPES {
        let icon = Engine::bio_draw_tool_icon_svg(
            *draw_kind,
            BioShapeFillType::Shaded,
            BioShapeLineType::Solid,
        );
        assert!(
            icon.contains("cc-bio-draw-icon"),
            "{draw_kind:?} should have a kernel-rendered toolbar icon"
        );
        let engine = draw_bio_shape(*draw_kind);
        let object = engine
            .state()
            .document
            .objects
            .iter()
            .find(|object| object.payload.bio_shape.is_some())
            .unwrap_or_else(|| panic!("{draw_kind:?} should create a typed BioShape"));
        let data = object.payload.bio_shape.as_ref().expect("BioShape data");
        assert_eq!(data.kind, *shape_kind);
        assert_eq!(data.fill_type, BioShapeFillType::Shaded);
        assert_eq!(data.line_type, BioShapeLineType::Dashed);
        assert_eq!(data.color, "#336699");
        assert!(
            engine
                .render_list()
                .iter()
                .any(|primitive| primitive.object_id() == Some(object.id.as_str())),
            "{draw_kind:?} should emit render primitives"
        );

        let cdxml = engine.document_cdxml();
        assert!(
            cdxml.contains(&format!("BioShapeType=\"{cdxml_type}\"")),
            "{draw_kind:?} should use the official CDXML type: {cdxml}"
        );
        let mut reopened = Engine::new();
        reopened
            .load_cdxml_document(&cdxml)
            .unwrap_or_else(|error| panic!("{draw_kind:?} should reopen: {error}"));
        let reopened_data = reopened
            .state()
            .document
            .objects
            .iter()
            .find_map(|object| object.payload.bio_shape.as_ref())
            .unwrap_or_else(|| panic!("{draw_kind:?} should survive CDXML import"));
        assert_eq!(reopened_data.kind, *shape_kind);
        assert_eq!(reopened_data.fill_type, BioShapeFillType::Shaded);
        assert_eq!(reopened_data.line_type, BioShapeLineType::Dashed);
    }
    assert!(
        Engine::bio_draw_tool_icon_svg(
            BioDrawKind::PlasmidMap,
            BioShapeFillType::Shaded,
            BioShapeLineType::Solid,
        )
        .contains("cc-shape-icon"),
        "the native plasmid tool should also use a kernel-rendered icon"
    );
}

#[test]
fn add_bio_shape_command_accepts_browser_camel_case_fields() {
    let mut engine = Engine::new();
    let result = engine
        .execute_command_json(
            &json!({
                "type": "add-bio-shape",
                "kind": "dna",
                "fillType": "solid",
                "lineType": "wavy",
                "color": "#123456",
                "begin": {"x": 40.0, "y": 50.0},
                "end": {"x": 100.0, "y": 70.0}
            })
            .to_string(),
        )
        .expect("browser command should deserialize");
    let result: serde_json::Value = serde_json::from_str(&result).expect("command result JSON");
    assert_eq!(result["changed"], true);
    let data = engine
        .state()
        .document
        .objects
        .iter()
        .find_map(|object| object.payload.bio_shape.as_ref())
        .expect("command should create BioShape data");
    assert_eq!(data.kind, BioShapeKind::Dna);
    assert_eq!(data.fill_type, BioShapeFillType::Solid);
    assert_eq!(data.line_type, BioShapeLineType::Wavy);
}

#[test]
fn native_bio_shape_survives_cdx_round_trip() {
    let engine = draw_bio_shape(BioDrawKind::Dna);
    let cdx = document_to_cdx(&engine.state().document).expect("BioShape CDX writes");
    let reopened = parse_cdx_document(&cdx, Some("bio-shape-cdx")).expect("BioShape CDX reopens");
    let data = reopened
        .objects
        .iter()
        .find_map(|object| object.payload.bio_shape.as_ref())
        .expect("typed BioShape after CDX");
    assert_eq!(data.kind, BioShapeKind::Dna);
    assert_eq!(data.fill_type, BioShapeFillType::Shaded);
    assert_eq!(data.line_type, BioShapeLineType::Dashed);
}

#[test]
fn unsupported_bio_shape_values_are_rejected_instead_of_falling_back() {
    let cdxml = draw_bio_shape(BioDrawKind::Dna).document_cdxml();
    let unknown_kind = cdxml.replace("BioShapeType=\"DNA\"", "BioShapeType=\"MysteryShape\"");
    let kind_error = Engine::new()
        .load_cdxml_document(&unknown_kind)
        .expect_err("unknown BioShape type should be explicit");
    assert!(
        kind_error.contains("unsupported BioShapeType"),
        "{kind_error}"
    );

    let unknown_line = cdxml.replace("LineType=\"Dashed\"", "LineType=\"MysteryLine\"");
    let line_error = Engine::new()
        .load_cdxml_document(&unknown_line)
        .expect_err("unknown BioShape line type should be explicit");
    assert!(line_error.contains("unsupported LineType"), "{line_error}");
}

#[test]
fn bio_shape_properties_command_updates_typed_data_and_is_undoable() {
    let mut engine = draw_bio_shape(BioDrawKind::Dna);
    let object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.payload.bio_shape.is_some())
        .expect("BioShape object");
    let object_id = object.id.clone();
    let mut data = object.payload.bio_shape.clone().expect("BioShape data");
    let mut select_tool = engine.state().tool.clone();
    select_tool.active_tool = Tool::Select;
    engine.set_tool_state(select_tool);
    assert!(
        engine
            .context_menu_json(
                &json!({"kind": "shape", "objectId": object_id}).to_string(),
                false,
            )
            .contains("bio-shape-dialog"),
        "a selected BioShape should expose its kernel-defined properties dialog"
    );
    data.parameters.dna_wave_height = Some(22.5);
    data.line_width = 1.25;
    let result = engine
        .execute_command_json(
            &json!({
                "type": "set-bio-shape",
                "objectId": object_id,
                "data": data,
            })
            .to_string(),
        )
        .expect("BioShape properties command");
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&result).unwrap()["changed"],
        true
    );
    let changed = engine
        .state()
        .document
        .find_scene_object(&object_id)
        .unwrap()
        .payload
        .bio_shape
        .as_ref()
        .unwrap();
    assert_eq!(changed.parameters.dna_wave_height, Some(22.5));
    assert_eq!(changed.line_width, 1.25);
    assert!(engine.undo());
    assert_ne!(
        engine
            .state()
            .document
            .find_scene_object(&object_id)
            .unwrap()
            .payload
            .bio_shape
            .as_ref()
            .unwrap()
            .parameters
            .dna_wave_height,
        Some(22.5)
    );
}

#[test]
fn official_specialized_receptor_handle_edits_neck_width() {
    let mut engine = draw_bio_shape(BioDrawKind::Receptor);
    let object = engine
        .state()
        .document
        .objects
        .iter()
        .find(|object| object.payload.bio_shape.is_some())
        .unwrap()
        .clone();
    let data = object.payload.bio_shape.as_ref().unwrap();
    let world = |u: f64, v: f64| {
        let x = data.center[0]
            + (data.major_axis_end[0] - data.center[0]) * u
            + (data.minor_axis_end[0] - data.center[0]) * v;
        let y = data.center[1]
            + (data.major_axis_end[1] - data.center[1]) * u
            + (data.minor_axis_end[1] - data.center[1]) * v;
        let x = x * object.transform.scale[0];
        let y = y * object.transform.scale[1];
        let angle = object.transform.rotate.to_radians();
        chemsema_engine::Point::new(
            object.transform.translate[0] + x * angle.cos() - y * angle.sin(),
            object.transform.translate[1] + x * angle.sin() + y * angle.cos(),
        )
    };
    let handle = world(0.25, -0.48);
    assert_eq!(
        engine.hover_shape_action_at_point(handle),
        "bio-receptor-width"
    );
    assert_eq!(engine.begin_hover_shape_edit(handle), "bio-receptor-width");
    assert!(engine.finish_hover_shape_edit(world(0.42, -0.48), false));
    assert_eq!(
        engine
            .state()
            .document
            .find_scene_object(&object.id)
            .unwrap()
            .payload
            .bio_shape
            .as_ref()
            .unwrap()
            .parameters
            .neck_width,
        Some(42.0)
    );
}

#[test]
fn every_specialized_bio_handle_is_parameter_derived_and_does_not_jump() {
    let radii = |data: &chemsema_engine::BioShapeData| {
        let center = chemsema_engine::Point::new(data.center[0], data.center[1]);
        (
            center.distance(chemsema_engine::Point::new(
                data.major_axis_end[0],
                data.major_axis_end[1],
            )),
            center.distance(chemsema_engine::Point::new(
                data.minor_axis_end[0],
                data.minor_axis_end[1],
            )),
        )
    };
    assert_bio_handle_is_stationary(BioDrawKind::Receptor, "bio-receptor-width", |data| {
        let p = data.parameters.resolved_for(data.kind);
        (p.neck_width.expect("neck width") / 100.0, -0.48)
    });
    assert_bio_handle_is_stationary(
        BioDrawKind::GProteinGamma,
        "bio-gprotein-gamma-shape",
        |data| {
            let p = data.parameters.resolved_for(data.kind);
            (
                0.2,
                0.65 + p.gprotein_upper_height.expect("upper height") / 500.0,
            )
        },
    );
    for (action, normalized) in [
        (
            "bio-dna-height",
            Box::new(move |data: &chemsema_engine::BioShapeData| {
                let (_, minor) = radii(data);
                let p = data.parameters.resolved_for(data.kind);
                (
                    -0.82,
                    p.dna_wave_height.expect("wave height") / (minor * 2.0),
                )
            }) as Box<dyn Fn(&chemsema_engine::BioShapeData) -> (f64, f64)>,
        ),
        (
            "bio-dna-spacing",
            Box::new(move |data: &chemsema_engine::BioShapeData| {
                let (major, _) = radii(data);
                let p = data.parameters.resolved_for(data.kind);
                (
                    -p.dna_wave_length.expect("wave length") / (major * 2.0),
                    0.0,
                )
            }),
        ),
        (
            "bio-dna-strand-width",
            Box::new(move |data: &chemsema_engine::BioShapeData| {
                let (_, minor) = radii(data);
                let p = data.parameters.resolved_for(data.kind);
                (-0.82, p.dna_wave_width.expect("strand width") / minor)
            }),
        ),
        (
            "bio-dna-offset",
            Box::new(move |data: &chemsema_engine::BioShapeData| {
                let (_, minor) = radii(data);
                let p = data.parameters.resolved_for(data.kind);
                (-0.82, -p.dna_wave_offset.expect("strand offset") / minor)
            }),
        ),
    ] {
        assert_bio_handle_is_stationary(BioDrawKind::Dna, action, normalized);
    }
    for (action, normalized) in [
        (
            "bio-helix-height",
            Box::new(move |data: &chemsema_engine::BioShapeData| {
                let (_, minor) = radii(data);
                let p = data.parameters.resolved_for(data.kind);
                (
                    -0.82,
                    p.cylinder_height.expect("cylinder height") / (minor * 2.0),
                )
            }) as Box<dyn Fn(&chemsema_engine::BioShapeData) -> (f64, f64)>,
        ),
        (
            "bio-helix-strand-width",
            Box::new(move |data: &chemsema_engine::BioShapeData| {
                let (_, minor) = radii(data);
                let p = data.parameters.resolved_for(data.kind);
                (-0.82, -p.pipe_width.expect("strand width") / minor)
            }),
        ),
        (
            "bio-helix-cylinder-width",
            Box::new(move |data: &chemsema_engine::BioShapeData| {
                let (major, _) = radii(data);
                let p = data.parameters.resolved_for(data.kind);
                (-p.cylinder_width.expect("cylinder width") / major, 0.30)
            }),
        ),
        (
            "bio-helix-spacing",
            Box::new(move |data: &chemsema_engine::BioShapeData| {
                let (major, _) = radii(data);
                let p = data.parameters.resolved_for(data.kind);
                (
                    -p.cylinder_distance.expect("cylinder distance") / major,
                    0.0,
                )
            }),
        ),
    ] {
        assert_bio_handle_is_stationary(BioDrawKind::HelixProtein, action, normalized);
    }
    for kind in [
        BioDrawKind::MembraneLine,
        BioDrawKind::MembraneEllipse,
        BioDrawKind::MembraneMicelle,
    ] {
        assert_bio_handle_is_stationary(kind, "bio-membrane-unit-size", move |data| {
            let (_, minor) = radii(data);
            let p = data.parameters.resolved_for(data.kind);
            (
                0.0,
                p.membrane_element_size.expect("membrane element size") / (minor * 2.0),
            )
        });
    }
    assert_bio_handle_is_stationary(BioDrawKind::MembraneArc, "bio-membrane-arc-start", |data| {
        let p = data.parameters.resolved_for(data.kind);
        let angle = p.membrane_start_angle.expect("start angle").to_radians();
        (0.78 * angle.cos(), 0.78 * angle.sin())
    });
    assert_bio_handle_is_stationary(BioDrawKind::MembraneArc, "bio-membrane-arc-end", |data| {
        let p = data.parameters.resolved_for(data.kind);
        let angle = p.membrane_end_angle.expect("end angle").to_radians();
        (0.78 * angle.cos(), 0.78 * angle.sin())
    });
    assert_bio_handle_is_stationary(
        BioDrawKind::MembraneArc,
        "bio-membrane-unit-size",
        move |data| {
            let (_, minor) = radii(data);
            let p = data.parameters.resolved_for(data.kind);
            (
                0.0,
                p.membrane_element_size.expect("membrane element size") / (minor * 2.0),
            )
        },
    );
}

#[test]
fn omitted_parameters_use_the_same_authoritative_defaults_as_new_objects() {
    for kind in [
        BioDrawKind::OneSubstrateEnzyme,
        BioDrawKind::Receptor,
        BioDrawKind::GProteinGamma,
        BioDrawKind::Dna,
        BioDrawKind::HelixProtein,
        BioDrawKind::MembraneLine,
        BioDrawKind::MembraneArc,
        BioDrawKind::MembraneEllipse,
        BioDrawKind::MembraneMicelle,
    ] {
        let mut engine = draw_bio_shape(kind);
        let before = serde_json::to_value(engine.render_list()).expect("render list JSON");
        let object = engine
            .state()
            .document
            .objects
            .iter()
            .find(|object| object.payload.bio_shape.is_some())
            .expect("BioShape")
            .clone();
        let object_id = object.id.clone();
        let mut data = object.payload.bio_shape.expect("BioShape data");
        data.parameters = chemsema_engine::BioShapeParameters::default();
        engine
            .execute_command_json(
                &json!({
                    "type": "set-bio-shape",
                    "objectId": object_id,
                    "data": data,
                })
                .to_string(),
            )
            .expect("clear explicit BioShape parameters");
        let after = serde_json::to_value(engine.render_list()).expect("render list JSON");
        assert_eq!(
            after, before,
            "{kind:?} must resolve omitted fields through the same defaults used at creation"
        );
    }
}

#[test]
fn bio_shape_body_is_not_treated_as_an_unrelated_parameter_handle() {
    for kind in [BioDrawKind::GProteinAlpha, BioDrawKind::Dna] {
        let mut engine = draw_bio_shape(kind);
        let object = engine
            .state()
            .document
            .objects
            .iter()
            .find(|object| object.payload.bio_shape.is_some())
            .expect("BioShape")
            .clone();
        let data = object.payload.bio_shape.as_ref().expect("BioShape data");
        let local_x = data.center[0] * object.transform.scale[0];
        let local_y = data.center[1] * object.transform.scale[1];
        let angle = object.transform.rotate.to_radians();
        let center = chemsema_engine::Point::new(
            object.transform.translate[0] + local_x * angle.cos() - local_y * angle.sin(),
            object.transform.translate[1] + local_x * angle.sin() + local_y * angle.cos(),
        );
        assert_eq!(
            engine.hover_shape_action_at_point(center),
            "",
            "{kind:?} body must not impersonate the membrane unit-size handle"
        );
        let before = engine
            .state()
            .document
            .objects
            .iter()
            .filter(|object| object.payload.bio_shape.is_some())
            .count();
        engine.pointer_down(PointerEvent {
            x: center.x,
            y: center.y,
            button: Some(0),
            alt_key: false,
        });
        engine.pointer_up(PointerEvent {
            x: center.x,
            y: center.y,
            button: Some(0),
            alt_key: false,
        });
        let after = engine
            .state()
            .document
            .objects
            .iter()
            .filter(|object| object.payload.bio_shape.is_some())
            .count();
        assert_eq!(
            after,
            before + 1,
            "clicking a BioShape body with BioDraw active should create the selected tool"
        );
    }
}

#[test]
fn bio_draw_icons_use_a_square_kernel_fitted_viewbox() {
    for (draw_kind, _, _) in BIO_SHAPES {
        let icon = Engine::bio_draw_tool_icon_svg(
            *draw_kind,
            BioShapeFillType::Shaded,
            BioShapeLineType::Solid,
        );
        let view_box = icon
            .split("viewBox=\"")
            .nth(1)
            .and_then(|tail| tail.split('"').next())
            .expect("viewBox");
        let values: Vec<f64> = view_box
            .split_ascii_whitespace()
            .map(|value| value.parse().expect("numeric viewBox"))
            .collect();
        assert_eq!(values.len(), 4);
        assert!(
            (values[2] - values[3]).abs() < 0.001,
            "{draw_kind:?} icon must preserve a square frame"
        );
        assert!(icon.contains("currentColor"));
    }
}

#[test]
fn fixed_outline_bio_shapes_use_the_verified_chemdraw_cubic_templates() {
    for kind in [
        BioDrawKind::TwoSubstrateEnzyme,
        BioDrawKind::GProteinAlpha,
        BioDrawKind::GProteinBeta,
        BioDrawKind::Immunoglobulin,
        BioDrawKind::IonChannel,
        BioDrawKind::EndoplasmicReticulum,
        BioDrawKind::Golgi,
        BioDrawKind::Mitochondrion,
        BioDrawKind::Cloud,
        BioDrawKind::TRna,
        BioDrawKind::RibosomeA,
        BioDrawKind::RibosomeB,
    ] {
        let engine = draw_bio_shape(kind);
        let render_list = engine.render_list();
        let outline = render_list
            .iter()
            .find_map(|primitive| match primitive {
                RenderPrimitive::Path { d, .. } => Some(d),
                _ => None,
            })
            .unwrap_or_else(|| panic!("{kind:?} should render an explicit cubic outline"));
        assert!(
            outline.contains(" C "),
            "{kind:?} must retain ChemDraw cubic control points"
        );
    }
}

#[test]
fn mitochondrion_cristae_use_chemdraw_fixed_fill_and_line_width() {
    let engine = draw_bio_shape(BioDrawKind::Mitochondrion);
    let expected_line_width = engine
        .state()
        .document
        .objects
        .iter()
        .find_map(|object| object.payload.bio_shape.as_ref())
        .expect("mitochondrion BioShape")
        .line_width;
    let render_list = engine.render_list();
    assert!(
        render_list.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::FilledPath { fill, .. } if fill == "#d9d9d9"
        )),
        "the inner cristae must be the fixed ChemDraw gray instead of a transparent path"
    );
    assert!(
        render_list.iter().any(|primitive| matches!(
            primitive,
            RenderPrimitive::Path { stroke_width, .. }
                if (*stroke_width - expected_line_width).abs() < 0.000_001
        )),
        "the inner cristae outline must use LineWidth rather than the shaded outer contour width"
    );
}

#[test]
fn shaded_cubic_clip_uses_chemdraw_sampling_and_bounding_box_inset() {
    let engine = draw_bio_shape(BioDrawKind::Dna);
    let render_list = engine.render_list();
    let (outer, clip_path) = render_list
        .iter()
        .find_map(|primitive| match primitive {
            RenderPrimitive::FilledPath {
                points,
                clip_path_d: Some(clip_path),
                ..
            } => Some((points, clip_path)),
            _ => None,
        })
        .expect("shaded DNA layer with a clip");
    let clip_numbers = clip_path
        .split_ascii_whitespace()
        .filter_map(|token| token.parse::<f64>().ok())
        .collect::<Vec<_>>();
    let clip = clip_numbers
        .chunks_exact(2)
        .map(|pair| chemsema_engine::Point::new(pair[0], pair[1]))
        .collect::<Vec<_>>();
    assert_eq!(
        outer.len(),
        44,
        "each cubic uses ChemDraw's 21-part shade sampling, with straight runs collapsed"
    );
    assert_eq!(clip.len(), outer.len());

    let left = outer
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let top = outer
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let right = outer
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let bottom = outer
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let horizontal_scale = (right - left - 0.2) / (right - left);
    let vertical_scale = (bottom - top - 0.2) / (bottom - top);
    for (source, clipped) in outer.iter().zip(&clip) {
        let expected_x = left + 0.1 + (source.x - left) * horizontal_scale;
        let expected_y = top + 0.1 + (source.y - top) * vertical_scale;
        assert!((clipped.x - expected_x).abs() < 0.000_01);
        assert!((clipped.y - expected_y).abs() < 0.000_01);
    }
}

#[test]
fn properties_only_expose_parameters_that_chemdraw_actually_renders() {
    assert_eq!(
        BioShapeKind::OneSubstrateEnzyme.parameter_fields(),
        &["enzymeReceptorSize"]
    );
    assert!(BioShapeKind::TwoSubstrateEnzyme
        .parameter_fields()
        .is_empty());
    assert_eq!(BioShapeKind::Receptor.parameter_fields(), &["neckWidth"]);
    assert_eq!(
        BioShapeKind::GProteinGamma.parameter_fields(),
        &["gproteinUpperHeight"]
    );
    assert!(BioShapeKind::Immunoglobulin.parameter_fields().is_empty());
    assert!(BioShapeKind::Golgi.parameter_fields().is_empty());
    assert_eq!(
        BioShapeKind::MembraneArc.parameter_fields(),
        &[
            "membraneElementSize",
            "membraneStartAngle",
            "membraneEndAngle"
        ]
    );
}
