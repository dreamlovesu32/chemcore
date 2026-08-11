function closeGeometry(left, right, tolerance) {
  if (left == null || right == null) return left === right;
  if (!Array.isArray(left) || !Array.isArray(right) || left.length !== right.length) {
    return false;
  }
  return left.every(
    (value, index) =>
      Number.isFinite(value)
      && Number.isFinite(right[index])
      && Math.abs(value - right[index]) <= tolerance,
  );
}

function geometryItemsEquivalent(left, right, tolerance) {
  return left.key === right.key
    && closeGeometry(left.position, right.position, tolerance)
    && closeGeometry(left.box, right.box, tolerance)
    && closeGeometry(left.lineHeight, right.lineHeight, tolerance);
}

export function compareVisualGeometry(before = [], after = [], tolerance = 0.5) {
  if (before.length !== after.length) return false;

  // Repeated labels and captions can have the same semantic key. Sorting by a
  // serialized coordinate makes a 0.01 pt x change outrank a large y
  // separation and can pair the wrong instances. Use an actual one-to-one
  // tolerance match within each key instead.
  const matchedBeforeByAfter = new Array(after.length).fill(-1);
  const match = (beforeIndex, visitedAfter) => {
    for (let afterIndex = 0; afterIndex < after.length; afterIndex += 1) {
      if (visitedAfter[afterIndex]) continue;
      if (!geometryItemsEquivalent(before[beforeIndex], after[afterIndex], tolerance)) continue;
      visitedAfter[afterIndex] = true;
      const previousBefore = matchedBeforeByAfter[afterIndex];
      if (
        previousBefore === -1
        || match(previousBefore, visitedAfter)
      ) {
        matchedBeforeByAfter[afterIndex] = beforeIndex;
        return true;
      }
    }
    return false;
  };

  return before.every((_, beforeIndex) =>
    match(beforeIndex, new Array(after.length).fill(false)));
}
