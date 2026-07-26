use super::*;
use crate::angle_between;

#[path = "graphics/brackets.rs"]
mod brackets;
#[path = "graphics/curves.rs"]
mod curves;
#[path = "graphics/orbitals.rs"]
mod orbitals;
#[path = "graphics/shapes.rs"]
mod shapes;
#[path = "graphics/tables.rs"]
mod tables;

use brackets::*;
use orbitals::*;
use shapes::*;
use tables::*;

pub(crate) use brackets::render_bracket_object;
pub(crate) use curves::render_curve_object;
pub(crate) use shapes::render_shape_object;
pub(crate) use tables::render_table_object;

#[derive(Clone, Copy)]
struct OrbitalLobeProfile {
    start_ctrl: f64,
    side_ctrl: f64,
    belly_ctrl: f64,
    shoulder: f64,
    tip_ctrl: f64,
    tip_half: f64,
}

#[derive(Clone, Copy)]
struct OrbitalLobeGeometry {
    apex: Point,
    c1: Point,
    c2: Point,
    p1: Point,
    c3: Point,
    c4: Point,
    tip: Point,
    c5: Point,
    c6: Point,
    p2: Point,
    c7: Point,
    c8: Point,
}

// These lobe profiles are calibrated against ChemDraw's exported orbital templates.
// Keep them centralized so geometry tweaks stay explicit instead of drifting as inline literals.
const P_ORBITAL_PROFILE: OrbitalLobeProfile = OrbitalLobeProfile {
    start_ctrl: 0.156,
    side_ctrl: 0.291,
    belly_ctrl: 0.51,
    shoulder: 0.667,
    tip_ctrl: 0.86,
    tip_half: 0.25,
};

const DXY_ORBITAL_PROFILE: OrbitalLobeProfile = OrbitalLobeProfile {
    start_ctrl: 0.0,
    side_ctrl: 0.352,
    belly_ctrl: 0.357,
    shoulder: 0.668,
    tip_ctrl: 0.86,
    tip_half: 0.25,
};

#[derive(Debug, Clone, Copy)]
struct ChargeSymbolLayout {
    circle_sign_size: f64,
    circle_sign_offset: f64,
    radical_sign_size: f64,
    sign_thickness: f64,
    dot_diameter: f64,
    radical_gap: f64,
    lone_pair_gap: f64,
}

impl ChargeSymbolLayout {
    fn scaled(self, factor: f64) -> Self {
        Self {
            circle_sign_size: self.circle_sign_size * factor,
            circle_sign_offset: self.circle_sign_offset * factor,
            radical_sign_size: self.radical_sign_size * factor,
            sign_thickness: self.sign_thickness * factor,
            dot_diameter: self.dot_diameter * factor,
            radical_gap: self.radical_gap * factor,
            lone_pair_gap: self.lone_pair_gap * factor,
        }
    }
}

#[derive(Clone)]
struct ShapeStyleSpec {
    fill: Option<String>,
    stroke: Option<String>,
    stroke_width: f64,
    dash_array: Vec<f64>,
    fill_gradient: Option<JsonValue>,
    render_style: ShapeRenderStyle,
    shadow_size: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ShapeRenderStyle {
    Solid,
    Dashed,
    Filled,
    Shaded,
    Shadowed,
    Custom,
}

impl ShapeStyleSpec {
    fn from_style(style: Option<&JsonValue>) -> Self {
        let fill = style.and_then(|value| style_nullable_string(value, "fill"));
        let stroke = style.and_then(|value| style_nullable_string(value, "stroke"));
        let stroke_width = style
            .and_then(|value| {
                style_number(value, "strokeWidth").or_else(|| style_number(value, "stroke_width"))
            })
            .unwrap_or(px_to_pt(1.0));
        let dash_array = style
            .and_then(|value| style_number_array(value, "dashArray"))
            .unwrap_or_default();
        let fill_gradient = style
            .and_then(|value| value.get("fillGradient").cloned())
            .filter(|value| !value.is_null());
        let shaded = style
            .and_then(|value| value.get("shaded"))
            .and_then(JsonValue::as_bool)
            .unwrap_or(false);
        let shadowed = style
            .and_then(|value| value.get("shadow"))
            .and_then(JsonValue::as_bool)
            .unwrap_or(false);
        let shadow_size = style
            .and_then(|value| style_number(value, "shadowSize"))
            .unwrap_or(4.0);
        let render_style = if shaded {
            ShapeRenderStyle::Shaded
        } else if shadowed {
            ShapeRenderStyle::Shadowed
        } else if fill.is_some() && stroke.is_none() && fill_gradient.is_none() {
            ShapeRenderStyle::Filled
        } else if fill.is_none() && stroke.is_some() && !dash_array.is_empty() {
            ShapeRenderStyle::Dashed
        } else if fill.is_none() && stroke.is_some() && dash_array.is_empty() {
            ShapeRenderStyle::Solid
        } else {
            ShapeRenderStyle::Custom
        };
        Self {
            fill,
            stroke,
            stroke_width,
            dash_array,
            fill_gradient,
            render_style,
            shadow_size,
        }
    }

