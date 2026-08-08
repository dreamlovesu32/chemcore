import { createHash } from "node:crypto";
import { mkdir, readFile, rename, writeFile } from "node:fs/promises";
import { dirname, join, relative } from "node:path";
import { assertValidDocument } from "../protocol/validate.mjs";

function digest(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

async function writeImmutable(path, bytes) {
  await mkdir(dirname(path), { recursive: true });
  try {
    await writeFile(path, bytes, { flag: "wx" });
  } catch (error) {
    if (error.code !== "EEXIST") throw error;
    const existing = await readFile(path);
    if (!existing.equals(bytes)) throw new Error(`Immutable evidence object collision at ${path}.`);
  }
}

export async function writeEvidenceBundle({ report, artifactPayloads = [], root }) {
  await assertValidDocument(report, `run report ${report.runId}`);
  const reportBytes = Buffer.from(`${JSON.stringify(report, null, 2)}\n`, "utf8");
  const reportSha256 = digest(reportBytes);
  const objectPath = join(root, "objects", "sha256", reportSha256.slice(0, 2), `${reportSha256}-run-report.json`);
  await writeImmutable(objectPath, reportBytes);

  const payloadByName = new Map(artifactPayloads.map((artifact) => [artifact.descriptor?.name, artifact]));
  if (payloadByName.size !== artifactPayloads.length) throw new Error("Evidence payload names must be unique.");
  const storedPayloads = [];
  for (const descriptor of report.artifacts) {
    const payload = payloadByName.get(descriptor.name);
    if (!payload) throw new Error(`Evidence payload ${descriptor.name} is absent.`);
    const bytes = Buffer.isBuffer(payload.bytes) ? payload.bytes : Buffer.from(payload.bytes || []);
    if (bytes.length !== descriptor.size || digest(bytes) !== descriptor.sha256) {
      throw new Error(`Evidence payload ${descriptor.name} does not match its report descriptor.`);
    }
    const payloadPath = join(root, "objects", "sha256", descriptor.sha256.slice(0, 2), `${descriptor.sha256}-${descriptor.name}`);
    await writeImmutable(payloadPath, bytes);
    storedPayloads.push({ descriptor, path: payloadPath });
    payloadByName.delete(descriptor.name);
  }
  if (payloadByName.size) throw new Error(`Unreported evidence payloads are present: ${[...payloadByName.keys()].join(", ")}.`);

  const manifest = {
    schema: "chemsema.gui.artifact-manifest.v1",
    runId: report.runId,
    artifacts: [{
      name: "run-report.json",
      mediaType: "application/json",
      uri: relative(root, objectPath).replaceAll("\\", "/"),
      size: reportBytes.length,
      sha256: reportSha256,
      retention: report.status === "passed" ? "sample" : "failure",
    }, ...storedPayloads.map(({ descriptor, path }) => ({
      name: descriptor.name,
      mediaType: descriptor.mediaType,
      uri: relative(root, path).replaceAll("\\", "/"),
      size: descriptor.size,
      sha256: descriptor.sha256,
      retention: descriptor.retention,
    }))],
  };
  await assertValidDocument(manifest, `artifact manifest ${report.runId}`);
  const manifestPath = join(root, "records", report.evidenceKey, report.runId, "artifact-manifest.json");
  const temporaryPath = `${manifestPath}.${process.pid}.tmp`;
  await mkdir(dirname(manifestPath), { recursive: true });
  await writeFile(temporaryPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
  await rename(temporaryPath, manifestPath);
  return { manifest, manifestPath, objectPath, artifactObjectPaths: storedPayloads.map(({ path }) => path) };
}
