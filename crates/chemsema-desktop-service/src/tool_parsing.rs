use crate::*;

pub(crate) fn point(x: f64, y: f64) -> Point {
    Point::from_world(WorldPoint::new(WorldPt(x), WorldPt(y)))
}

pub(crate) fn pointer_event(x: f64, y: f64, button: Option<u8>, alt_key: bool) -> PointerEvent {
    PointerEvent::from_world_point(WorldPoint::new(WorldPt(x), WorldPt(y)), button, alt_key)
}

pub(crate) fn parse_tool(value: &str) -> Tool {
    match value {
        "bond" => Tool::Bond,
        "arrow" => Tool::Arrow,
        "bracket" => Tool::Bracket,
        "symbol" => Tool::Symbol,
        "element" => Tool::Element,
        "delete" => Tool::Delete,
        "text" => Tool::Text,
        "shape" => Tool::Shape,
        "tlc-plate" | "tlcPlate" => Tool::TlcPlate,
        "orbital" => Tool::Orbital,
        "templates" => Tool::Templates,
        _ => Tool::Select,
    }
}

pub(crate) fn parse_bracket_kind(value: &str) -> BracketKind {
    parse_bracket_tool_value(value)
}

pub(crate) fn parse_arrow_variant(value: &str) -> ArrowVariant {
    match value {
        "curved" => ArrowVariant::Curved,
        "curved-mirror" => ArrowVariant::CurvedMirror,
        "hollow" => ArrowVariant::Hollow,
        "open" => ArrowVariant::Open,
        "equilibrium" => ArrowVariant::Equilibrium,
        _ => ArrowVariant::Solid,
    }
}

pub(crate) fn parse_shape_kind(value: &str) -> ShapeKind {
    match value {
        "ellipse" => ShapeKind::Ellipse,
        "round-rect" | "roundRect" => ShapeKind::RoundRect,
        "rect" => ShapeKind::Rect,
        "tlc-plate" | "tlcPlate" => ShapeKind::TlcPlate,
        _ => ShapeKind::Circle,
    }
}

pub(crate) fn parse_shape_style(value: &str) -> ShapeStyle {
    match value {
        "dashed" => ShapeStyle::Dashed,
        "shaded" => ShapeStyle::Shaded,
        "filled" => ShapeStyle::Filled,
        "shadowed" | "shadow" => ShapeStyle::Shadowed,
        _ => ShapeStyle::Solid,
    }
}

pub(crate) fn parse_orbital_template(value: &str) -> OrbitalTemplate {
    match value {
        "p" => OrbitalTemplate::P,
        "dxy" => OrbitalTemplate::Dxy,
        "oval" => OrbitalTemplate::Oval,
        "hybrid" => OrbitalTemplate::Hybrid,
        "dz2" => OrbitalTemplate::Dz2,
        "lobe" => OrbitalTemplate::Lobe,
        _ => OrbitalTemplate::S,
    }
}

pub(crate) fn parse_orbital_style(value: &str) -> OrbitalStyle {
    match value {
        "filled" => OrbitalStyle::Filled,
        "shaded" => OrbitalStyle::Shaded,
        _ => OrbitalStyle::Hollow,
    }
}

pub(crate) fn parse_orbital_phase(value: &str) -> OrbitalPhase {
    match value {
        "minus" => OrbitalPhase::Minus,
        _ => OrbitalPhase::Plus,
    }
}

pub(crate) fn parse_arrow_curve(value: &str) -> ArrowCurve {
    match value {
        "180" | "arc-180" | "arc180" => ArrowCurve::Arc180,
        "120" | "arc-120" | "arc120" => ArrowCurve::Arc120,
        "90" | "arc-90" | "arc90" => ArrowCurve::Arc90,
        _ => ArrowCurve::Arc270,
    }
}

pub(crate) fn parse_arrow_head_size(value: &str) -> ArrowHeadSize {
    match value {
        "large" => ArrowHeadSize::Large,
        "medium" => ArrowHeadSize::Medium,
        "small" => ArrowHeadSize::Small,
        _ => ArrowHeadSize::Small,
    }
}

pub(crate) fn parse_arrow_endpoint_style(value: &str) -> ArrowEndpointStyle {
    match value {
        "full" => ArrowEndpointStyle::Full,
        "left" | "top" | "half-left" => ArrowEndpointStyle::Left,
        "right" | "bottom" | "half-right" => ArrowEndpointStyle::Right,
        _ => ArrowEndpointStyle::None,
    }
}

pub(crate) fn parse_arrow_no_go(value: &str) -> ArrowNoGo {
    match value {
        "cross" => ArrowNoGo::Cross,
        "hash" => ArrowNoGo::Hash,
        _ => ArrowNoGo::None,
    }
}

pub(crate) fn parse_bond_variant(value: &str) -> BondVariant {
    parse_bond_tool_value(value)
}
