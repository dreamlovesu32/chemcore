use crate::*;

pub(crate) fn parse_render_bounds_scope(scope: &str) -> RenderBoundsScope {
    match scope {
        "document" => RenderBoundsScope::Document,
        "selection" => RenderBoundsScope::Selection,
        _ => RenderBoundsScope::All,
    }
}

pub(crate) fn bounds_json_for_snapshot(
    primitives: Option<&[RenderPrimitive]>,
    scope: RenderBoundsScope,
    include: bool,
) -> Result<Option<String>, String> {
    if !include {
        return Ok(None);
    }
    let bounds = primitives.and_then(|items| {
        chemcore_engine::render_primitives_bounds(
            items
                .iter()
                .filter(|primitive| render_bounds_scope_accepts(scope, primitive)),
        )
        .map(RenderBounds::from)
    });
    serde_json::to_string(&bounds)
        .map(Some)
        .map_err(|error| error.to_string())
}

pub(crate) fn selection_bounds_json_for_snapshot(
    engine: &Engine,
    include: bool,
) -> Result<Option<String>, String> {
    if !include {
        return Ok(None);
    }
    let bounds = engine.selection_bounds().map(RenderBounds::from);
    serde_json::to_string(&bounds)
        .map(Some)
        .map_err(|error| error.to_string())
}

pub(crate) fn render_bounds_scope_accepts(
    scope: RenderBoundsScope,
    primitive: &RenderPrimitive,
) -> bool {
    match scope {
        RenderBoundsScope::All => true,
        RenderBoundsScope::Document => {
            let role = render_primitive_role(primitive);
            role != RenderRole::DocumentKnockout
                && !render_role_is_selection(role)
                && !render_role_is_hover(role)
                && !render_role_is_preview(role)
        }
        RenderBoundsScope::Selection => {
            render_role_is_selection_bounds(render_primitive_role(primitive))
        }
    }
}

pub(crate) fn render_primitive_role(primitive: &RenderPrimitive) -> RenderRole {
    match primitive {
        RenderPrimitive::Line { role, .. }
        | RenderPrimitive::Circle { role, .. }
        | RenderPrimitive::Polygon { role, .. }
        | RenderPrimitive::Rect { role, .. }
        | RenderPrimitive::Ellipse { role, .. }
        | RenderPrimitive::Polyline { role, .. }
        | RenderPrimitive::Path { role, .. }
        | RenderPrimitive::FilledPath { role, .. }
        | RenderPrimitive::Text { role, .. } => *role,
    }
}

pub(crate) fn render_role_is_selection(role: RenderRole) -> bool {
    render_role_is_selection_bounds(role)
        || matches!(
            role,
            RenderRole::SelectionCenterCross
                | RenderRole::SelectionResizeHandle
                | RenderRole::SelectionRotateGlyph
                | RenderRole::SelectionRotateHandle
                | RenderRole::SelectionRotateStem
        )
}

pub(crate) fn render_role_is_selection_bounds(role: RenderRole) -> bool {
    matches!(
        role,
        RenderRole::SelectionBox
            | RenderRole::SelectionBond
            | RenderRole::SelectionBondDot
            | RenderRole::SelectionNode
            | RenderRole::SelectionTextBox
    )
}

pub(crate) fn render_role_is_hover(role: RenderRole) -> bool {
    matches!(
        role,
        RenderRole::HoverEndpoint
            | RenderRole::HoverLabelGlyph
            | RenderRole::HoverBondCenter
            | RenderRole::HoverArrowCenter
            | RenderRole::HoverArrowHandle
            | RenderRole::HoverShapeHandle
            | RenderRole::HoverTextBox
    )
}

pub(crate) fn render_role_is_preview(role: RenderRole) -> bool {
    matches!(role, RenderRole::PreviewBond | RenderRole::PreviewEnd)
}
