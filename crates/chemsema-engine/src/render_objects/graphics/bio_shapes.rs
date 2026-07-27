use super::*;
use crate::{BioShapeData, BioShapeFillType, BioShapeKind, BioShapeLineType};
use std::f64::consts::TAU;

const BIO_SHAPE_CIRCLE_SHADED_REMAIN_RATIO: f64 = 0.173_016_519_552_486_72;

pub(crate) fn render_bio_shape_object(out: &mut Vec<RenderPrimitive>, object: &SceneObject) {
    let Some(data) = object.payload.bio_shape.as_ref() else {
        return;
    };
    let geometry = BioGeometry::new(object, data);
    match data.kind {
        BioShapeKind::OneSubstrateEnzyme => render_one_substrate_enzyme(out, &geometry),
        BioShapeKind::TwoSubstrateEnzyme => {
            render_cubic_template(out, &geometry, &TWO_SUBSTRATE_ENZYME_TEMPLATE)
        }
        BioShapeKind::Receptor => render_receptor(out, &geometry),
        BioShapeKind::GProteinAlpha => {
            render_cubic_template(out, &geometry, &G_PROTEIN_ALPHA_TEMPLATE)
        }
        BioShapeKind::GProteinBeta => {
            render_cubic_template(out, &geometry, &G_PROTEIN_BETA_TEMPLATE)
        }
        BioShapeKind::GProteinGamma => render_gprotein_gamma(out, &geometry),
        BioShapeKind::Immunoglobulin => render_immunoglobulin(out, &geometry),
        BioShapeKind::IonChannel => render_cubic_templates(out, &geometry, &ION_CHANNEL_TEMPLATES),
        BioShapeKind::EndoplasmicReticulum => {
            render_cubic_templates(out, &geometry, &ENDOPLASMIC_RETICULUM_TEMPLATES)
        }
        BioShapeKind::Golgi => render_cubic_templates(out, &geometry, &GOLGI_TEMPLATES),
        BioShapeKind::MembraneLine => render_membrane_line(out, &geometry),
        BioShapeKind::MembraneArc => render_membrane_arc(out, &geometry),
        BioShapeKind::MembraneEllipse => render_membrane_ellipse(out, &geometry),
        BioShapeKind::MembraneMicelle => render_membrane_micelle(out, &geometry),
        BioShapeKind::Dna => render_dna(out, &geometry),
        BioShapeKind::HelixProtein => render_helix_protein(out, &geometry),
        BioShapeKind::Mitochondrion => render_mitochondrion(out, &geometry),
        BioShapeKind::Cloud => render_cubic_template(out, &geometry, &CLOUD_TEMPLATE),
        BioShapeKind::TRna => render_trna_templates(out, &geometry),
        BioShapeKind::RibosomeA => render_cubic_template(out, &geometry, &RIBOSOME_A_TEMPLATE),
        BioShapeKind::RibosomeB => render_cubic_template(out, &geometry, &RIBOSOME_B_TEMPLATE),
    }
}

#[derive(Clone, Copy)]
struct CubicSegment {
    control_1: (f64, f64),
    control_2: (f64, f64),
    end: (f64, f64),
}

struct CubicTemplate {
    start: (f64, f64),
    segments: &'static [CubicSegment],
}

const fn cubic(control_1: (f64, f64), control_2: (f64, f64), end: (f64, f64)) -> CubicSegment {
    CubicSegment {
        control_1,
        control_2,
        end,
    }
}

const G_PROTEIN_ALPHA_SEGMENTS: [CubicSegment; 4] = [
    cubic((-1.0, 0.636_362), (-1.0, -1.000_012), (-1.0, -1.000_012)),
    cubic(
        (0.000_025, -1.000_012),
        (1.000_05, -0.181_825),
        (1.000_05, 0.636_362),
    ),
    cubic(
        (1.000_05, 1.454_55),
        (0.000_025, 0.636_362),
        (-1.0, 0.636_362),
    ),
    cubic((-1.0, 0.636_362), (-1.0, 0.636_362), (-1.0, 0.636_362)),
];
const G_PROTEIN_ALPHA_TEMPLATE: CubicTemplate = CubicTemplate {
    start: (-1.0, 0.636_362),
    segments: &G_PROTEIN_ALPHA_SEGMENTS,
};

const G_PROTEIN_BETA_SEGMENTS: [CubicSegment; 4] = [
    cubic((1.0, 0.636_362), (1.0, -1.000_012), (1.0, -1.000_012)),
    cubic(
        (-0.000_025, -1.000_012),
        (-1.000_05, -0.181_825),
        (-1.000_05, 0.636_362),
    ),
    cubic(
        (-1.000_05, 1.454_55),
        (-0.000_025, 0.636_362),
        (1.0, 0.636_362),
    ),
    cubic((1.0, 0.636_362), (1.0, 0.636_362), (1.0, 0.636_362)),
];
const G_PROTEIN_BETA_TEMPLATE: CubicTemplate = CubicTemplate {
    start: (1.0, 0.636_362),
    segments: &G_PROTEIN_BETA_SEGMENTS,
};

const ION_CHANNEL_RIGHT_SEGMENTS: [CubicSegment; 6] = [
    cubic((0.375, -1.0), (0.25, -0.5), (0.25, 0.0)),
    cubic((0.25, 0.5), (0.500_008, 1.0), (0.750_008, 1.0)),
    cubic((0.875_008, 1.0), (1.000_008, 0.5), (1.000_008, 0.0)),
    cubic((1.000_008, -0.25), (0.750_008, -0.5), (0.500_008, -0.5)),
    cubic((0.500_008, -0.75), (0.437_5, -1.0), (0.375, -1.0)),
    cubic((0.375, -1.0), (0.375, -1.0), (0.375, -1.0)),
];
const ION_CHANNEL_LEFT_SEGMENTS: [CubicSegment; 6] = [
    cubic((-0.375, -1.0), (-0.25, -0.5), (-0.25, 0.0)),
    cubic((-0.25, 0.5), (-0.500_008, 1.0), (-0.750_008, 1.0)),
    cubic((-0.875_008, 1.0), (-1.000_008, 0.5), (-1.000_008, 0.0)),
    cubic((-1.000_008, -0.25), (-0.750_008, -0.5), (-0.500_008, -0.5)),
    cubic((-0.500_008, -0.75), (-0.437_5, -1.0), (-0.375, -1.0)),
    cubic((-0.375, -1.0), (-0.375, -1.0), (-0.375, -1.0)),
];
const ION_CHANNEL_RIGHT_TEMPLATE: CubicTemplate = CubicTemplate {
    start: (0.375, -1.0),
    segments: &ION_CHANNEL_RIGHT_SEGMENTS,
};
const ION_CHANNEL_LEFT_TEMPLATE: CubicTemplate = CubicTemplate {
    start: (-0.375, -1.0),
    segments: &ION_CHANNEL_LEFT_SEGMENTS,
};
const ION_CHANNEL_TEMPLATES: [&CubicTemplate; 2] =
    [&ION_CHANNEL_RIGHT_TEMPLATE, &ION_CHANNEL_LEFT_TEMPLATE];

const IMMUNOGLOBULIN_MAIN_SEGMENTS: [CubicSegment; 10] = [
    cubic(
        (0.097_908, 0.561_325),
        (0.097_908, -0.28),
        (0.097_908, -0.28),
    ),
    cubic(
        (0.097_908, -0.28),
        (0.095_608, -0.506_062),
        (0.210_058, -0.592_775),
    ),
    cubic(
        (0.324_4, -0.679_475),
        (0.662_833, -0.919_738),
        (0.742_567, -0.973_775),
    ),
    cubic(
        (0.851_075, -1.047_438),
        (0.917_567, -0.952_063),
        (0.804_167, -0.876_325),
    ),
    cubic(
        (0.724_117, -0.822_762),
        (0.394_025, -0.582_838),
        (0.311_683, -0.501_788),
    ),
    cubic(
        (0.232_258, -0.423_75),
        (0.225_692, -0.290_05),
        (0.225_692, -0.107_862),
    ),
    cubic(
        (0.225_692, 0.074_212),
        (0.237_467, 0.708_763),
        (0.237_467, 0.873_512),
    ),
    cubic(
        (0.237_467, 1.038_263),
        (0.106_558, 1.044_038),
        (0.096_758, 0.879_287),
    ),
    cubic(
        (0.086_858, 0.714_525),
        (0.097_908, 0.856_075),
        (0.097_908, 0.708_763),
    ),
    cubic(
        (0.097_908, 0.708_763),
        (0.097_908, 0.708_763),
        (0.097_908, 0.708_763),
    ),
];
const IMMUNOGLOBULIN_MAIN_TEMPLATE: CubicTemplate = CubicTemplate {
    start: (0.097_908, 0.708_763),
    segments: &IMMUNOGLOBULIN_MAIN_SEGMENTS,
};
const IMMUNOGLOBULIN_ARM_SEGMENTS: [CubicSegment; 5] = [
    cubic(
        (0.545_258, -0.539_888),
        (0.762_475, -0.704_65),
        (0.868_683, -0.774_038),
    ),
    cubic(
        (1.006_167, -0.864_088),
        (1.052_342, -0.780_963),
        (0.927_267, -0.681_438),
    ),
    cubic(
        (0.836_792, -0.609_4),
        (0.596_125, -0.423_975),
        (0.505_133, -0.354_588),
    ),
    cubic(
        (0.414_242, -0.285_2),
        (0.343_267, -0.366_475),
        (0.444_258, -0.453_187),
    ),
    cubic(
        (0.444_258, -0.453_187),
        (0.444_258, -0.453_187),
        (0.444_258, -0.453_187),
    ),
];
const IMMUNOGLOBULIN_ARM_TEMPLATE: CubicTemplate = CubicTemplate {
    start: (0.444_258, -0.453_187),
    segments: &IMMUNOGLOBULIN_ARM_SEGMENTS,
};

