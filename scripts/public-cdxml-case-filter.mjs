export function matchesPublicCdxmlCasePattern(entry, pattern) {
  const needle = String(pattern ?? "").trim().replaceAll("\\", "/").toLowerCase();
  if (!needle) return false;

  const caseId = String(entry.caseId ?? entry.id?.match(/^(\d{4})_/)?.[1] ?? "");
  if (/^\d+$/.test(needle)) {
    return caseId === needle.padStart(4, "0");
  }

  const relativePath = String(
    entry.relativeCdxml
      ?? [entry.source, entry.path].filter(Boolean).join("/"),
  ).replaceAll("\\", "/").toLowerCase();
  if (needle.includes("/")) {
    return relativePath === needle;
  }
  if (/\.[a-z0-9]+$/i.test(needle)) {
    return relativePath.split("/").at(-1) === needle;
  }

  return [
    entry.id,
    entry.label,
    entry.source,
    entry.path,
    relativePath,
  ].some((value) => String(value ?? "").toLowerCase().includes(needle));
}
