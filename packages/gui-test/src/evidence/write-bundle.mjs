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

export async function writeEvidenceBundle({ report, root }) {
  await assertValidDocument(report, `run report ${report.runId}`);
  const reportBytes = Buffer.from(`${JSON.stringify(report, null, 2)}\n`, "utf8");
  const reportSha256 = digest(reportBytes);
  const objectPath = join(root, "objects", "sha256", reportSha256.slice(0, 2), `${reportSha256}.json`);
  await writeImmutable(objectPath, reportBytes);

  const manifest = {
    schema: "chemsema.gui.artifact-manifest.v1",
    runId: report.runId,
    artifacts: [{
      uri: relative(root, objectPath).replaceAll("\\", "/"),
      size: reportBytes.length,
      sha256: reportSha256,
      retention: report.status === "passed" ? "sample" : "failure",
    }],
  };
  await assertValidDocument(manifest, `artifact manifest ${report.runId}`);
  const manifestPath = join(root, "records", report.evidenceKey, report.runId, "artifact-manifest.json");
  const temporaryPath = `${manifestPath}.${process.pid}.tmp`;
  await mkdir(dirname(manifestPath), { recursive: true });
  await writeFile(temporaryPath, `${JSON.stringify(manifest, null, 2)}\n`, "utf8");
  await rename(temporaryPath, manifestPath);
  return { manifest, manifestPath, objectPath };
}