    fn base_color(&self) -> &str {
        self.stroke
            .as_deref()
            .or(self.fill.as_deref())
            .unwrap_or("#000000")
    }
}

enum ShapeGeometry {
    Oval {
        center: Point,
        rx: f64,
        ry: f64,
        rotate: f64,
        ellipse: bool,
    },
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        corner_radius: Option<f64>,
        rounded: bool,
        rotate: f64,
    },
}

impl ShapeGeometry {
    fn from_object(object: &SceneObject) -> Option<Self> {
        let [tx, ty] = object.transform.translate;
        let kind = payload_string(&object.payload, "kind").unwrap_or_else(|| "rect".to_string());
        if matches!(kind.as_str(), "circle" | "ellipse") {
            let center = payload_point(&object.payload, "center")?;
            let major_axis_end = payload_point(&object.payload, "majorAxisEnd")?;
            let minor_axis_end = payload_point(&object.payload, "minorAxisEnd")?;
            let rx = center.distance(major_axis_end);
            let ry = center.distance(minor_axis_end);
            if rx <= crate::EPSILON || ry <= crate::EPSILON {
                return None;
            }
            return Some(Self::Oval {
                center,
                rx,
                ry,
                rotate: crate::angle_between(center, major_axis_end),
                ellipse: kind == "ellipse",
            });
        }

        let [_, _, width, height] = object.payload.bbox?;
        if width <= 0.0 || height <= 0.0 {
            return None;
        }
        let corner_radius =
            payload_number(&object.payload, "cornerRadius").filter(|value| *value > 0.0);
        Some(Self::Rect {
            x: tx,
            y: ty,
            width,
            height,
            corner_radius,
            rounded: kind == "roundRect",
            rotate: object.transform.rotate,
        })
    }

    fn fill_path_d(&self) -> String {
        match *self {
            Self::Oval {
                center,
                rx,
                ry,
                rotate,
                ellipse,
            } => oval_path_d(center, rx, ry, rotate, ellipse),
            Self::Rect {
                x,
                y,
                width,
                height,
                corner_radius,
                rounded,
                rotate,
            } => {
                if rotate.abs() > crate::EPSILON {
                    rotated_rect_path_d(x, y, width, height, rotate)
                } else if rounded {
                    rounded_rect_path_d(x, y, width, height, corner_radius.unwrap_or(0.0))
                } else {
                    rect_path_d(x, y, width, height)
                }
            }
        }
    }

    fn outline_path_d(&self, dash_array: &[f64]) -> String {
        match *self {
            Self::Oval {
                center,
                rx,
                ry,
                rotate,
                ellipse,
            } => oval_path_d(center, rx, ry, rotate, ellipse || !dash_array.is_empty()),
            _ => self.fill_path_d(),
        }
    }

    fn shifted_fill_path_d(&self, dx: f64, dy: f64) -> String {
        match *self {
            Self::Oval {
                center,
                rx,
                ry,
                rotate,
                ellipse,
            } => oval_path_d(
                center.translated(crate::Vector::new(dx, dy)),
                rx,
                ry,
                rotate,
                ellipse,
            ),
            Self::Rect {
                x,
                y,
                width,
                height,
                corner_radius,
                rounded,
                rotate,
            } => {
                if rotate.abs() > crate::EPSILON {
                    rotated_rect_path_d(x + dx, y + dy, width, height, rotate)
                } else if rounded {
                    rounded_rect_path_d(x + dx, y + dy, width, height, corner_radius.unwrap_or(0.0))
                } else {
                    rect_path_d(x + dx, y + dy, width, height)
                }
            }
        }
    }

