import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { isAbsolute, join, relative, resolve } from "node:path";
import { assertValidDocument } from "../protocol/validate.mjs";

function digest(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

export async function verifyQualificationEvidence({ reports, evidenceRoot }) {
  const root = resolve(evidenceRoot);
  const diagnostics = [];
  let artifactCount = 0;
  let totalBytes = 0;
  let verifiedHashes = 0;
  for (const report of reports) {
    const manifestPath = join(root, "records", report.evidenceKey, report.runId, "artifact-manifest.json");
    try {
      const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
      await assertValidDocument(manifest, `qualification artifact manifest ${report.runId}`);
      if (manifest.runId !== report.runId) throw new Error("manifest run identity mismatch");
      for (const artifact of manifest.artifacts) {
        if (isAbsolute(artifact.uri)) throw new Error(`absolute artifact URI ${artifact.uri}`);
        const objectPath = resolve(root, ...artifact.uri.split("/"));
        const boundary = relative(root, objectPath);
        if (!boundary || boundary.startsWith("..") || isAbsolute(boundary)) throw new Error(`artifact URI escapes evidence root: ${artifact.uri}`);
        const bytes = await readFile(objectPath);
        artifactCount += 1;
        totalBytes += bytes.length;
        if (bytes.length !== artifact.size) throw new Error(`artifact size mismatch: ${artifact.name}`);
        if (digest(bytes) !== artifact.sha256) throw new Error(`artifact SHA-256 mismatch: ${artifact.name}`);
        verifiedHashes += 1;
      }
    } catch (error) {
      diagnostics.push(`evidence:${report.runId}:${error.message}`);
    }
  }
  return {
    diagnostics,
    summary: { manifestCount: reports.length - diagnostics.length, artifactCount, totalBytes, verifiedHashes },
  };
}
