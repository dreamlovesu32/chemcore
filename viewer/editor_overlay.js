import {
  makeSvgNode,
  normalizeDisplayColor,
} from "./render_support.js";
import { renderCorePrimitive } from "./primitive_dom_renderer.js";

const SELECTION_RESIZE_MIN_SCALE = 0.05;

export function createEditorOverlayRenderer(options) {
  function currentSelectionOverlayBehavior() {
    const info = options.getSelectionInfo();
    const onlySingleGraphic = info.graphicObjects.length === 1
      && info.textObjects.length === 0
      && info.nodes.length === 0
      && info.bonds.length === 0
      && info.labelNodes.length === 0;
    const base = {
      showResizeHandles: true,
      showRotateHandle: true,
      rotateHandleShape: "circle",
      showRotateGlyph: true,
      showCenterCross: false,
      useGlobalBoundsOnly: false,
    };
    if (!onlySingleGraphic) {
      return base;
    }
    const object = info.graphicObjects[0];
    const kind = object?.payload?.kind || "";
    if (object?.type === "line") {
      return {
        ...base,
        showResizeHandles: false,
        showRotateHandle: false,
        showRotateGlyph: false,
        useGlobalBoundsOnly: true,
      };
    }
    if (object?.type === "shape" && kind === "orbital") {
      return {
        ...base,
        showRotateHandle: false,
        showRotateGlyph: false,
        showCenterCross: true,
        useGlobalBoundsOnly: true,
      };
    }
    if (object?.type === "shape" && kind === "tlcPlate") {
      return {
        ...base,
        showResizeHandles: false,
        rotateHandleShape: "square",
        showRotateGlyph: false,
        showCenterCross: true,
        useGlobalBoundsOnly: true,
      };
    }
    if (object?.type === "shape" && kind === "crossTable") {
      return {
        ...base,
        showResizeHandles: false,
        showRotateHandle: false,
        showRotateGlyph: false,
        useGlobalBoundsOnly: true,
      };
    }
    return base;
  }

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

  function selectionRotateHandleFromBounds(bounds, behavior = currentSelectionOverlayBehavior()) {
    if (!bounds || behavior.showRotateHandle === false) {
      return null;
    }
    const radius = options.screenPxToWorld(5);
    return {
      x: (bounds.minX + bounds.maxX) * 0.5,
      y: bounds.minY - options.screenPxToWorld(18),
      radius,
      hitRadius: options.screenPxToWorld(10),
      bounds,
    };
  }

  function currentSelectionRotateHandle() {
    return selectionRotateHandleFromBounds(
      options.currentRenderBounds("selection"),
      currentSelectionOverlayBehavior(),
    );
  }

  function selectionBoxPrimitives(renderList = options.currentEditorRenderList()) {
    const selectionRoles = new Set(["selection-box", "selection-bond", "selection-node", "selection-text-box"]);
    return (renderList || []).filter((primitive) => (
      primitive.kind === "rect" && selectionRoles.has(primitive.role)
    ));
  }

  function selectionResizeHandlesForBounds(bounds) {
    if (!bounds) {
      return [];
    }
    const size = options.screenPxToWorld(8);
    const hitRadius = options.screenPxToWorld(10);
    const centerX = (bounds.minX + bounds.maxX) * 0.5;
    const centerY = (bounds.minY + bounds.maxY) * 0.5;
    return [
      { name: "nw", cursor: "nwse-resize", x: bounds.minX, y: bounds.minY, size, hitRadius },
      { name: "n", cursor: "ns-resize", x: centerX, y: bounds.minY, size, hitRadius },
      { name: "ne", cursor: "nesw-resize", x: bounds.maxX, y: bounds.minY, size, hitRadius },
      { name: "e", cursor: "ew-resize", x: bounds.maxX, y: centerY, size, hitRadius },
      { name: "se", cursor: "nwse-resize", x: bounds.maxX, y: bounds.maxY, size, hitRadius },
      { name: "s", cursor: "ns-resize", x: centerX, y: bounds.maxY, size, hitRadius },
      { name: "sw", cursor: "nesw-resize", x: bounds.minX, y: bounds.maxY, size, hitRadius },
      { name: "w", cursor: "ew-resize", x: bounds.minX, y: centerY, size, hitRadius },
    ];
  }

  function selectionResizeHandles(
    renderList = options.currentEditorRenderList(),
    behavior = currentSelectionOverlayBehavior(),
  ) {
    if (!behavior.showResizeHandles) {
      return [];
    }
    const handles = behavior.useGlobalBoundsOnly
      ? []
      : selectionBoxPrimitives(renderList).flatMap((primitive) => selectionResizeHandlesForBounds({
        minX: primitive.x,
        minY: primitive.y,
        maxX: primitive.x + primitive.width,
        maxY: primitive.y + primitive.height,
      }));
    const globalBounds = options.currentRenderBounds("selection");
    if (globalBounds) {
      handles.push(...selectionResizeHandlesForBounds(globalBounds).map((handle) => ({
        ...handle,
        global: true,
      })));
    }
    return handles;
  }

  function selectionResizeHandleHit(point) {
    return selectionResizeHandles(options.currentEditorRenderList(), currentSelectionOverlayBehavior())
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
        const globalPriority = Number(b.handle.global) - Number(a.handle.global);
        if (globalPriority) {
          return globalPriority;
        }
        return a.distance - b.distance;
      })[0]?.handle || null;
  }

  function selectionCenterCrossFromBounds(bounds) {
    if (!bounds) {
      return null;
    }
    const halfSize = options.screenPxToWorld(5);
    return {
      x: (bounds.minX + bounds.maxX) * 0.5,
      y: (bounds.minY + bounds.maxY) * 0.5,
      halfSize,
    };
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
        if (primitive.role !== "selection-bond") {
          continue;
        }
        overlay.appendChild(makeSvgNode("line", {
          x1: primitive.from.x,
          y1: primitive.from.y,
          x2: primitive.to.x,
          y2: primitive.to.y,
          class: "editor-selection-bond",
          "stroke-width": options.primitiveStrokeWidthValue(
            primitive,
            options.editorBondStrokeWidth(),
          ),
          "data-role": primitive.role,
        }));
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
          "selection-box": "editor-selection-box",
          "selection-bond": "editor-selection-bond-box",
          "selection-node": "editor-selection-node-box",
          "selection-text-box": "editor-selection-text-box",
        };
        const className = classByRole[primitive.role];
        if (!className) {
          continue;
        }
        const selectionRole = primitive.role?.startsWith("selection-");
        overlay.appendChild(makeSvgNode("rect", {
          x: primitive.x,
          y: primitive.y,
          width: primitive.width,
          height: primitive.height,
          class: className,
          fill: selectionRole ? "none" : undefined,
          "data-role": primitive.role,
        }));
      } else if (primitive.kind === "circle" && primitive.center) {
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
    const editorState = options.editorState();
    const activeSelectionGesture = options.activeSelectionGesture();
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
    } else if (editorState.activeTool === "select" && !activeSelectionGesture) {
      const selectionBehavior = currentSelectionOverlayBehavior();
      for (const handle of selectionResizeHandles(primitives, selectionBehavior)) {
        overlay.appendChild(makeSvgNode("rect", {
          x: handle.x - handle.size * 0.5,
          y: handle.y - handle.size * 0.5,
          width: handle.size,
          height: handle.size,
          class: "editor-selection-resize-handle",
          "data-role": `selection-resize-${handle.name}`,
        }));
      }
      const selectionBounds = options.currentRenderBounds("selection");
      if (selectionBehavior.showCenterCross) {
        const cross = selectionCenterCrossFromBounds(selectionBounds);
        if (cross) {
          overlay.appendChild(makeSvgNode("line", {
            x1: cross.x - cross.halfSize,
            y1: cross.y,
            x2: cross.x + cross.halfSize,
            y2: cross.y,
            class: "editor-selection-center-cross",
            "data-role": "selection-center-cross",
          }));
          overlay.appendChild(makeSvgNode("line", {
            x1: cross.x,
            y1: cross.y - cross.halfSize,
            x2: cross.x,
            y2: cross.y + cross.halfSize,
            class: "editor-selection-center-cross",
            "data-role": "selection-center-cross",
          }));
        }
      }
      const handle = selectionRotateHandleFromBounds(selectionBounds, selectionBehavior);
      if (handle) {
        const topCenter = {
          x: (handle.bounds.minX + handle.bounds.maxX) * 0.5,
          y: handle.bounds.minY,
        };
        overlay.appendChild(makeSvgNode("line", {
          x1: topCenter.x,
          y1: topCenter.y,
          x2: handle.x,
          y2: handle.y + handle.radius,
          class: "editor-selection-rotate-stem",
          "data-role": "selection-rotate-stem",
        }));
        if (selectionBehavior.rotateHandleShape === "square") {
          const size = handle.radius * 1.25;
          overlay.appendChild(makeSvgNode("rect", {
            x: handle.x - size * 0.5,
            y: handle.y - size * 0.5,
            width: size,
            height: size,
            class: "editor-selection-top-handle",
            "data-role": "selection-rotate-handle",
          }));
        } else {
          overlay.appendChild(makeSvgNode("circle", {
            cx: handle.x,
            cy: handle.y,
            r: handle.radius,
            class: "editor-selection-rotate-handle",
            "data-role": "selection-rotate-handle",
          }));
        }
        if (selectionBehavior.showRotateGlyph) {
          overlay.appendChild(makeSvgNode("path", {
            d: `M ${handle.x - handle.radius * 0.55} ${handle.y} A ${handle.radius * 0.55} ${handle.radius * 0.55} 0 1 1 ${handle.x + handle.radius * 0.35} ${handle.y + handle.radius * 0.42}`,
            class: "editor-selection-rotate-glyph",
            "data-role": "selection-rotate-glyph",
          }));
        }
      }
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
    currentSelectionOverlayBehavior,
    currentSelectionRotateHandle,
    selectionResizeHandleHit,
    selectionResizeGestureScale,
    selectionRotateAngleForGesture,
    renderEditorOverlay,
  };
}