    fn bounds_points(&self) -> Vec<Point> {
        match *self {
            Self::Oval {
                center,
                rx,
                ry,
                rotate,
                ..
            } => ellipse_bounds_points(center, rx, ry, rotate),
            Self::Rect {
                x,
                y,
                width,
                height,
                rotate,
                ..
            } => rotated_rect_points(x, y, width, height, rotate),
        }
    }

    fn shadow_bounds_points(&self, offset: f64) -> Vec<Point> {
        match *self {
            Self::Oval {
                center,
                rx,
                ry,
                rotate,
                ..
            } => {
                let mut points = ellipse_bounds_points(center, rx, ry, rotate);
                points.push(Point::new(points[1].x + offset, points[1].y + offset));
                points
            }
            Self::Rect {
                x,
                y,
                width,
                height,
                rotate,
                ..
            } => {
                let mut points = rotated_rect_points(x, y, width, height, rotate);
                let shifted = points
                    .iter()
                    .map(|point| Point::new(point.x + offset, point.y + offset))
                    .collect::<Vec<_>>();
                points.extend(shifted);
                points
            }
        }
    }
}

const SHADED_LEVELS: &[&str] = &[
    "#000000", "#0f0f0f", "#1e1e1e", "#2d2d2d", "#3b3b3b", "#494949", "#565656", "#636363",
    "#6f6f6f", "#7b7b7b", "#868686", "#919191", "#9b9b9b", "#a5a5a5", "#aeaeae", "#b7b7b7",
    "#bfbfbf", "#c7c7c7", "#cecece", "#d5d5d5", "#dbdbdb", "#e1e1e1", "#e6e6e6", "#ebebeb",
    "#efefef", "#f3f3f3", "#f6f6f6", "#f9f9f9", "#fbfbfb", "#fdfdfd", "#fefefe", "#ffffff",
];

const CIRCLE_SHADED_LEVELS: &[&str] = &[
    "#000000", "#0f0f0f", "#1e1e1e", "#2d2d2d", "#3b3b3b", "#494949", "#565656", "#636363",
    "#6f6f6f", "#7b7b7b", "#868686", "#919191", "#9b9b9b", "#a5a5a5", "#aeaeae", "#b7b7b7",
    "#bfbfbf", "#c6c6c6", "#cecece", "#d4d4d4", "#dbdbdb", "#e0e0e0", "#e6e6e6", "#eaeaea",
    "#efefef", "#f2f2f2", "#f6f6f6", "#f8f8f8", "#fbfbfb", "#fcfcfc", "#fefefe", "#fefefe",
];

const CIRCLE_SHADED_REMAIN_RATIO: f64 = 0.152_470_445_589_572_57;
const CIRCLE_SHADED_CENTER_SHIFT_RATIO: f64 = 0.484_377_144_287_654_77;
const ELLIPSE_SHADED_REMAIN_RATIO: f64 = 0.111_974_358_974_358_58;
const ELLIPSE_SHADED_CENTER_SHIFT_RATIO: f64 = 0.484_730_769_230_768_24;
const RECT_SHADED_INSET_RATIO: f64 = 0.058_648_052_902_278_19;
const ROUND_RECT_SHADED_INSET_RATIO: f64 = 0.127_129_977_460_556;
const RECT_SHADED_REMAIN_RATIO: f64 = 0.111_976_487_876_561_09;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_circled_charge_sign_matches_chemdraw_width() {
        let layout = charge_symbol_layout(crate::CdxmlSymbolStyle::Default);
        assert!((layout.circle_sign_size - 5.444).abs() < 1.0e-9);
        assert!((layout.sign_thickness - 0.8).abs() < 1.0e-9);
    }

    #[test]
    fn shadow_size_scales_with_outline_width() {
        let cases = [(0.6, 2.4), (1.0, 4.0), (2.0, 8.0)];
        for (stroke_width, expected_offset) in cases {
            let style = ShapeStyleSpec {
                fill: None,
                stroke: Some("#000000".to_string()),
                stroke_width,
                dash_array: Vec::new(),
                fill_gradient: None,
                render_style: ShapeRenderStyle::Shadowed,
                shadow_size: 4.0,
            };
            assert!((style.shadow_size * style.stroke_width - expected_offset).abs() < 1.0e-9);
        }
    }
}
