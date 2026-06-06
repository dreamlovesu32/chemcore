import {
  makeSvgNode,
  normalizeDisplayColor,
} from "./render_support.js";
import { renderCorePrimitive } from "./primitive_dom_renderer.js";

const SELECTION_RESIZE_MIN_SCALE = 0.05;

export function createEditorOverlayRenderer(options) {
  function formatTlcRfValue(rf) {
    return `Rf ${Number(rf || 0).toFixed(2)}`;
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
    const label = formatTlcRfValue(hit.rf);
    const labelX = hit.center.x + options.screenPxToWorld(10);
    const labelY = hit.center.y - options.screenPxToWorld(10);
    const paddingX = options.screenPxToWorld(6);
    const paddingY = options.screenPxToWorld(4);
    const labelWidth = Math.max(
      options.screenPxToWorld(44),
      options.screenPxToWorld(label.length * 7),
    );
    const labelHeight = options.screenPxToWorld(20);
    overlay.appendChild(makeSvgNode("rect", {
      x: labelX - paddingX,
      y: labelY - labelHeight + paddingY,
      width: labelWidth + paddingX * 2,
      height: labelHeight,
      rx: options.screenPxToWorld(4),
      ry: options.screenPxToWorld(4),
      class: "editor-selection-text-box",
      fill: "#ffffff",
      "data-role": "tlc-spot-rf-box",
    }));
    overlay.appendChild(makeSvgNode("text", {
      x: labelX,
      y: labelY,
      class: "editor-selection-rotate-angle",
      "data-role": "tlc-spot-rf-label",
    }));
    overlay.lastChild.textContent = label;
  }

  function currentSelectionRotateHandle(renderList = options.currentEditorRenderList()) {
    return selectionRotateHandles(renderList)[0] || null;
  }

  function selectionRotateHandles(renderList = options.currentEditorRenderList()) {
    return (renderList || [])
      .filter((primitive) => (
        primitive.role === "selection-rotate-handle"
        && (primitive.kind === "circle" || primitive.kind === "rect")
      ))
      .map(selectionRotateHandleFromPrimitive)
      .filter(Boolean);
  }

  function selectionRotateHandleFromPrimitive(primitive) {
    if (!primitive) {
      return null;
    }
    const bounds = options.currentRenderBounds("selection");
    if (!bounds) {
      return null;
    }
    if (primitive.kind === "circle" && primitive.center) {
      return {
        x: primitive.center.x,
        y: primitive.center.y,
        radius: Number(primitive.radius || 0),
        hitRadius: options.screenPxToWorld(10),
        bounds,
        primitive,
      };
    }
    return {
      x: primitive.x + primitive.width * 0.5,
      y: primitive.y + primitive.height * 0.5,
      radius: Math.max(primitive.width, primitive.height) * 0.5,
      hitRadius: options.screenPxToWorld(10),
      bounds,
      primitive,
    };
  }

  function selectionRotateHandleHit(point) {
    return selectionRotateHandles(options.currentEditorRenderList())
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
    const rawId = String(primitive.objectId || primitive.object_id || "");
    const name = resizeHandleShortName(rawId);
    if (!name) {
      return null;
    }
    return {
      name,
      cursor: resizeHandleCursor(name),
      x: primitive.x + primitive.width * 0.5,
      y: primitive.y + primitive.height * 0.5,
      size: primitive.width,
      hitRadius: options.screenPxToWorld(10),
      primitive,
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
    const visibleResizeHandles = selectionResizeHandles(primitives);
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
        if (!primitive.role?.startsWith("selection-")) {
          continue;
        }
        renderCorePrimitive(overlay, primitive, options.corePrimitiveRenderOptions());
      } else if (primitive.kind === "path" && primitive.d) {
        if (!primitive.role?.startsWith("selection-")) {
          continue;
        }
        renderCorePrimitive(overlay, primitive, options.corePrimitiveRenderOptions());
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
        if (primitive.role?.startsWith("selection-")) {
          if (primitive.role === "selection-resize-handle") {
            if (activeSelectionGesture || !visibleResizeHandles.some((handle) => handle.primitive === primitive)) {
              continue;
            }
          }
          renderCorePrimitive(overlay, primitive, options.corePrimitiveRenderOptions());
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
        if (primitive.role?.startsWith("selection-")) {
          renderCorePrimitive(overlay, primitive, options.corePrimitiveRenderOptions());
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
    if (editorState.activeTool === "select" && activeSelectionGesture?.kind === "resize") {
      const bounds = options.currentRenderBounds("selection") || activeSelectionGesture.bounds;
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
    } else if (editorState.activeTool === "select" && activeSelectionGesture?.kind === "rotate") {
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
      if (hit?.center) {
        const label = formatTlcRfValue(hit.rf);
        const labelX = hit.center.x + options.screenPxToWorld(10);
        const labelY = hit.center.y - options.screenPxToWorld(10);
        const paddingX = options.screenPxToWorld(6);
        const paddingY = options.screenPxToWorld(4);
        const labelWidth = Math.max(
          options.screenPxToWorld(44),
          options.screenPxToWorld(label.length * 7),
        );
        const labelHeight = options.screenPxToWorld(20);
        overlay.appendChild(makeSvgNode("rect", {
          x: labelX - paddingX,
          y: labelY - labelHeight + paddingY,
          width: labelWidth + paddingX * 2,
          height: labelHeight,
          rx: options.screenPxToWorld(4),
          ry: options.screenPxToWorld(4),
          class: "editor-selection-text-box",
          fill: "#ffffff",
          "data-role": "tlc-spot-rf-box",
        }));
        overlay.appendChild(makeSvgNode("text", {
          x: labelX,
          y: labelY,
          class: "editor-selection-rotate-angle",
          "data-role": "tlc-spot-rf-label",
        }));
        overlay.lastChild.textContent = label;
      }
    } else if ((editorState.activeTool === "select" || editorState.activeTool === "tlc-plate")
      && !activeSelectionGesture
      && options.activeTlcLaneHover()) {
      drawTlcSpotGuideOverlay(overlay, options.activeTlcLaneHover());
    }
    if (editorState.activeTool === "select" && activeSelectionGesture?.dragged) {
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