const TWO_SUBSTRATE_ENZYME_SEGMENTS: [CubicSegment; 13] = [
    cubic(
        (0.428_575, 0.166_662),
        (0.523_817, 0.333_338),
        (0.619_05, 0.333_338),
    ),
    cubic(
        (0.619_05, 0.666_675),
        (0.190_475, 1.000_013),
        (-0.238_1, 1.000_013),
    ),
    cubic((-0.619_05, 1.000_013), (-1.000_008, 0.5), (-1.000_008, 0.0)),
    cubic(
        (-1.000_008, -0.2),
        (-0.880_958, -0.400_013),
        (-0.761_908, -0.400_013),
    ),
    cubic(
        (-0.761_908, -0.400_013),
        (-0.523_817, -0.400_013),
        (-0.523_817, -0.400_013),
    ),
    cubic(
        (-0.523_817, -0.400_013),
        (-0.523_817, -0.733_35),
        (-0.523_817, -0.733_35),
    ),
    cubic(
        (-0.523_817, -0.733_35),
        (-0.619_05, -0.733_35),
        (-0.619_05, -0.733_35),
    ),
    cubic(
        (-0.619_05, -0.866_687),
        (-0.095_242, -1.000_012),
        (0.428_575, -1.000_012),
    ),
    cubic(
        (0.714_292, -1.000_012),
        (1.000_008, -0.833_35),
        (1.000_008, -0.666_675),
    ),
    cubic(
        (1.000_008, -0.500_013),
        (0.904_767, -0.333_338),
        (0.809_533, -0.333_338),
    ),
    cubic(
        (0.809_533, -0.333_338),
        (0.619_05, -0.333_338),
        (0.619_05, -0.333_338),
    ),
    cubic(
        (0.523_817, -0.333_338),
        (0.428_575, -0.166_675),
        (0.428_575, 0.0),
    ),
    cubic((0.428_575, 0.0), (0.428_575, 0.0), (0.428_575, 0.0)),
];
const TWO_SUBSTRATE_ENZYME_TEMPLATE: CubicTemplate = CubicTemplate {
    start: (0.428_575, 0.0),
    segments: &TWO_SUBSTRATE_ENZYME_SEGMENTS,
};

const CLOUD_SEGMENTS: [CubicSegment; 15] = [
    cubic(
        (-0.994_275, 0.328_75),
        (-0.795_125, 0.500_062),
        (-0.621_583, 0.509_325),
    ),
    cubic(
        (-0.445_275, 0.518_7),
        (-0.359_85, 0.416_725),
        (-0.359_85, 0.416_725),
    ),
    cubic(
        (-0.359_85, 0.416_725),
        (-0.388_3, 0.990_837),
        (-0.038_367, 1.000_1),
    ),
    cubic(
        (0.294_458, 1.008_925),
        (0.285_967, 0.588_025),
        (0.314_417, 0.588_025),
    ),
    cubic(
        (0.314_417, 0.588_025),
        (0.337_175, 0.768_413),
        (0.550_55, 0.759_337),
    ),
    cubic(
        (0.877_717, 0.745_45),
        (0.775_3, 0.175_962),
        (0.775_3, 0.175_962),
    ),
    cubic(
        (0.775_3, 0.175_962),
        (1.000_05, -0.027_763),
        (1.000_05, -0.356_488),
    ),
    cubic(
        (1.000_05, -0.685_225),
        (0.766_95, -0.947_525),
        (0.610_292, -0.953_762),
    ),
    cubic(
        (0.377, -0.963_025),
        (0.217_683, -0.726_888),
        (0.217_683, -0.726_888),
    ),
    cubic(
        (0.217_683, -0.726_888),
        (0.155_092, -1.000_062),
        (0.024_225, -1.000_062),
    ),
    cubic(
        (-0.106_642, -1.000_062),
        (-0.177_767, -0.763_938),
        (-0.177_767, -0.763_938),
    ),
    cubic(
        (-0.177_767, -0.763_938),
        (-0.257_35, -0.939_875),
        (-0.470_8, -0.939_875),
    ),
    cubic(
        (-0.758_142, -0.939_875),
        (-0.743_917, -0.453_725),
        (-0.743_917, -0.453_725),
    ),
    cubic(
        (-0.743_917, -0.453_725),
        (-1.003_458, -0.263_887),
        (-0.999_967, 0.032_425),
    ),
    cubic(
        (-0.999_967, 0.032_425),
        (-0.999_967, 0.032_425),
        (-0.999_967, 0.032_425),
    ),
];
const CLOUD_TEMPLATE: CubicTemplate = CubicTemplate {
    start: (-0.999_967, 0.032_425),
    segments: &CLOUD_SEGMENTS,
};

const RIBOSOME_A_SEGMENTS: [CubicSegment; 5] = [
    cubic(
        (-1.664_338, -1.545_079),
        (1.699_708, -1.630_746),
        (0.866_242, 0.748_913),
    ),
    cubic(
        (0.578_45, 1.424_362),
        (0.449_9, 0.330_088),
        (0.127_883, 0.865_512),
    ),
    cubic(
        (-0.160_817, 1.345_463),
        (-0.218_067, 0.349_488),
        (-0.530_717, 0.828_9),
    ),
    cubic(
        (-0.793_4, 1.231_612),
        (-0.881_225, 0.778_562),
        (-0.881_225, 0.778_562),
    ),
    cubic(
        (-0.881_225, 0.778_562),
        (-0.881_225, 0.778_562),
        (-0.881_225, 0.778_562),
    ),
];
const RIBOSOME_A_TEMPLATE: CubicTemplate = CubicTemplate {
    start: (-0.881_225, 0.778_562),
    segments: &RIBOSOME_A_SEGMENTS,
};

const RIBOSOME_B_SEGMENTS: [CubicSegment; 9] = [
    cubic(
        (-1.026_917, -1.139_175),
        (-0.899_658, 0.047_038),
        (-0.646_767, -0.594_938),
    ),
    cubic(
        (-0.387_233, -1.253_9),
        (-0.344_275, -0.991_163),
        (-0.176_275, -0.639_2),
    ),
    cubic(
        (-0.015_65, -0.302_812),
        (0.015_5, -0.311_662),
        (0.165_192, -0.648_05),
    ),
    cubic(
        (0.312_825, -0.979_838),
        (0.322_275, -1.205_388),
        (0.646_767, -0.577_225),
    ),
    cubic(
        (0.914_417, -0.059_187),
        (1.037_983, -1.360_488),
        (0.99, -0.453_3),
    ),
    cubic(
        (0.949_258, 0.316_5),
        (0.828_2, 0.625_263),
        (0.462_225, 0.843_737),
    ),
    cubic(
        (0.098_758, 1.060_45),
        (-0.198_417, 1.051_6),
        (-0.473_3, 0.821_438),
    ),
    cubic(
        (-0.730_625, 0.606_15),
        (-0.962_692, 0.367_138),
        (-0.995_617, -0.404_437),
    ),
    cubic(
        (-0.995_617, -0.404_437),
        (-0.995_617, -0.404_437),
        (-0.995_617, -0.404_437),
    ),
];
const RIBOSOME_B_TEMPLATE: CubicTemplate = CubicTemplate {
    start: (-0.995_617, -0.404_437),
    segments: &RIBOSOME_B_SEGMENTS,
};

include!("bio_shape_templates.generated.rs");

struct BioGeometry<'a> {
    object: &'a SceneObject,
    data: &'a BioShapeData,
    major: Vector,
    minor: Vector,
}

impl<'a> BioGeometry<'a> {
    fn new(object: &'a SceneObject, data: &'a BioShapeData) -> Self {
        Self {
            object,
            data,
            major: Vector::new(
                data.major_axis_end[0] - data.center[0],
                data.major_axis_end[1] - data.center[1],
            ),
            minor: Vector::new(
                data.minor_axis_end[0] - data.center[0],
                data.minor_axis_end[1] - data.center[1],
            ),
        }
    }

