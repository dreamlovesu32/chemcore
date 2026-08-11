import { createHash } from "node:crypto";

export function canonicalize(value) {
  if (Array.isArray(value)) {
    return value.map(canonicalize);
  }
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((key) => [key, canonicalize(value[key])]),
    );
  }
  return value;
}

export function canonicalJson(value) {
  return JSON.stringify(canonicalize(value));
}

export function sha256(value) {
  return createHash("sha256").update(
    typeof value === "string" || Buffer.isBuffer(value) ? value : canonicalJson(value),
  ).digest("hex");
}

export function evidenceKey({ scenario, driver, environment, componentClosure = [], artifacts = [] }) {
  return sha256({
    schema: "chemsema.gui.evidence-key.v1",
    scenario,
    driver,
    environment,
    componentClosure: [...componentClosure].sort(),
    artifacts: [...artifacts].sort(),
  });
}
