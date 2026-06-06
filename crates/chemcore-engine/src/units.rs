/// CSS layout unit used by browser DOM APIs and SVG viewport math.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct CssPx(pub f64);

impl CssPx {
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> f64 {
        self.0
    }

    pub const fn to_world_pt(self) -> WorldPt {
        WorldPt(self.0 * CSS_PX_TO_PT)
    }
}

/// Canonical internal geometry unit for the Rust engine.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct WorldPt(pub f64);

impl WorldPt {
    pub const fn new(value: f64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> f64 {
        self.0
    }

    pub const fn to_css_px(self) -> CssPx {
        CssPx(self.0 * PT_TO_CSS_PX)
    }
}

/// Name the internal world-space unit explicitly at boundary code.
pub type WorldUnit = WorldPt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldPoint {
    pub x: WorldPt,
    pub y: WorldPt,
}

impl WorldPoint {
    pub const fn new(x: WorldPt, y: WorldPt) -> Self {
        Self { x, y }
    }

    pub const fn values(self) -> (f64, f64) {
        (self.x.value(), self.y.value())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CssPxPoint {
    pub x: CssPx,
    pub y: CssPx,
}

impl CssPxPoint {
    pub const fn new(x: CssPx, y: CssPx) -> Self {
        Self { x, y }
    }

    pub const fn to_world_point(self) -> WorldPoint {
        WorldPoint::new(self.x.to_world_pt(), self.y.to_world_pt())
    }
}

pub const fn css_px(value: f64) -> CssPx {
    CssPx::new(value)
}

pub const fn world_pt(value: f64) -> WorldPt {
    WorldPt::new(value)
}

impl From<CssPx> for WorldPt {
    fn from(value: CssPx) -> Self {
        value.to_world_pt()
    }
}

impl From<WorldPt> for CssPx {
    fn from(value: WorldPt) -> Self {
        value.to_css_px()
    }
}

// Browser layout APIs use CSS pixels, not physical device pixels. A 150%
// scaled display is typically 144 device px/in, but still 96 CSS px/in.
pub const CSS_PX_PER_INCH: f64 = 96.0;
pub const PT_PER_INCH: f64 = 72.0;
pub const CM_PER_INCH: f64 = 2.54;
pub const PT_PER_CM: f64 = PT_PER_INCH / CM_PER_INCH;
pub const CM_PER_PT: f64 = CM_PER_INCH / PT_PER_INCH;

pub const PT_TO_CSS_PX: f64 = CSS_PX_PER_INCH / PT_PER_INCH;
pub const CSS_PX_TO_PT: f64 = PT_PER_INCH / CSS_PX_PER_INCH;

pub const fn pt_to_css_px(pt: f64) -> f64 {
    world_pt(pt).to_css_px().value()
}

pub const fn css_px_to_pt(px: f64) -> f64 {
    css_px(px).to_world_pt().value()
}

pub const fn pt_to_px(pt: f64) -> f64 {
    pt_to_css_px(pt)
}

pub const fn px_to_pt(px: f64) -> f64 {
    css_px_to_pt(px)
}

pub const DEFAULT_PAGE_WIDTH_PT: f64 = 900.0;
pub const DEFAULT_PAGE_HEIGHT_PT: f64 = 600.0;
pub const DEFAULT_BOND_LENGTH_PT: f64 = 30.0;
pub const DEFAULT_BOND_STROKE_PT: f64 = 1.0;
pub const DEFAULT_TEXT_FONT_SIZE_PT: f64 = 10.0;
pub const DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT: f64 = 10.0;
pub const DEFAULT_TEXT_LINE_HEIGHT_PT: f64 = DEFAULT_TEXT_FONT_SIZE_PT * 1.2;
pub const DEFAULT_CENTERED_LABEL_FONT_SIZE_PT: f64 = DEFAULT_MOLECULE_LABEL_FONT_SIZE_PT;
pub const DEFAULT_TEXT_BLOCK_LINE_HEIGHT_PT: f64 = 11.25;
pub const DEFAULT_TEXT_BLOCK_PADDING_PT: f64 = px_to_pt(8.0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_px_and_world_pt_round_trip() {
        let px = css_px(12.0);
        let world = px.to_world_pt();
        assert!((world.value() - px_to_pt(12.0)).abs() < 1.0e-9);
        assert!((world.to_css_px().value() - 12.0).abs() < 1.0e-9);
    }

    #[test]
    fn css_px_point_converts_per_axis() {
        let point = CssPxPoint::new(css_px(96.0), css_px(48.0)).to_world_point();
        assert!((point.x.value() - 72.0).abs() < 1.0e-9);
        assert!((point.y.value() - 36.0).abs() < 1.0e-9);
    }
}