    fn point(&self, u: f64, v: f64) -> Point {
        let local = Point::new(
            self.data.center[0] + self.major.x * u + self.minor.x * v,
            self.data.center[1] + self.major.y * u + self.minor.y * v,
        );
        let scaled = Point::new(
            local.x * self.object.transform.scale[0],
            local.y * self.object.transform.scale[1],
        );
        let angle = self.object.transform.rotate.to_radians();
        Point::new(
            self.object.transform.translate[0] + scaled.x * angle.cos() - scaled.y * angle.sin(),
            self.object.transform.translate[1] + scaled.x * angle.sin() + scaled.y * angle.cos(),
        )
    }

    fn object_id(&self) -> Option<String> {
        Some(self.object.id.clone())
    }

    fn stroke_width(&self) -> f64 {
        match self.data.line_type {
            BioShapeLineType::Bold => self.data.bold_width,
            _ => self.data.line_width,
        }
        .max(0.05)
    }

    fn cubic_outline_width(&self) -> f64 {
        if matches!(self.data.fill_type, BioShapeFillType::Shaded) {
            0.05
        } else {
            self.data.margin_width.max(0.05)
        }
    }

    fn dash_array(&self) -> Vec<f64> {
        match self.data.line_type {
            BioShapeLineType::Dashed => vec![self.data.hash_spacing.max(1.0)],
            _ => Vec::new(),
        }
    }

    fn fill(&self) -> Option<String> {
        match self.data.fill_type {
            BioShapeFillType::Unspecified | BioShapeFillType::None => None,
            BioShapeFillType::Solid => Some(self.data.color.clone()),
            BioShapeFillType::Shaded => Some(blend_with_white(&self.data.color, 0.58)),
        }
    }

    fn push_line(&self, out: &mut Vec<RenderPrimitive>, from: Point, to: Point, width: f64) {
        out.push(RenderPrimitive::Line {
            role: RenderRole::DocumentGraphic,
            object_id: self.object_id(),
            bond_id: None,
            from,
            to,
            stroke: self.data.color.clone(),
            stroke_width: width,
            dash_array: self.dash_array(),
        });
    }

    fn push_circle(&self, out: &mut Vec<RenderPrimitive>, center: Point, radius: f64) {
        if matches!(self.data.fill_type, BioShapeFillType::Shaded) {
            let max_index = (CIRCLE_SHADED_LEVELS.len() - 1) as f64;
            for (index, level) in CIRCLE_SHADED_LEVELS.iter().enumerate() {
                let t = index as f64 / max_index;
                let layer_radius =
                    radius * (1.0 - (1.0 - BIO_SHAPE_CIRCLE_SHADED_REMAIN_RATIO) * t);
                let layer_center = center.translated(Vector::new(
                    -CIRCLE_SHADED_CENTER_SHIFT_RATIO * radius * t,
                    -CIRCLE_SHADED_CENTER_SHIFT_RATIO * radius * t,
                ));
                super::shapes::push_shape_ellipse_fill(
                    out,
                    self.object.id.as_str(),
                    layer_center,
                    layer_radius,
                    layer_radius,
                    0.0,
                    true,
                    super::shapes::shaded_level_color(&self.data.color, level, t),
                );
            }
            out.push(RenderPrimitive::Circle {
                role: RenderRole::DocumentGraphic,
                object_id: self.object_id(),
                node_id: None,
                center,
                radius,
                fill: "none".to_string(),
                stroke: self.data.color.clone(),
                stroke_width: self.stroke_width(),
            });
            return;
        }
        out.push(RenderPrimitive::Circle {
            role: RenderRole::DocumentGraphic,
            object_id: self.object_id(),
            node_id: None,
            center,
            radius,
            fill: self.fill().unwrap_or_else(|| "#ffffff".to_string()),
            stroke: self.data.color.clone(),
            stroke_width: self.stroke_width(),
        });
    }

    fn push_cubic_template(&self, out: &mut Vec<RenderPrimitive>, template: &CubicTemplate) {
        self.push_cubic_template_indexed(out, template, 0);
    }

    fn push_cubic_template_indexed(
        &self,
        out: &mut Vec<RenderPrimitive>,
        template: &CubicTemplate,
        index: usize,
    ) {
        self.push_cubic_segments_with_fill_and_anchor(
            out,
            template.start,
            template.segments,
            1.0,
            true,
            self.shading_anchor_fraction(index),
        );
    }

    fn push_scaled_cubic_template(
        &self,
        out: &mut Vec<RenderPrimitive>,
        template: &CubicTemplate,
        u_scale: f64,
        index: usize,
    ) {
        self.push_cubic_segments_with_fill_and_anchor(
            out,
            template.start,
            template.segments,
            u_scale,
            true,
            self.shading_anchor_fraction(index),
        );
    }

    fn push_cubic_segments(
        &self,
        out: &mut Vec<RenderPrimitive>,
        start_local: (f64, f64),
        segments: &[CubicSegment],
        u_scale: f64,
    ) {
        self.push_cubic_segments_with_fill_and_anchor(
            out,
            start_local,
            segments,
            u_scale,
            true,
            self.shading_anchor_fraction(0),
        );
    }

