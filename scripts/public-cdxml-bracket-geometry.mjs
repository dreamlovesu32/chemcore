function finitePoint(value, fallback = [0, 0]) {
  if (!Array.isArray(value) || value.length < 2) return fallback;
  return [
    Number.isFinite(value[0]) ? value[0] : fallback[0],
    Number.isFinite(value[1]) ? value[1] : fallback[1],
  ];
}

function rotatePoint(point, center, degrees) {
  if (!Number.isFinite(degrees) || Math.abs(degrees) <= Number.EPSILON) return point;
  const radians = degrees * Math.PI / 180;
  const cosine = Math.cos(radians);
  const sine = Math.sin(radians);
  const dx = point[0] - center[0];
  const dy = point[1] - center[1];
  return [
    center[0] + dx * cosine - dy * sine,
    center[1] + dx * sine + dy * cosine,
  ];
}

export function bracketWorldEndpoints({
  bbox,
  translate,
  rotate,
  kind,
  side,
}) {
  if (!Array.isArray(bbox) || bbox.length < 4 || !["left", "right"].includes(side)) {
    return null;
  }
  const [x, y, width, height] = bbox;
  if (![x, y, width, height].every(Number.isFinite) || width <= 0 || height <= 0) {
    return null;
  }
  const [translateX, translateY] = finitePoint(translate);
  const handleX = (
    (kind === "square" && side === "left")
    || ((kind === "round" || kind === "curly") && side === "right")
  ) ? 0 : width;
  const center = [
    translateX + x + width * 0.5,
    translateY + y + height * 0.5,
  ];
  const top = rotatePoint(
    [translateX + x + handleX, translateY + y],
    center,
    rotate,
  );
  const bottom = rotatePoint(
    [translateX + x + handleX, translateY + y + height],
    center,
    rotate,
  );
  return { top, bottom };
}
