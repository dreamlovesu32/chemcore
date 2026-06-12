import {
  makeSvgNode,
  normalizeDisplayColor,
} from "./render_support.js";
import { renderCorePrimitive } from "./primitive_dom_renderer.js";

const SELECTION_RESIZE_MIN_SCALE = 0.05;
const SELECTION_STROKE_SCREEN_PX = 1;
const SELECTION_RESIZE_HANDLE_SCREEN_PX = 1.5;
const SELECTION_ROTATE_HANDLE_RADIUS_SCREEN_PX = 2.0;
const SELECTION_ROTATE_HANDLE_OFFSET_SCREEN_PX = 18;
const SELECTION_CENTER_CROSS_HALF_SCREEN_PX = 5;

export function createEditorOverlayRenderer(options) {
  function appendTlcRfLabel(overlay, hit) {
    if (!hit?.center) {
      return;
    }
    const rfValue = Number(hit.rf || 0).toFixed(2).replace(/\.?0+$/, "");
    const labelX = hit.center.x + options.screenPxToWorld(10);
    const labelY = hit.center.y - options.screenPxToWorld(10);
    const paddingX = options.screenPxToWorld(3);
    const paddingY = options.screenPxToWorld(2);
    const labelWidth = options.screenPxToWorld(30);
    const labelHeight = options.screenPxToWorld(10);
    overlay.appendChild(makeSvgNode("rect", {
      x: labelX - paddingX,
      y: labelY - labelHeight + paddingY,
      width: labelWidth + paddingX * 2,
      height: labelHeight,
      rx: options.screenPxToWorld(3),
      ry: options.screenPxToWorld(3),
      class: "tlc-spot-rf-box",
      fill: "#ffffff",
      "data-role": "tlc-spot-rf-box",
    }));
    const text = makeSvgNode("text", {
      x: labelX,
      y: labelY,
      class: "tlc-spot-rf-label",
      "data-role": "tlc-spot-rf-label",
    });
    text.appendChild(makeSvgNode("tspan", {})).textContent = "R";
    text.appendChild(makeSvgNode("tspan", {
      class: "tlc-spot-rf-subscript",
    })).textContent = "f";
    text.appendChild(makeSvgNode("tspan", {
      dx: options.screenPxToWorld(2),
    })).textContent = ` = ${rfValue}`;
    overlay.appendChild(text);
  }

  function tlcSpotSupportsOverlay(hit) {
    return Array.isArray(hit?.guidePoints) && hit.guidePoints.length >= 4;
  }

  function drawTlcSpotGuideOverlay(overlay, hit, { showLabel = false } = {}) {
    if (!tlcSpotSupportsOverlay(hit)) {
      return;
    }
    overlay.appendChild(makeSvgNode("polygon", {
      points: hit.guidePoints.map((point) => `${point.x},${point.y}`).join(" "),
      class: "editor-selection-box",
      fill: "none",
      "data-role": showLabel ? "tlc-spot-drag-guide" : "tlc-spot-hover-guide",
    }));
    if (!showLabel || !hit.center) {
      return;
    }
    appendTlcRfLabel(overlay, hit);
  }

  function currentSelectionRotateHandle(renderList = options.currentEditorRenderList()) {
    const selectionBounds = options.currentRenderBounds("selection");
    return selectionRotateHandles(renderList, selectionBounds)[0] || null;
  }

  function normalizeSelectionPrimitiveForViewport(primitive, selectionBounds = null) {
    if (!primitive?.role?.startsWith("selection-")) {
      return primitive;
    }
    const strokeWidth = options.screenPxToWorld(SELECTION_STROKE_SCREEN_PX);
    if (primitive.role === "selection-resize-handle" && primitive.kind === "rect") {
      const size = options.screenPxToWorld(SELECTION_RESIZE_HANDLE_SCREEN_PX);
      const centerX = primitive.x + primitive.width * 0.5;
      const centerY = primitive.y + primitive.height * 0.5;
      return {
        ...primitive,
        x: centerX - size * 0.5,
        y: centerY - size * 0.5,
        width: size,
        height: size,
        strokeWidth: 0,
        stroke_width: undefined,
      };
    }

    if (primitive.role === "selection-center-cross" && primitive.kind === "line") {
      const half = options.screenPxToWorld(SELECTION_CENTER_CROSS_HALF_SCREEN_PX);
      const center = {
        x: (primitive.from.x + primitive.to.x) * 0.5,
        y: (primitive.from.y + primitive.to.y) * 0.5,
      };
      const horizontal = Math.abs(primitive.to.x - primitive.from.x) >= Math.abs(primitive.to.y - primitive.from.y);
      return {
        ...primitive,
        from: horizontal ? { x: center.x - half, y: center.y } : { x: center.x, y: center.y - half },
        to: horizontal ? { x: center.x + half, y: center.y } : { x: center.x, y: center.y + half },
        strokeWidth,
        stroke_width: undefined,
      };
    }

    if (primitive.role?.startsWith("selection-rotate-")) {
      return normalizeSelectionRotatePrimitiveForViewport(primitive, strokeWidth, selectionBounds);
    }

    if (
      primitive.kind === "line"
      || primitive.kind === "path"
      || primitive.kind === "rect"
      || primitive.kind === "circle"
      || primitive.kind === "ellipse"
    ) {
      return {
        ...primitive,
        strokeWidth,
        stroke_width: undefined,
      };
    }
    return primitive;
  }

  function normalizedSelectionRotateGeometry(selectionBounds = null) {
    const bounds = selectionBounds || options.currentRenderBounds("selection");
    if (!bounds) {
      return null;
    }
    const radius = options.screenPxToWorld(SELECTION_ROTATE_HANDLE_RADIUS_SCREEN_PX);
    const offset = options.screenPxToWorld(SELECTION_ROTATE_HANDLE_OFFSET_SCREEN_PX);
    const centerX = (bounds.minX + bounds.maxX) * 0.5;
    return {
      bounds,
      radius,
      handle: {
        x: centerX,
        y: bounds.minY - offset,
      },
      topCenter: {
        x: centerX,
        y: bounds.minY,
      },
    };
  }

  function normalizeSelectionRotatePrimitiveForViewport(primitive, strokeWidth, selectionBounds = null) {
    const geometry = normalizedSelectionRotateGeometry(selectionBounds);
    if (!geometry) {
      return primitive;
    }
    const { handle, radius, topCenter } = geometry;
    if (primitive.role === "selection-rotate-stem" && primitive.kind === "line") {
      return {
        ...primitive,
        from: topCenter,
        to: { x: handle.x, y: handle.y + radius },
        strokeWidth,
        stroke_width: undefined,
      };
    }
    if (primitive.role === "selection-rotate-handle" && primitive.kind === "circle") {
      return {
        ...primitive,
        center: handle,
        radius,
        strokeWidth,
        stroke_width: undefined,
      };
    }
    if (primitive.role === "selection-rotate-handle" && primitive.kind === "rect") {
      const size = radius * 2;
      return {
        ...primitive,
        x: handle.x - size * 0.5,
        y: handle.y - size * 0.5,
        width: size,
        height: size,
        strokeWidth,
        stroke_width: undefined,
      };
    }
    if (primitive.role === "selection-rotate-glyph" && primitive.kind === "path") {
      return {
        ...primitive,
        d: [
          `M ${handle.x - radius * 0.55} ${handle.y}`,
          `A ${radius * 0.55} ${radius * 0.55} 0 1 1 ${handle.x + radius * 0.35} ${handle.y + radius * 0.42}`,
        ].join(" "),
        strokeWidth,
        stroke_width: undefined,
      };
    }
    return primitive;
  }

  function selectionRotateHandles(renderList = options.currentEditorRenderList(), selectionBounds = null) {
    return (renderList || [])
      .filter((primitive) => (
        primitive.role === "selection-rotate-handle"
        && (primitive.kind === "circle" || primitive.kind === "rect")
      ))
      .map((primitive) => selectionRotateHandleFromPrimitive(primitive, selectionBounds))
      .filter(Boolean);
  }

  function selectionRotateHandleFromPrimitive(primitive, selectionBounds = null) {
    const normalized = normalizeSelectionPrimitiveForViewport(primitive, selectionBounds);
    if (!normalized) {
      return null;
    }
    const bounds = selectionBounds || options.currentRenderBounds("selection");
    if (!bounds) {
      return null;
    }
    if (normalized.kind === "circle" && normalized.center) {
      return {
        x: normalized.center.x,
        y: normalized.center.y,
        radius: Number(normalized.radius || 0),
        hitRadius: options.screenPxToWorld(10),
        bounds,
        primitive: normalized,
        sourcePrimitive: primitive,
      };
    }
    return {
      x: normalized.x + normalized.width * 0.5,
      y: normalized.y + normalized.height * 0.5,
      radius: Math.max(normalized.width, normalized.height) * 0.5,
      hitRadius: options.screenPxToWorld(10),
      bounds,
      primitive: normalized,
      sourcePrimitive: primitive,
    };
  }

  function selectionRotateHandleHit(point) {
    const selectionBounds = options.currentRenderBounds("selection");
    return selectionRotateHandles(options.currentEditorRenderList(), selectionBounds)
      .map((handle) => ({
        handle,
        distance: options.pointDistance(point, handle),
      }))
      .filter((entry) => entry.distance <= entry.handle.hitRadius)
      .sort((a, b) => a.distance - b.distance)[0]?.handle || null;
  }

  function selectionResizeHandles(renderList = options.currentEditorRenderList()) {
    return (renderList || [])
      .filter((primitive) => primitive.kind === "rect" && primitive.role === "selection-resize-handle")
      .map(selectionResizeHandleFromPrimitive)
      .filter(Boolean);
  }

  function selectionResizeHandleFromPrimitive(primitive) {
    const normalized = normalizeSelectionPrimitiveForViewport(primitive);
    const rawId = String(primitive.objectId || primitive.object_id || "");
    const name = resizeHandleShortName(rawId);
    if (!name) {
      return null;
    }
    return {
      name,
      cursor: resizeHandleCursor(name),
      x: normalized.x + normalized.width * 0.5,
      y: normalized.y + normalized.height * 0.5,
      size: normalized.width,
      hitRadius: options.screenPxToWorld(10),
      primitive: normalized,
      sourcePrimitive: primitive,
    };
  }

  function resizeHandleShortName(name) {
    switch (String(name || "").toLowerCase().replace(/[_-]/g, "")) {
      case "n":
      case "north":
        return "n";
      case "s":
      case "south":
        return "s";
      case "e":
      case "east":
        return "e";
      case "w":
      case "west":
        return "w";
      case "ne":
      case "northeast":
        return "ne";
      case "nw":
      case "northwest":
        return "nw";
      case "se":
      case "southeast":
        return "se";
      case "sw":
      case "southwest":
        return "sw";
      default:
        return "";
    }
  }

  function resizeHandleCursor(name) {
    switch (name) {
      case "n":
      case "s":
        return "ns-resize";
      case "e":
      case "w":
        return "ew-resize";
      case "ne":
      case "sw":
        return "nesw-resize";
      case "nw":
      case "se":
        return "nwse-resize";
      default:
        return "default";
    }
  }

  function selectionResizeHandleHit(point) {
    return selectionResizeHandles(options.currentEditorRenderList())
      .map((handle) => {
        const dx = Math.abs(point.x - handle.x);
        const dy = Math.abs(point.y - handle.y);
        const squareHit = dx <= handle.hitRadius && dy <= handle.hitRadius;
        const distance = options.pointDistance(point, handle);
        return { handle, distance, squareHit };
      })
      .filter((entry) => entry.squareHit || entry.distance <= entry.handle.hitRadius)
      .sort((a, b) => {
        const cornerPriority = Number(b.handle.name.length === 2) - Number(a.handle.name.length === 2);
        if (cornerPriority) {
          return cornerPriority;
        }
        return a.distance - b.distance;
      })[0]?.handle || null;
  }

  function selectionResizePivot(handleName, bounds) {
    const centerX = (bounds.minX + bounds.maxX) * 0.5;
    const centerY = (bounds.minY + bounds.maxY) * 0.5;
    switch (handleName) {
      case "n": return { x: centerX, y: bounds.maxY };
      case "s": return { x: centerX, y: bounds.minY };
      case "e": return { x: bounds.minX, y: centerY };
      case "w": return { x: bounds.maxX, y: centerY };
      case "ne": return { x: bounds.minX, y: bounds.maxY };
      case "nw": return { x: bounds.maxX, y: bounds.maxY };
      case "se": return { x: bounds.minX, y: bounds.minY };
      case "sw": return { x: bounds.maxX, y: bounds.minY };
      default: return { x: centerX, y: centerY };
    }
  }

  function selectionResizeHandlePoint(handleName, bounds) {
    const centerX = (bounds.minX + bounds.maxX) * 0.5;
    const centerY = (bounds.minY + bounds.maxY) * 0.5;
    switch (handleName) {
      case "n": return { x: centerX, y: bounds.minY };
      case "s": return { x: centerX, y: bounds.maxY };
      case "e": return { x: bounds.maxX, y: centerY };
      case "w": return { x: bounds.minX, y: centerY };
      case "ne": return { x: bounds.maxX, y: bounds.minY };
      case "nw": return { x: bounds.minX, y: bounds.minY };
      case "se": return { x: bounds.maxX, y: bounds.maxY };
      case "sw": return { x: bounds.minX, y: bounds.maxY };
      default: return { x: centerX, y: centerY };
    }
  }

  function selectionResizeGestureScale(gesture, point) {
    const bounds = gesture?.bounds;
    const handle = gesture?.handle;
    if (!bounds || !handle) {
      return 1;
    }
    const width = Math.max(Number.EPSILON, bounds.maxX - bounds.minX);
    const height = Math.max(Number.EPSILON, bounds.maxY - bounds.minY);
    if (handle.length === 2) {
      const pivot = selectionResizePivot(handle, bounds);
      const original = selectionResizeHandlePoint(handle, bounds);
      const dx = original.x - pivot.x;
      const dy = original.y - pivot.y;
      const denominator = dx * dx + dy * dy;
      if (denominator <= Number.EPSILON) {
        return 1;
      }
      return Math.max(
        SELECTION_RESIZE_MIN_SCALE,
        ((point.x - pivot.x) * dx + (point.y - pivot.y) * dy) / denominator,
      );
    }
    if (handle === "e") {
      return Math.max(SELECTION_RESIZE_MIN_SCALE, (point.x - bounds.minX) / width);
    }
    if (handle === "w") {
      return Math.max(SELECTION_RESIZE_MIN_SCALE, (bounds.maxX - point.x) / width);
    }
    if (handle === "s") {
      return Math.max(SELECTION_RESIZE_MIN_SCALE, (point.y - bounds.minY) / height);
    }
    if (handle === "n") {
      return Math.max(SELECTION_RESIZE_MIN_SCALE, (bounds.maxY - point.y) / height);
    }
    return 1;
  }

  function formatResizeScale(scale) {
    return `${(scale * 100).toFixed(1)}%`;
  }

  function signedAngleDelta(start, end) {
    let delta = ((end - start) % 360 + 360) % 360;
    if (delta > 180) {
      delta -= 360;
    }
    return delta;
  }

  function angleBetweenPoints(from, to) {
    const raw = Math.atan2(to.y - from.y, to.x - from.x) * 180 / Math.PI;
    return ((raw % 360) + 360) % 360;
  }

  function selectionRotateAngleForGesture(gesture, point, altKey) {
    if (!gesture?.center) {
      return 0;
    }
    const raw = signedAngleDelta(gesture.startAngle, angleBetweenPoints(gesture.center, point));
    return altKey ? raw : Math.round(raw / 15) * 15;
  }

  function formatRotationAngle(angle) {
    const rounded = Math.round(angle);
    return `${rounded}${String.fromCharCode(176)}`;
  }

  function renderEditorOverlay(renderList = null) {
    const viewerSvg = options.viewerSvg();
    viewerSvg?.querySelector('[data-layer="editor-overlay"]')?.remove();
    if (!options.isEditingRustDocument()) {
      return;
    }
    const primitives = renderList || options.currentEditorRenderList();
    const overlay = makeSvgNode("g", { "data-layer": "editor-overlay", "pointer-events": "none" });
    const previewTransform = options.activeDocumentPreviewTransform();
    if (previewTransform) {
      overlay.setAttribute("transform", previewTransform);
    }
    const previewActive = options.activeGestureUsesDocumentPreview()
      || primitives.some((primitive) => primitive.role === "preview-end");
    const editorState = options.editorState();
    const activeSelectionGesture = options.activeSelectionGesture();
    const hasSelectionOverlayPrimitives = primitives
      .some((primitive) => String(primitive?.role || "").startsWith("selection-"));
    const selectionBounds = hasSelectionOverlayPrimitives
      ? options.currentRenderBounds("selection")
      : null;
    const hideSelectionOverlayDuringGesture = ["move", "resize", "rotate"]
      .includes(activeSelectionGesture?.kind);
    const visibleResizeHandles = hideSelectionOverlayDuringGesture
      ? []
      : selectionResizeHandles(primitives);
    if (previewActive) {
      const viewBox = options.activeViewBox();
      const pageBackground = normalizeDisplayColor(
        options.currentPageBackground(),
        options.defaultPageBackground(),
      );
      overlay.appendChild(makeSvgNode("rect", {
        x: viewBox.x,
        y: viewBox.y,
        width: viewBox.width,
        height: viewBox.height,
        fill: pageBackground,
        "data-role": "preview-document-mask",
      }));
    }
    for (const primitive of primitives) {
      if (options.shouldHidePrimitiveForActiveEndpointEditor(primitive)) {
        continue;
      }
      if (options.isDocumentPreviewPrimitive(primitive)) {
        if (previewActive) {
          renderCorePrimitive(overlay, primitive, options.corePrimitiveRenderOptions());
        }
        continue;
      }
      if (primitive.kind === "line" && primitive.from && primitive.to) {
        if (hideSelectionOverlayDuringGesture && primitive.role?.startsWith("selection-")) {
          continue;
        }
        if (!primitive.role?.startsWith("selection-")) {
          continue;
        }
        renderCorePrimitive(overlay, normalizeSelectionPrimitiveForViewport(primitive, selectionBounds), options.corePrimitiveRenderOptions());
      } else if (primitive.kind === "path" && primitive.d) {
        if (hideSelectionOverlayDuringGesture && primitive.role?.startsWith("selection-")) {
          continue;
        }
        if (!primitive.role?.startsWith("selection-")) {
          continue;
        }
        renderCorePrimitive(overlay, normalizeSelectionPrimitiveForViewport(primitive, selectionBounds), options.corePrimitiveRenderOptions());
      } else if (primitive.kind === "polygon" && Array.isArray(primitive.points)) {
        const className = primitive.role === "hover-bond-center" ? "editor-bond-center-rect" : "";
        if (!className) {
          continue;
        }
        overlay.appendChild(makeSvgNode("polygon", {
          points: primitive.points.map((point) => `${point.x},${point.y}`).join(" "),
          class: className,
          "data-role": primitive.role,
        }));
      } else if (primitive.kind === "rect") {
        const classByRole = {
          "hover-text-box": "editor-text-box-focus",
          "hover-label-glyph": "editor-label-glyph-focus",
          "hover-arrow-handle": "editor-arrow-focus-handle",
        };
        if (hideSelectionOverlayDuringGesture && primitive.role?.startsWith("selection-")) {
          continue;
        }
        if (primitive.role?.startsWith("selection-")) {
          if (primitive.role === "selection-resize-handle") {
            if (activeSelectionGesture || !visibleResizeHandles.some((handle) => handle.sourcePrimitive === primitive)) {
              continue;
            }
          }
          renderCorePrimitive(overlay, normalizeSelectionPrimitiveForViewport(primitive, selectionBounds), options.corePrimitiveRenderOptions());
          continue;
        }
        const className = classByRole[primitive.role];
        if (!className) {
          continue;
        }
        overlay.appendChild(makeSvgNode("rect", {
          x: primitive.x,
          y: primitive.y,
          width: primitive.width,
          height: primitive.height,
          class: className,
          "data-role": primitive.role,
        }));
      } else if (primitive.kind === "circle" && primitive.center) {
        if (hideSelectionOverlayDuringGesture && primitive.role?.startsWith("selection-")) {
          continue;
        }
        if (primitive.role?.startsWith("selection-")) {
          renderCorePrimitive(overlay, normalizeSelectionPrimitiveForViewport(primitive, selectionBounds), options.corePrimitiveRenderOptions());
          continue;
        }
        const classByRole = {
          "hover-endpoint": "editor-endpoint-halo",
          "hover-bond-center": "editor-bond-center-halo",
          "hover-arrow-center": "editor-arrow-center-halo",
          "hover-arrow-handle": "editor-arrow-focus-handle",
          "hover-shape-handle": "editor-arrow-focus-handle",
          "preview-end": "editor-preview-end",
          "selection-bond-dot": "editor-selection-bond-dot",
        };
        const className = classByRole[primitive.role];
        if (!className) {
          continue;
        }
        overlay.appendChild(makeSvgNode("circle", {
          cx: primitive.center.x,
          cy: primitive.center.y,
          r: primitive.radius,
          class: className,
          "data-role": primitive.role,
        }));
      }
    }
    if (!hideSelectionOverlayDuringGesture && editorState.activeTool === "select" && activeSelectionGesture?.kind === "resize") {
      const bounds = selectionBounds || activeSelectionGesture.bounds;
      if (bounds) {
        const labelOffset = options.screenPxToWorld(8);
        overlay.appendChild(makeSvgNode("text", {
          x: bounds.maxX + labelOffset,
          y: bounds.minY - labelOffset,
          class: "editor-selection-resize-label",
          "data-role": "selection-resize-scale",
        }));
        overlay.lastChild.textContent = formatResizeScale(activeSelectionGesture.scale || 1);
      }
    } else if (!hideSelectionOverlayDuringGesture && editorState.activeTool === "select" && activeSelectionGesture?.kind === "rotate") {
      const bounds = activeSelectionGesture.bounds;
      const labelOffset = options.screenPxToWorld(8);
      overlay.appendChild(makeSvgNode("text", {
        x: bounds.maxX + labelOffset,
        y: bounds.minY - labelOffset,
        class: "editor-selection-rotate-angle",
        "data-role": "selection-rotate-angle",
      }));
      overlay.lastChild.textContent = formatRotationAngle(activeSelectionGesture.angle || 0);
    } else if ((editorState.activeTool === "select" || editorState.activeTool === "arrow")
      && activeSelectionGesture?.kind === "arrow-curve") {
      const labelOffset = options.screenPxToWorld(8);
      const point = activeSelectionGesture.current || activeSelectionGesture.start;
      overlay.appendChild(makeSvgNode("text", {
        x: point.x + labelOffset,
        y: point.y - labelOffset,
        class: "editor-selection-rotate-angle",
        "data-role": "arrow-curve-angle",
      }));
      overlay.lastChild.textContent = formatRotationAngle(activeSelectionGesture.angle || 0);
    } else if ((editorState.activeTool === "select" || editorState.activeTool === "tlc-plate")
      && activeSelectionGesture?.kind === "tlc-spot-drag") {
      const hit = activeSelectionGesture.hit;
      appendTlcRfLabel(overlay, hit);
    } else if ((editorState.activeTool === "select" || editorState.activeTool === "tlc-plate")
      && !activeSelectionGesture
      && options.activeTlcLaneHover()) {
      drawTlcSpotGuideOverlay(overlay, options.activeTlcLaneHover());
    }
    if (editorState.activeTool === "select"
      && activeSelectionGesture?.kind === "select"
      && activeSelectionGesture?.dragged) {
      if (editorState.selectMode === "box") {
        const start = activeSelectionGesture.start;
        const current = activeSelectionGesture.current;
        overlay.appendChild(makeSvgNode("rect", {
          x: Math.min(start.x, current.x),
          y: Math.min(start.y, current.y),
          width: Math.abs(current.x - start.x),
          height: Math.abs(current.y - start.y),
          class: "editor-selection-marquee",
          "data-role": "selection-marquee",
        }));
      } else {
        const points = activeSelectionGesture.points
          .concat([activeSelectionGesture.current])
          .map((candidate) => `${candidate.x},${candidate.y}`)
          .join(" ");
        overlay.appendChild(makeSvgNode("polyline", {
          points,
          class: "editor-selection-lasso",
          "data-role": "selection-lasso",
        }));
      }
    }
    if (overlay.childNodes.length) {
      viewerSvg.appendChild(overlay);
    }
  }

  return {
    currentSelectionRotateHandle,
    selectionRotateHandleHit,
    selectionResizeHandleHit,
    selectionResizeGestureScale,
    selectionRotateAngleForGesture,
    renderEditorOverlay,
  };
}
