export const uiStyleProperties = Object.freeze([
  "backgroundColor",
  "borderColor",
  "boxShadow",
  "cursor",
  "display",
  "fill",
  "opacity",
  "outlineColor",
  "outlineStyle",
  "outlineWidth",
  "pointerEvents",
  "stroke",
  "strokeWidth",
  "visibility",
]);

function compareCount(actual, expected, label, failures) {
  if (expected !== undefined && actual !== expected) failures.push(`${label} is ${actual}; expected ${expected}.`);
}

function unionContains(container, contained, tolerance) {
  return container && contained
    && container[0] <= contained[0] + tolerance
    && container[1] <= contained[1] + tolerance
    && container[2] >= contained[2] - tolerance
    && container[3] >= contained[3] - tolerance;
}

function overlaps(left, right, tolerance) {
  return left && right
    && left[0] <= right[2] + tolerance
    && left[2] >= right[0] - tolerance
    && left[1] <= right[3] + tolerance
    && left[3] >= right[1] - tolerance;
}

export function uiStateRequest(expectation) {
  const properties = [...new Set((expectation.styles || []).map((item) => item.property))];
  return {
    mode: "ui-state",
    selector: expectation.selector,
    ...(expectation.referenceSelector ? { referenceSelector: expectation.referenceSelector } : {}),
    ...(properties.length ? { styleProperties: properties } : {}),
  };
}

export function evaluateUiState(observed, expectation) {
  const failures = [];
  if (observed.truncated) failures.push("UI selector matched more than the 128-element observation bound.");
  if (observed.reference?.truncated) failures.push("UI reference selector matched more than the 128-element observation bound.");
  compareCount(observed.count, expectation.count, "match count", failures);
  compareCount(observed.visibleCount, expectation.visibleCount, "visible match count", failures);
  compareCount(observed.focusedCount, expectation.focusedCount, "focused match count", failures);
  compareCount(observed.focusWithinCount, expectation.focusWithinCount, "focus-within match count", failures);
  compareCount(observed.hoverCount, expectation.hoverCount, "hover match count", failures);
  compareCount(observed.disabledCount, expectation.disabledCount, "disabled match count", failures);

  for (const rule of expectation.styles || []) {
    const values = observed.styleValues?.[rule.property] || [];
    if (!values.length) {
      failures.push(`computed style ${rule.property} has no observed values.`);
      continue;
    }
    const passed = values.every((value) => {
      if (rule.operator === "eq") return value === rule.value;
      if (rule.operator === "neq") return value !== rule.value;
      return value.includes(rule.value);
    });
    if (!passed) failures.push(`computed style ${rule.property} values ${JSON.stringify(values)} did not satisfy ${rule.operator} ${JSON.stringify(rule.value)}.`);
  }

  if (expectation.rect) {
    if (!observed.rects?.length) failures.push("no visible rectangles were observed.");
    for (const rect of observed.rects || []) {
      const width = rect[2] - rect[0];
      const height = rect[3] - rect[1];
      if (expectation.rect.minWidth !== undefined && width < expectation.rect.minWidth) failures.push(`rectangle width ${width} is below ${expectation.rect.minWidth}.`);
      if (expectation.rect.maxWidth !== undefined && width > expectation.rect.maxWidth) failures.push(`rectangle width ${width} exceeds ${expectation.rect.maxWidth}.`);
      if (expectation.rect.minHeight !== undefined && height < expectation.rect.minHeight) failures.push(`rectangle height ${height} is below ${expectation.rect.minHeight}.`);
      if (expectation.rect.maxHeight !== undefined && height > expectation.rect.maxHeight) failures.push(`rectangle height ${height} exceeds ${expectation.rect.maxHeight}.`);
    }
  }

  if (expectation.geometry) {
    const tolerance = expectation.geometry.tolerancePx;
    let passed = false;
    if (expectation.geometry.relation === "contains-reference") passed = unionContains(observed.unionRect, observed.reference?.unionRect, tolerance);
    else if (expectation.geometry.relation === "inside-reference") passed = unionContains(observed.reference?.unionRect, observed.unionRect, tolerance);
    else passed = overlaps(observed.unionRect, observed.reference?.unionRect, tolerance);
    if (!passed) failures.push(`geometry relation ${expectation.geometry.relation} failed with ${tolerance}px tolerance.`);
  }

  if (expectation.viewport) {
    const viewport = observed.viewport || {};
    if (expectation.viewport.devicePixelRatio !== undefined && Math.abs(viewport.devicePixelRatio - expectation.viewport.devicePixelRatio) > 0.001) failures.push(`devicePixelRatio is ${viewport.devicePixelRatio}; expected ${expectation.viewport.devicePixelRatio}.`);
    if (expectation.viewport.minWidth !== undefined && viewport.width < expectation.viewport.minWidth) failures.push(`viewport width ${viewport.width} is below ${expectation.viewport.minWidth}.`);
    if (expectation.viewport.maxWidth !== undefined && viewport.width > expectation.viewport.maxWidth) failures.push(`viewport width ${viewport.width} exceeds ${expectation.viewport.maxWidth}.`);
    if (expectation.viewport.minHeight !== undefined && viewport.height < expectation.viewport.minHeight) failures.push(`viewport height ${viewport.height} is below ${expectation.viewport.minHeight}.`);
    if (expectation.viewport.maxHeight !== undefined && viewport.height > expectation.viewport.maxHeight) failures.push(`viewport height ${viewport.height} exceeds ${expectation.viewport.maxHeight}.`);
  }

  return { passed: failures.length === 0, failures, observed };
}