    fn push_cubic_segments_with_fill_and_anchor(
        &self,
        out: &mut Vec<RenderPrimitive>,
        start_local: (f64, f64),
        segments: &[CubicSegment],
        u_scale: f64,
        fill_enabled: bool,
        shade_anchor_fraction: f64,
    ) {
        self.push_cubic_segments_styled(
            out,
            start_local,
            segments,
            u_scale,
            fill_enabled,
            self.data.color.clone(),
            self.cubic_outline_width(),
            shade_anchor_fraction,
            None,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn push_cubic_segments_styled(
        &self,
        out: &mut Vec<RenderPrimitive>,
        start_local: (f64, f64),
        segments: &[CubicSegment],
        u_scale: f64,
        fill_enabled: bool,
        stroke: String,
        stroke_width: f64,
        shade_anchor_fraction: f64,
        none_fill: Option<&str>,
    ) {
        let (d, points, world_segments, closed) =
            self.cubic_world_path(start_local, segments, u_scale);
        let start = points[0];
        let uses_expanded_stroke_fill = matches!(self.data.kind, BioShapeKind::TRna);
        if fill_enabled && (closed || uses_expanded_stroke_fill) {
            match self.data.fill_type {
                BioShapeFillType::Shaded => {
                    push_shaded_cubic_layers(
                        out,
                        self,
                        start,
                        &world_segments,
                        shade_anchor_fraction,
                    );
                }
                BioShapeFillType::Solid => {
                    self.push_cubic_fill(out, &d, points.clone(), &self.data.color);
                }
                BioShapeFillType::Unspecified | BioShapeFillType::None => {
                    if let Some(fill) = none_fill {
                        self.push_cubic_fill(out, &d, points.clone(), fill);
                    }
                }
            }
        }
        out.push(RenderPrimitive::Path {
            role: RenderRole::DocumentGraphic,
            object_id: self.object_id(),
            bond_id: None,
            d,
            points,
            stroke,
            stroke_width,
            dash_array: self.dash_array(),
            line_cap: None,
            line_join: None,
            rotate: 0.0,
            rotate_center: None,
        });
    }

    fn cubic_world_path(
        &self,
        start_local: (f64, f64),
        segments: &[CubicSegment],
        u_scale: f64,
    ) -> (String, Vec<Point>, Vec<[Point; 3]>, bool) {
        let start = self.point(start_local.0 * u_scale, start_local.1);
        let mut d = format!("M {:.5} {:.5}", start.x, start.y);
        let mut points = vec![start];
        let mut world_segments = Vec::with_capacity(segments.len());
        for segment in segments {
            let control_1 = self.point(segment.control_1.0 * u_scale, segment.control_1.1);
            let control_2 = self.point(segment.control_2.0 * u_scale, segment.control_2.1);
            let end = self.point(segment.end.0 * u_scale, segment.end.1);
            d.push_str(&format!(
                " C {:.5} {:.5} {:.5} {:.5} {:.5} {:.5}",
                control_1.x, control_1.y, control_2.x, control_2.y, end.x, end.y
            ));
            points.extend([control_1, control_2, end]);
            world_segments.push([control_1, control_2, end]);
        }
        let closed = world_segments
            .last()
            .is_some_and(|segment| segment[2].distance(start) <= 0.001);
        (d, points, world_segments, closed)
    }

    fn push_cubic_fill(
        &self,
        out: &mut Vec<RenderPrimitive>,
        d: &str,
        points: Vec<Point>,
        fill: &str,
    ) {
        out.push(RenderPrimitive::FilledPath {
            role: RenderRole::DocumentGraphic,
            object_id: self.object_id(),
            node_id: None,
            bond_id: None,
            d: format!("{d} Z"),
            points,
            fill: fill.to_string(),
            fill_rule: None,
            clip_path_d: None,
            clip_rule: None,
            rotate: 0.0,
            rotate_center: None,
        });
    }

    fn push_cubic_underlay(
        &self,
        out: &mut Vec<RenderPrimitive>,
        start_local: (f64, f64),
        segments: &[CubicSegment],
        fill: &str,
        stroke_width: Option<f64>,
    ) {
        let (d, points, _, _) = self.cubic_world_path(start_local, segments, 1.0);
        self.push_cubic_fill(out, &d, points.clone(), fill);
        if let Some(stroke_width) = stroke_width {
            out.push(RenderPrimitive::Path {
                role: RenderRole::DocumentGraphic,
                object_id: self.object_id(),
                bond_id: None,
                d,
                points,
                stroke: fill.to_string(),
                stroke_width,
                dash_array: Vec::new(),
                line_cap: None,
                line_join: None,
                rotate: 0.0,
                rotate_center: None,
            });
        }
    }

    fn shading_anchor_fraction(&self, index: usize) -> f64 {
        match self.data.kind {
            BioShapeKind::OneSubstrateEnzyme => 0.207_473_689,
            BioShapeKind::TwoSubstrateEnzyme => 0.284_698_795,
            BioShapeKind::Receptor => 0.251_633_251,
            BioShapeKind::GProteinAlpha => 0.033_484_112,
            BioShapeKind::GProteinBeta => 0.341_666_784,
            BioShapeKind::GProteinGamma => 0.223_558_944,
            BioShapeKind::Immunoglobulin => {
                const ANCHORS: [f64; 4] =
                    [0.343_134_866, 0.256_183_746, 0.440_002_480, 0.129_993_334];
                ANCHORS[index]
            }
            BioShapeKind::IonChannel => {
                const ANCHORS: [f64; 2] = [0.217_529_541, 0.334_681_053];
                ANCHORS[index]
            }
            BioShapeKind::EndoplasmicReticulum => ENDOPLASMIC_RETICULUM_SHADE_ANCHORS[index],
            BioShapeKind::Golgi => GOLGI_SHADE_ANCHORS[index],
            BioShapeKind::Mitochondrion => MITOCHONDRION_SHADE_ANCHORS[index],
            BioShapeKind::Cloud => 0.221_376_625,
            BioShapeKind::TRna => TRNA_SHADE_ANCHORS[index],
            BioShapeKind::RibosomeA => 0.216_241_458,
            BioShapeKind::RibosomeB => 0.281_636_867,
            BioShapeKind::HelixProtein => {
                unreachable!("helix protein supplies a verified anchor per generated component")
            }
            BioShapeKind::Dna => {
                const ANCHORS: [f64; 2] = [0.445_286_068, 0.239_711_155];
                ANCHORS[index]
            }
            BioShapeKind::MembraneLine
            | BioShapeKind::MembraneArc
            | BioShapeKind::MembraneEllipse
            | BioShapeKind::MembraneMicelle => {
                unreachable!("membrane heads use the verified circle shading rule")
            }
        }
    }
}

fn render_cubic_template(
    out: &mut Vec<RenderPrimitive>,
    geometry: &BioGeometry<'_>,
    template: &CubicTemplate,
) {
    geometry.push_cubic_template(out, template);
}

fn render_cubic_templates(
    out: &mut Vec<RenderPrimitive>,
    geometry: &BioGeometry<'_>,
    templates: &[&CubicTemplate],
) {
    for (index, template) in templates.iter().enumerate() {
        geometry.push_cubic_template_indexed(out, template, index);
    }
}

fn render_mitochondrion(out: &mut Vec<RenderPrimitive>, geometry: &BioGeometry<'_>) {
    geometry.push_cubic_segments_with_fill_and_anchor(
        out,
        MITOCHONDRION_TEMPLATES[0].start,
        MITOCHONDRION_TEMPLATES[0].segments,
        1.0,
        true,
        geometry.shading_anchor_fraction(0),
    );

    let inner = MITOCHONDRION_TEMPLATES[1];
    let (d, points, _, _) = geometry.cubic_world_path(inner.start, inner.segments, 1.0);
    geometry.push_cubic_fill(out, &d, points.clone(), "#d9d9d9");
    out.push(RenderPrimitive::Path {
        role: RenderRole::DocumentGraphic,
        object_id: geometry.object_id(),
        bond_id: None,
        d,
        points,
        stroke: geometry.data.color.clone(),
        stroke_width: geometry.data.line_width.max(0.05),
        dash_array: geometry.dash_array(),
        line_cap: None,
        line_join: None,
        rotate: 0.0,
        rotate_center: None,
    });
}

fn push_shaded_cubic_layers(
    out: &mut Vec<RenderPrimitive>,
    geometry: &BioGeometry<'_>,
    start: Point,
    segments: &[[Point; 3]],
    anchor_fraction: f64,
) {
    const CHEMDRAW_CLIP_SEGMENTS_PER_CUBIC: usize = 21;
    let mut polygon = vec![start];
    let mut from = start;
    for [control_1, control_2, end] in segments {
        for index in 1..=CHEMDRAW_CLIP_SEGMENTS_PER_CUBIC {
            let t = index as f64 / CHEMDRAW_CLIP_SEGMENTS_PER_CUBIC as f64;
            let inverse = 1.0 - t;
            polygon.push(Point::new(
                inverse.powi(3) * from.x
                    + 3.0 * inverse.powi(2) * t * control_1.x
                    + 3.0 * inverse * t.powi(2) * control_2.x
                    + t.powi(3) * end.x,
                inverse.powi(3) * from.y
                    + 3.0 * inverse.powi(2) * t * control_1.y
                    + 3.0 * inverse * t.powi(2) * control_2.y
                    + t.powi(3) * end.y,
            ));
        }
        from = *end;
    }
    polygon.dedup_by(|left, right| left.distance(*right) <= 0.000_001);
    if polygon.len() > 1
        && polygon
            .last()
            .is_some_and(|last| last.distance(polygon[0]) <= 0.000_001)
    {
        polygon.pop();
    }
    let polygon = simplify_collinear_polygon(&polygon);
    let clip_polygon = inset_polygon_bounding_box(&polygon, 0.1);
    let clip_path_d = polygon_path_d(&clip_polygon);
    let left = polygon
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let top = polygon
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let right = polygon
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let bottom = polygon
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let anchor = Point::new(
        left + (right - left) * anchor_fraction,
        top + (bottom - top) * anchor_fraction,
    );
    // tRNA's primary contour is a thick stroked centerline. ChemDraw clips its
    // shading against the expanded stroke outline rather than the centerline
    // path itself; using the centerline as an SVG clip would erase the body.
    let clip_path =
        (!matches!(geometry.data.kind, BioShapeKind::TRna)).then(|| clip_path_d.clone());
    let max_index = (SHADED_LEVELS.len() - 1) as f64;
    for (index, level) in SHADED_LEVELS.iter().enumerate() {
        let t = index as f64 / max_index;
        let scale = 1.0 - (1.0 - ELLIPSE_SHADED_REMAIN_RATIO) * t;
        let transform = |point: Point| {
            Point::new(
                anchor.x + (point.x - anchor.x) * scale,
                anchor.y + (point.y - anchor.y) * scale,
            )
        };
        let points = polygon.iter().copied().map(transform).collect::<Vec<_>>();
        let d = polygon_path_d(&points);
        out.push(RenderPrimitive::FilledPath {
            role: RenderRole::DocumentGraphic,
            object_id: geometry.object_id(),
            node_id: None,
            bond_id: None,
            d,
            points,
            fill: super::shapes::shaded_level_color(&geometry.data.color, level, t),
            fill_rule: None,
            clip_path_d: clip_path.clone(),
            clip_rule: clip_path.as_ref().map(|_| "nonzero".to_string()),
            rotate: 0.0,
            rotate_center: None,
        });
    }
}

fn simplify_collinear_polygon(points: &[Point]) -> Vec<Point> {
    if points.len() < 3 {
        return points.to_vec();
    }
    let mut simplified = Vec::with_capacity(points.len());
    for index in 0..points.len() {
        let previous = points[(index + points.len() - 1) % points.len()];
        let current = points[index];
        let next = points[(index + 1) % points.len()];
        let before = Vector::new(current.x - previous.x, current.y - previous.y);
        let after = Vector::new(next.x - current.x, next.y - current.y);
        let cross = before.x * after.y - before.y * after.x;
        let dot = before.x * after.x + before.y * after.y;
        if cross.abs() > 0.000_000_01 || dot < 0.0 {
            simplified.push(current);
        }
    }
    if simplified.len() < 3 {
        points.to_vec()
    } else {
        simplified
    }
}

fn polygon_path_d(points: &[Point]) -> String {
    let Some(first) = points.first() else {
        return String::new();
    };
    let mut d = format!("M {:.5} {:.5}", first.x, first.y);
    for point in &points[1..] {
        d.push_str(&format!(" L {:.5} {:.5}", point.x, point.y));
    }
    d.push_str(" Z");
    d
}

fn inset_polygon_bounding_box(points: &[Point], distance: f64) -> Vec<Point> {
    if points.len() < 3 || distance <= 0.0 {
        return points.to_vec();
    }
    let left = points
        .iter()
        .map(|point| point.x)
        .fold(f64::INFINITY, f64::min);
    let top = points
        .iter()
        .map(|point| point.y)
        .fold(f64::INFINITY, f64::min);
    let right = points
        .iter()
        .map(|point| point.x)
        .fold(f64::NEG_INFINITY, f64::max);
    let bottom = points
        .iter()
        .map(|point| point.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let width = right - left;
    let height = bottom - top;
    if width <= distance * 2.0 || height <= distance * 2.0 {
        return points.to_vec();
    }
    let horizontal_scale = (width - distance * 2.0) / width;
    let vertical_scale = (height - distance * 2.0) / height;
    points
        .iter()
        .map(|point| {
            Point::new(
                left + distance + (point.x - left) * horizontal_scale,
                top + distance + (point.y - top) * vertical_scale,
            )
        })
        .collect()
}

fn render_trna_templates(out: &mut Vec<RenderPrimitive>, geometry: &BioGeometry<'_>) {
    if matches!(geometry.data.fill_type, BioShapeFillType::Shaded) {
        render_trna_shaded_body(out, geometry);
    }
    for (index, template) in TRNA_TEMPLATES.iter().enumerate() {
        let (stroke, stroke_width) = match index {
            21 => ("#ffffff".to_string(), geometry.data.bold_width),
            0 if !matches!(geometry.data.fill_type, BioShapeFillType::Shaded) => (
                geometry.data.color.clone(),
                geometry.data.bold_width + geometry.data.line_width * 2.0,
            ),
            _ => (geometry.data.color.clone(), geometry.cubic_outline_width()),
        };
        geometry.push_cubic_segments_styled(
            out,
            template.start,
            template.segments,
            1.0,
            false,
            stroke,
            stroke_width,
            geometry.shading_anchor_fraction(0),
            None,
        );
    }
}

fn render_trna_shaded_body(out: &mut Vec<RenderPrimitive>, geometry: &BioGeometry<'_>) {
    let outer = TRNA_SHADE_OUTER_POLYGON
        .iter()
        .map(|point| geometry.point(point.0, point.1))
        .collect::<Vec<_>>();
    let clip = TRNA_SHADE_CLIP_POLYGON
        .iter()
        .map(|point| geometry.point(point.0, point.1))
        .collect::<Vec<_>>();
    let path_d = |points: &[Point]| {
        let mut d = format!("M {:.5} {:.5}", points[0].x, points[0].y);
        for point in &points[1..] {
            d.push_str(&format!(" L {:.5} {:.5}", point.x, point.y));
        }
        d.push_str(" Z");
        d
    };
    let clip_path_d = path_d(&clip);
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
    let anchor_fraction = geometry.shading_anchor_fraction(0);
    let anchor = Point::new(
        left + (right - left) * anchor_fraction,
        top + (bottom - top) * anchor_fraction,
    );
    let max_index = (SHADED_LEVELS.len() - 1) as f64;
    for (index, level) in SHADED_LEVELS.iter().enumerate() {
        let t = index as f64 / max_index;
        let scale = 1.0 - (1.0 - ELLIPSE_SHADED_REMAIN_RATIO) * t;
        let layer = outer
            .iter()
            .map(|point| {
                Point::new(
                    anchor.x + (point.x - anchor.x) * scale,
                    anchor.y + (point.y - anchor.y) * scale,
                )
            })
            .collect::<Vec<_>>();
        out.push(RenderPrimitive::FilledPath {
            role: RenderRole::DocumentGraphic,
            object_id: geometry.object_id(),
            node_id: None,
            bond_id: None,
            d: path_d(&layer),
            points: layer,
            fill: super::shapes::shaded_level_color(&geometry.data.color, level, t),
            fill_rule: None,
            clip_path_d: Some(clip_path_d.clone()),
            clip_rule: Some("nonzero".to_string()),
            rotate: 0.0,
            rotate_center: None,
        });
    }
}

fn blend_with_white(color: &str, white_fraction: f64) -> String {
    let Some((red, green, blue)) = super::shapes::parse_hex_color(color) else {
        return color.to_string();
    };
    let blend = |value: f64| {
        (value + (255.0 - value) * white_fraction)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    format!(
        "#{:02x}{:02x}{:02x}",
        blend(red as f64),
        blend(green as f64),
        blend(blue as f64)
    )
}

fn render_one_substrate_enzyme(out: &mut Vec<RenderPrimitive>, geometry: &BioGeometry<'_>) {
    let parameters = geometry.data.parameters.resolved_for(geometry.data.kind);
    let receptor_size = parameters
        .enzyme_receptor_size
        .expect("one-substrate defaults define enzyme receptor size")
        .max(0.0);
    let aperture = if receptor_size <= 50.0 {
        receptor_size / 50.0
    } else {
        50.0 / receptor_size
    };
    let maximum_t = if (aperture - 2.0 / 3.0).abs() <= f64::EPSILON {
        0.75
    } else {
        (2.0 - aperture - (aperture * (aperture + 2.0)).sqrt()) / (2.0 - 3.0 * aperture)
    };
    let inverse_t = 1.0 - maximum_t;
    let maximum_offset = 6.0 * inverse_t * inverse_t * maximum_t
        + 3.0 * (2.0 + aperture) * inverse_t * maximum_t * maximum_t
        + 2.0 * maximum_t * maximum_t * maximum_t;
    let center_x =
        (2.0 * (1.0 + aperture) - maximum_offset) / (2.0 * (1.0 + aperture) + maximum_offset);
    let control = (1.0 + center_x) / (2.0 * (1.0 + aperture));
    let shoulder = center_x + 2.0 * control;
    let inner = center_x + 2.0 * control * (1.0 - aperture);
    let segments = [
        cubic(
            (-1.0, -aperture),
            (center_x - 2.0 * control, -1.0),
            (center_x, -1.0),
        ),
        cubic(
            (shoulder, -1.0),
            (center_x + (2.0 + aperture) * control, -aperture),
            (shoulder, -aperture),
        ),
        cubic(
            (center_x + (2.0 - aperture) * control, -aperture),
            (inner, -aperture * 0.5),
            (inner, 0.0),
        ),
        cubic(
            (inner, aperture * 0.5),
            (center_x + (2.0 - aperture) * control, aperture),
            (shoulder, aperture),
        ),
        cubic(
            (center_x + (2.0 + aperture) * control, aperture),
            (shoulder, 1.0),
            (center_x, 1.0),
        ),
        cubic(
            (center_x - 2.0 * control, 1.0),
            (-1.0, aperture),
            (-1.0, 0.0),
        ),
        cubic((-1.0, 0.0), (-1.0, 0.0), (-1.0, 0.0)),
    ];
    geometry.push_cubic_segments(out, (-1.0, 0.0), &segments, 1.0);
}

fn render_receptor(out: &mut Vec<RenderPrimitive>, geometry: &BioGeometry<'_>) {
    let parameters = geometry.data.parameters.resolved_for(geometry.data.kind);
    let neck_width = parameters
        .neck_width
        .expect("receptor defaults define neck width")
        .max(0.0);
    let neck = if neck_width == 0.0 {
        0.0
    } else {
        neck_width / (neck_width + 187.5)
    };
    let lip_control = (16.0 / 15.0) * (1.0 - neck);
    let outer_control = 1.6 - 0.6 * neck;
    let outer = 0.8 + 0.2 * neck;
    let segments = [
        cubic((lip_control, 1.0), (lip_control * 0.5, 0.5), (neck, 0.5)),
        cubic((neck, 0.5), (neck, 0.0), (neck, 0.0)),
        cubic((outer_control, 0.0), (outer, -1.0), (outer, -1.0)),
        cubic((outer, -0.5), (outer, -0.5), (0.0, -0.5)),
        cubic((-outer, -0.5), (-outer, -0.5), (-outer, -1.0)),
        cubic((-outer, -1.0), (-outer_control, 0.0), (-neck, 0.0)),
        cubic((-neck, 0.0), (-neck, 0.5), (-neck, 0.5)),
        cubic((-lip_control * 0.5, 0.5), (-lip_control, 1.0), (0.0, 1.0)),
        cubic((0.0, 1.0), (0.0, 1.0), (0.0, 1.0)),
    ];
    geometry.push_cubic_segments(out, (0.0, 1.0), &segments, 1.0);
}

fn render_gprotein_gamma(out: &mut Vec<RenderPrimitive>, geometry: &BioGeometry<'_>) {
    let parameters = geometry.data.parameters.resolved_for(geometry.data.kind);
    let upper_height = parameters
        .gprotein_upper_height
        .expect("G-protein gamma defaults define upper height")
        .max(0.0);
    let shoulder_v = if upper_height == 0.0 {
        -1.0
    } else {
        (upper_height - 50.0) / (upper_height + 50.0)
    };
    let segments = [
        cubic((0.5, -1.0), (0.5, -1.0), (1.0, shoulder_v)),
        cubic((0.5, 1.0), (0.5, 1.0), (0.0, 1.0)),
        cubic((-0.5, 1.0), (-0.5, 1.0), (-1.0, shoulder_v)),
        cubic((-0.5, -1.0), (-0.5, -1.0), (0.0, -1.0)),
        cubic((0.0, -1.0), (0.0, -1.0), (0.0, -1.0)),
    ];
    geometry.push_cubic_segments(out, (0.0, -1.0), &segments, 1.0);
}

fn render_immunoglobulin(out: &mut Vec<RenderPrimitive>, geometry: &BioGeometry<'_>) {
    // ChemDraw preserves these two fields but does not use them when rendering
    // the Immunoglobin BioShape. Its four visible arms are one affine template.
    for (index, u_scale) in [1.0, -1.0].into_iter().enumerate() {
        geometry.push_scaled_cubic_template(out, &IMMUNOGLOBULIN_MAIN_TEMPLATE, u_scale, index);
    }
    for (offset, u_scale) in [1.0, -1.0].into_iter().enumerate() {
        geometry.push_scaled_cubic_template(out, &IMMUNOGLOBULIN_ARM_TEMPLATE, u_scale, offset + 2);
    }
}

fn render_membrane_line(out: &mut Vec<RenderPrimitive>, geometry: &BioGeometry<'_>) {
    let parameters = geometry.data.parameters.resolved_for(geometry.data.kind);
    let element_size = parameters
        .membrane_element_size
        .expect("membrane defaults define element size")
        .max(0.1);
    let major_half = geometry.point(1.0, 0.0).distance(geometry.point(0.0, 0.0));
    let minor_half = geometry.point(0.0, 1.0).distance(geometry.point(0.0, 0.0));
    let count = ((major_half * 2.0 / element_size).round() as usize + 1).max(2);
    let head_offset_v = element_size / minor_half;
    let head_inner_v = element_size * 0.5 / minor_half;
    let tail_end_v = element_size * 0.05 / minor_half;
    let tail_offset_u = element_size * 0.125 / major_half;
    for index in 0..count {
        let u = -1.0 + 2.0 * index as f64 / (count - 1) as f64;
        for side in [-1.0, 1.0] {
            geometry.push_circle(
                out,
                geometry.point(u, 1.0 + side * head_offset_v),
                element_size * 0.5,
            );
            for tangent in [-1.0, 1.0] {
                geometry.push_line(
                    out,
                    geometry.point(u + tangent * tail_offset_u, 1.0 + side * head_inner_v),
                    geometry.point(u + tangent * tail_offset_u, 1.0 + side * tail_end_v),
                    geometry.data.line_width.max(0.05),
                );
            }
        }
    }
}

fn render_membrane_arc(out: &mut Vec<RenderPrimitive>, geometry: &BioGeometry<'_>) {
    let parameters = geometry.data.parameters.resolved_for(geometry.data.kind);
    let start = parameters
        .membrane_start_angle
        .expect("membrane arc defaults define start angle")
        .to_radians();
    let end = parameters
        .membrane_end_angle
        .expect("membrane arc defaults define end angle")
        .to_radians();
    let element_size = parameters
        .membrane_element_size
        .expect("membrane defaults define element size")
        .max(0.1);
    let major = geometry.point(1.0, 0.0).distance(geometry.point(0.0, 0.0));
    let minor = geometry.point(0.0, 1.0).distance(geometry.point(0.0, 0.0));
    let arc = ellipse_arc_table(major, minor, start, end, 2048);
    let count = ((arc.total / element_size).round() as usize).max(2);
    for index in 0..count {
        let distance = arc.total * index as f64 / (count - 1) as f64;
        let angle = arc.angle_at_distance(distance);
        render_bilayer_pair(out, geometry, angle, element_size);
    }
}

fn render_membrane_ellipse(out: &mut Vec<RenderPrimitive>, geometry: &BioGeometry<'_>) {
    let parameters = geometry.data.parameters.resolved_for(geometry.data.kind);
    let element_size = parameters
        .membrane_element_size
        .expect("membrane defaults define element size")
        .max(0.1);
    let major = geometry.point(1.0, 0.0).distance(geometry.point(0.0, 0.0));
    let minor = geometry.point(0.0, 1.0).distance(geometry.point(0.0, 0.0));
    let arc = ellipse_arc_table(major, minor, 0.0, TAU, 4096);
    let count = ((arc.total / element_size).round() as usize).max(4);
    for index in 0..count {
        let distance = arc.total * index as f64 / count as f64;
        render_bilayer_pair(out, geometry, arc.angle_at_distance(distance), element_size);
    }
}

struct EllipseArcTable {
    start: f64,
    end: f64,
    cumulative: Vec<f64>,
    total: f64,
}

impl EllipseArcTable {
    fn angle_at_distance(&self, distance: f64) -> f64 {
        let target = distance.clamp(0.0, self.total);
        let upper = self.cumulative.partition_point(|value| *value < target);
        if upper == 0 {
            return self.start;
        }
        if upper >= self.cumulative.len() {
            return self.end;
        }
        let lower = upper - 1;
        let before = self.cumulative[lower];
        let after = self.cumulative[upper];
        let fraction = if after > before {
            (target - before) / (after - before)
        } else {
            0.0
        };
        let table_fraction = (lower as f64 + fraction) / (self.cumulative.len() - 1) as f64;
        self.start + (self.end - self.start) * table_fraction
    }
}

fn ellipse_arc_table(
    major: f64,
    minor: f64,
    start: f64,
    end: f64,
    steps: usize,
) -> EllipseArcTable {
    let mut cumulative = Vec::with_capacity(steps + 1);
    cumulative.push(0.0);
    let mut total = 0.0;
    let mut previous = Point::new(major * start.cos(), minor * start.sin());
    for index in 1..=steps {
        let fraction = index as f64 / steps as f64;
        let angle = start + (end - start) * fraction;
        let point = Point::new(major * angle.cos(), minor * angle.sin());
        total += previous.distance(point);
        cumulative.push(total);
        previous = point;
    }
    EllipseArcTable {
        start,
        end,
        cumulative,
        total,
    }
}

fn render_bilayer_pair(
    out: &mut Vec<RenderPrimitive>,
    geometry: &BioGeometry<'_>,
    angle: f64,
    element_size: f64,
) {
    let center = geometry.point(angle.cos(), angle.sin());
    let center_origin = geometry.point(0.0, 0.0);
    let major = Vector::new(
        geometry.point(1.0, 0.0).x - center_origin.x,
        geometry.point(1.0, 0.0).y - center_origin.y,
    );
    let minor = Vector::new(
        geometry.point(0.0, 1.0).x - center_origin.x,
        geometry.point(0.0, 1.0).y - center_origin.y,
    );
    let tangent = Vector::new(
        -major.x * angle.sin() + minor.x * angle.cos(),
        -major.y * angle.sin() + minor.y * angle.cos(),
    );
    let tangent_length = (tangent.x * tangent.x + tangent.y * tangent.y).sqrt();
    let tangent = Vector::new(tangent.x / tangent_length, tangent.y / tangent_length);
    let mut normal = Vector::new(tangent.y, -tangent.x);
    let radial = Vector::new(center.x - center_origin.x, center.y - center_origin.y);
    if normal.x * radial.x + normal.y * radial.y < 0.0 {
        normal = Vector::new(-normal.x, -normal.y);
    }
    for side in [-1.0, 1.0] {
        geometry.push_circle(
            out,
            center.translated(normal.scaled(side * element_size)),
            element_size * 0.5,
        );
        for tangent_side in [-1.0, 1.0] {
            let lateral = tangent.scaled(tangent_side * element_size * 0.125);
            geometry.push_line(
                out,
                center
                    .translated(normal.scaled(side * element_size * 0.5))
                    .translated(lateral),
                center
                    .translated(normal.scaled(side * element_size * 0.05))
                    .translated(lateral),
                geometry.data.line_width.max(0.05),
            );
        }
    }
}

fn render_membrane_micelle(out: &mut Vec<RenderPrimitive>, geometry: &BioGeometry<'_>) {
    let parameters = geometry.data.parameters.resolved_for(geometry.data.kind);
    let element_size = parameters
        .membrane_element_size
        .expect("membrane defaults define element size")
        .max(0.1);
    let origin = geometry.point(0.0, 0.0);
    let micelle_center = geometry.point(0.0, 1.0);
    let major_vector = Vector::new(
        geometry.point(1.0, 0.0).x - origin.x,
        geometry.point(1.0, 0.0).y - origin.y,
    );
    let major_length = (major_vector.x * major_vector.x + major_vector.y * major_vector.y).sqrt();
    let major_unit = Vector::new(major_vector.x / major_length, major_vector.y / major_length);
    let minor_vector = Vector::new(
        geometry.point(0.0, 1.0).x - origin.x,
        geometry.point(0.0, 1.0).y - origin.y,
    );
    let mut minor_unit = Vector::new(-major_unit.y, major_unit.x);
    if minor_unit.x * minor_vector.x + minor_unit.y * minor_vector.y < 0.0 {
        minor_unit = Vector::new(-minor_unit.x, -minor_unit.y);
    }
    let baseline_radius = major_length * 1.2;
    let count = ((TAU * baseline_radius / element_size).floor() as usize).max(4);
    let wave_amplitude = geometry.data.bold_width * 0.5;
    let wave_segments = (element_size * 1.6).round().max(1.0) as usize;
    let radial_step = 3.0 * element_size / (3.0 * element_size + 2.0);
    let kappa = 0.552_284_749_830_793_6;
    for index in 0..count {
        let angle = -TAU * (index + 1) as f64 / count as f64;
        let radial = Vector::new(
            major_unit.x * angle.cos() + minor_unit.x * angle.sin(),
            major_unit.y * angle.cos() + minor_unit.y * angle.sin(),
        );
        let tangent = Vector::new(-radial.y, radial.x);
        geometry.push_circle(
            out,
            micelle_center.translated(radial.scaled(baseline_radius + element_size)),
            element_size * 0.5,
        );
        let start_radius = baseline_radius + element_size * 0.5;
        let start = micelle_center.translated(radial.scaled(start_radius));
        let mut d = format!("M {:.5} {:.5}", start.x, start.y);
        let mut points = vec![start];
        let mut previous = start;
        for segment_index in 0..wave_segments {
            let phase = segment_index % 4;
            let previous_tangent = match phase {
                0 => 0.0,
                1 => -wave_amplitude,
                2 => 0.0,
                _ => wave_amplitude,
            };
            let (
                next_tangent,
                control_1_radial,
                control_1_tangent,
                control_2_radial,
                control_2_tangent,
            ) = match phase {
                0 => (
                    -wave_amplitude,
                    0.0,
                    -kappa * wave_amplitude,
                    (1.0 - kappa) * radial_step,
                    -wave_amplitude,
                ),
                1 => (
                    0.0,
                    kappa * radial_step,
                    -wave_amplitude,
                    radial_step,
                    -kappa * wave_amplitude,
                ),
                2 => (
                    wave_amplitude,
                    0.0,
                    kappa * wave_amplitude,
                    (1.0 - kappa) * radial_step,
                    wave_amplitude,
                ),
                _ => (
                    0.0,
                    kappa * radial_step,
                    wave_amplitude,
                    radial_step,
                    kappa * wave_amplitude,
                ),
            };
            let end_radius = start_radius - radial_step * (segment_index + 1) as f64;
            let end = micelle_center
                .translated(radial.scaled(end_radius))
                .translated(tangent.scaled(next_tangent));
            let control_1 = previous
                .translated(radial.scaled(-control_1_radial))
                .translated(tangent.scaled(control_1_tangent - previous_tangent));
            let control_2 = previous
                .translated(radial.scaled(-control_2_radial))
                .translated(tangent.scaled(control_2_tangent - previous_tangent));
            d.push_str(&format!(
                " C {:.5} {:.5} {:.5} {:.5} {:.5} {:.5}",
                control_1.x, control_1.y, control_2.x, control_2.y, end.x, end.y
            ));
            points.extend([control_1, control_2, end]);
            previous = end;
        }
        out.push(RenderPrimitive::Path {
            role: RenderRole::DocumentGraphic,
            object_id: geometry.object_id(),
            bond_id: None,
            d,
            points,
            stroke: geometry.data.color.clone(),
            stroke_width: geometry.data.line_width.max(0.05),
            dash_array: geometry.dash_array(),
            line_cap: None,
            line_join: None,
            rotate: 0.0,
            rotate_center: None,
        });
    }
}

fn render_dna(out: &mut Vec<RenderPrimitive>, geometry: &BioGeometry<'_>) {
    let parameters = geometry.data.parameters.resolved_for(geometry.data.kind);
    let wave_height = parameters
        .dna_wave_height
        .expect("DNA defaults define wave height")
        .max(0.0);
    let half_wave = parameters
        .dna_wave_length
        .expect("DNA defaults define wave length")
        .max(0.1)
        * 0.5;
    let second_offset = parameters
        .dna_wave_offset
        .expect("DNA defaults define second-strand offset")
        .max(0.0);
    let ribbon_width = parameters
        .dna_wave_width
        .expect("DNA defaults define strand width")
        .max(0.0);
    let major_half = geometry.point(1.0, 0.0).distance(geometry.point(0.0, 0.0));
    let minor_half = geometry.point(0.0, 1.0).distance(geometry.point(0.0, 0.0));
    let full_length = major_half * 2.0;
    let to_u = |distance: f64| -1.0 + distance / major_half;
    let bottom_v = 1.0;
    let top_v = 1.0 - wave_height / minor_half;
    let mut pieces = Vec::new();
    for (strand_index, (strand_offset, starts_down)) in [(0.0, true), (second_offset, false)]
        .into_iter()
        .enumerate()
    {
        let mut strand_distance = 0.0;
        let mut down = starts_down;
        while strand_distance + half_wave <= full_length + crate::EPSILON {
            let start = strand_distance + strand_offset;
            let start_u = to_u(start);
            let middle_u = to_u(start + half_wave * 0.5);
            let end_u = to_u(start + half_wave);
            let width_start_u = to_u(start + ribbon_width);
            let width_middle_u = to_u(start + ribbon_width + half_wave * 0.5);
            let width_end_u = to_u(start + ribbon_width + half_wave);
            let (from_v, to_v) = if down {
                (bottom_v, top_v)
            } else {
                (top_v, bottom_v)
            };
            let segments = vec![
                cubic((middle_u, from_v), (middle_u, to_v), (end_u, to_v)),
                cubic((end_u, to_v), (width_end_u, to_v), (width_end_u, to_v)),
                cubic(
                    (width_middle_u, to_v),
                    (width_middle_u, from_v),
                    (width_start_u, from_v),
                ),
                cubic(
                    (width_start_u, from_v),
                    (start_u, from_v),
                    (start_u, from_v),
                ),
                cubic((start_u, from_v), (start_u, from_v), (start_u, from_v)),
            ];
            pieces.push((
                (start_u, from_v),
                segments,
                geometry.shading_anchor_fraction(strand_index),
                !down,
            ));
            strand_distance += half_wave;
            down = !down;
        }
    }
    for front in [false, true] {
        for (start, segments, anchor, _) in pieces.iter().filter(|piece| piece.3 == front) {
            if front {
                geometry.push_cubic_underlay(out, *start, segments, "#ffffff", Some(0.05));
            }
            geometry.push_cubic_segments_styled(
                out,
                *start,
                segments,
                1.0,
                true,
                geometry.data.color.clone(),
                geometry.cubic_outline_width(),
                *anchor,
                (!front).then_some("#b3b3b3"),
            );
        }
    }
}

fn render_helix_protein(out: &mut Vec<RenderPrimitive>, geometry: &BioGeometry<'_>) {
    let parameters = geometry.data.parameters.resolved_for(geometry.data.kind);
    let cylinder_distance = parameters
        .cylinder_distance
        .expect("helix defaults define cylinder distance")
        .max(0.1);
    let cylinder_height = parameters
        .cylinder_height
        .expect("helix defaults define cylinder height")
        .max(0.1);
    let cylinder_width = parameters
        .cylinder_width
        .expect("helix defaults define cylinder width")
        .max(0.1);
    let extra = parameters
        .helix_protein_extra
        .expect("helix defaults define terminal extension")
        .max(0.0);
    let pipe_width = parameters
        .pipe_width
        .expect("helix defaults define strand width")
        .max(0.05);
    let center = geometry.point(0.0, 0.0);
    let major_half = center.distance(geometry.point(1.0, 0.0));
    let minor_half = center.distance(geometry.point(0.0, 1.0));
    let full_length = major_half * 2.0;
    let pitch = cylinder_width + cylinder_distance;
    let cylinder_count = ((full_length / pitch).floor() as usize).max(2);
    let local = |x: f64, y: f64| (-1.0 + x / major_half, 1.0 + y / minor_half);
    let push = |out: &mut Vec<RenderPrimitive>,
                start: (f64, f64),
                segments: Vec<CubicSegment>,
                anchor: f64| {
        geometry.push_cubic_segments_styled(
            out,
            start,
            &segments,
            1.0,
            true,
            geometry.data.color.clone(),
            geometry.cubic_outline_width(),
            anchor,
            Some("#b3b3b3"),
        );
    };
    let half_height = cylinder_height * 0.5;
    let quarter_width = cylinder_width * 0.25;

    let loop_segments = |center_x: f64, bottom: bool| {
        let outer_radius = (pitch + pipe_width) * 0.5;
        let inner_radius = (pitch - pipe_width).max(0.05) * 0.5;
        let direction = if bottom { 1.0 } else { -1.0 };
        let outer_connection = if bottom {
            half_height + pipe_width
        } else {
            -half_height - pipe_width * 0.5
        };
        let inner_connection = if bottom {
            half_height
        } else {
            -half_height + pipe_width * 0.5
        };
        let start_y = outer_connection + direction * outer_radius;
        let inner_mid_y = outer_connection + direction * inner_radius;
        let start = local(center_x, start_y);
        let segments = vec![
            cubic(
                local(center_x + outer_radius * 0.5, start_y),
                local(
                    center_x + outer_radius,
                    outer_connection + direction * outer_radius * 0.5,
                ),
                local(center_x + outer_radius, outer_connection),
            ),
            cubic(
                local(center_x + outer_radius, inner_connection),
                local(center_x + inner_radius, inner_connection),
                local(center_x + inner_radius, outer_connection),
            ),
            cubic(
                local(
                    center_x + inner_radius,
                    outer_connection + direction * inner_radius * 0.5,
                ),
                local(center_x + inner_radius * 0.5, inner_mid_y),
                local(center_x, inner_mid_y),
            ),
            cubic(
                local(center_x - inner_radius * 0.5, inner_mid_y),
                local(
                    center_x - inner_radius,
                    outer_connection + direction * inner_radius * 0.5,
                ),
                local(center_x - inner_radius, outer_connection),
            ),
            cubic(
                local(center_x - inner_radius, inner_connection),
                local(center_x - outer_radius, inner_connection),
                local(center_x - outer_radius, outer_connection),
            ),
            cubic(
                local(
                    center_x - outer_radius,
                    outer_connection + direction * outer_radius * 0.5,
                ),
                local(center_x - outer_radius * 0.5, start_y),
                start,
            ),
            cubic(start, start, start),
        ];
        (start, segments)
    };

    for index in (0..cylinder_count.saturating_sub(1)).step_by(2) {
        let center_x = index as f64 * pitch + cylinder_width + cylinder_distance * 0.5;
        let (start, segments) = loop_segments(center_x, true);
        push(out, start, segments, 0.170_053_040);
    }

    let right_center =
        (cylinder_count - 2) as f64 * pitch + cylinder_width + cylinder_distance * 0.5;
    let outer_radius = (pitch + pipe_width) * 0.5;
    let inner_radius = (pitch - pipe_width).max(0.05) * 0.5;
    let right_outer_x = right_center + outer_radius;
    let right_inner_x = right_outer_x - pipe_width;
    let right_outer_connection_y = half_height + pipe_width;
    let right_inner_connection_y = half_height;
    let right_curve_y = right_outer_connection_y + extra * 2.0;
    let right_outer_y = right_curve_y + outer_radius;
    let right_inner_y = right_outer_y - pipe_width;
    let right_tail_x = right_center - pipe_width * 10.0;
    let start = local(right_center, right_outer_y);
    let segments = vec![
        cubic(
            local(right_center + outer_radius * 0.5, right_outer_y),
            local(right_outer_x, right_outer_y - outer_radius * 0.5),
            local(right_outer_x, right_curve_y),
        ),
        cubic(
            local(right_outer_x, right_curve_y),
            local(right_outer_x, right_outer_connection_y),
            local(right_outer_x, right_outer_connection_y),
        ),
        cubic(
            local(right_outer_x, right_inner_connection_y),
            local(right_inner_x, right_inner_connection_y),
            local(right_inner_x, right_outer_connection_y),
        ),
        cubic(
            local(right_inner_x, right_outer_connection_y),
            local(right_inner_x, right_curve_y),
            local(right_inner_x, right_curve_y),
        ),
        cubic(
            local(right_inner_x, right_curve_y + inner_radius * 0.5),
            local(right_center + inner_radius * 0.5, right_inner_y),
            local(right_center, right_inner_y),
        ),
        cubic(
            local(right_center, right_inner_y),
            local(right_tail_x, right_inner_y),
            local(right_tail_x, right_inner_y),
        ),
        cubic(
            local(right_tail_x, right_inner_y),
            local(right_tail_x, right_outer_y),
            local(right_tail_x, right_outer_y),
        ),
        cubic(
            local(right_tail_x, right_outer_y),
            local(right_center, right_outer_y),
            start,
        ),
        cubic(start, start, start),
    ];
    push(out, start, segments, 0.098_221_983);

    for index in 0..cylinder_count {
        let x0 = index as f64 * pitch;
        let x1 = x0 + cylinder_width;
        let start = local(x0, 0.0);
        let body = vec![
            cubic(start, local(x0, half_height), local(x0, half_height)),
            cubic(
                local(x0, half_height + quarter_width),
                local(x1, half_height + quarter_width),
                local(x1, half_height),
            ),
            cubic(local(x1, half_height), local(x1, 0.0), local(x1, 0.0)),
            cubic(
                local(x1, 0.0),
                local(x1, -half_height),
                local(x1, -half_height),
            ),
            cubic(
                local(x1, -half_height - quarter_width),
                local(x0, -half_height - quarter_width),
                local(x0, -half_height),
            ),
            cubic(local(x0, -half_height), local(x0, 0.0), local(x0, 0.0)),
            cubic(start, start, start),
        ];
        push(out, start, body, 0.146_251_030);
        let cap_start = local(x0, -half_height);
        let cap = vec![
            cubic(
                local(x0, -half_height + quarter_width),
                local(x1, -half_height + quarter_width),
                local(x1, -half_height),
            ),
            cubic(
                local(x1, -half_height - quarter_width),
                local(x0, -half_height - quarter_width),
                cap_start,
            ),
            cubic(cap_start, cap_start, cap_start),
        ];
        push(out, cap_start, cap, 0.198_159_846);
    }

    let left_center = cylinder_width + cylinder_distance * 0.5;
    let left_outer_x = left_center - outer_radius;
    let left_inner_x = left_outer_x + pipe_width;
    let left_outer_connection_y = -half_height - pipe_width * 0.5;
    let left_inner_connection_y = -half_height + pipe_width * 0.5;
    let left_curve_y = left_outer_connection_y - extra * 2.0;
    let left_outer_y = left_curve_y - outer_radius;
    let left_inner_y = left_outer_y + pipe_width;
    let left_tail_x = left_center + pipe_width * 10.0;
    let start = local(left_center, left_outer_y);
    let segments = vec![
        cubic(
            local(left_center - outer_radius * 0.5, left_outer_y),
            local(left_outer_x, left_outer_y + outer_radius * 0.5),
            local(left_outer_x, left_curve_y),
        ),
        cubic(
            local(left_outer_x, left_curve_y),
            local(left_outer_x, left_outer_connection_y),
            local(left_outer_x, left_outer_connection_y),
        ),
        cubic(
            local(left_outer_x, left_inner_connection_y),
            local(left_inner_x, left_inner_connection_y),
            local(left_inner_x, left_outer_connection_y),
        ),
        cubic(
            local(left_inner_x, left_outer_connection_y),
            local(left_inner_x, left_curve_y),
            local(left_inner_x, left_curve_y),
        ),
        cubic(
            local(left_inner_x, left_curve_y - inner_radius * 0.5),
            local(left_center - inner_radius * 0.5, left_inner_y),
            local(left_center, left_inner_y),
        ),
        cubic(
            local(left_center, left_inner_y),
            local(left_tail_x, left_inner_y),
            local(left_tail_x, left_inner_y),
        ),
        cubic(
            local(left_tail_x, left_inner_y),
            local(left_tail_x, left_outer_y),
            local(left_tail_x, left_outer_y),
        ),
        cubic(
            local(left_tail_x, left_outer_y),
            local(left_center, left_outer_y),
            start,
        ),
        cubic(start, start, start),
    ];
    push(out, start, segments, 0.402_478_448);
    for index in (1..cylinder_count.saturating_sub(1)).step_by(2) {
        let center_x = index as f64 * pitch + cylinder_width + cylinder_distance * 0.5;
        let (start, segments) = loop_segments(center_x, false);
        push(out, start, segments, 0.283_068_135);
    }
}
