import { spawn } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { repositoryRoot } from "../protocol/paths.mjs";

const defaultCliPath = join(repositoryRoot, "target", "release", "chemsema-cli.exe");
const outputLimit = 16 * 1024 * 1024;

function runCli(executable, args, { timeoutMs = 30000 } = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(executable, args, { windowsHide: true, stdio: ["ignore", "pipe", "pipe"] });
    let stdout = Buffer.alloc(0);
    let stderr = Buffer.alloc(0);
    let failure = null;
    const append = (current, chunk) => {
      const next = Buffer.concat([current, chunk]);
      if (next.length > outputLimit) {
        failure = new Error("Independent document oracle exceeded its 16 MiB output limit.");
        child.kill();
      }
      return next;
    };
    child.stdout.on("data", (chunk) => { stdout = append(stdout, chunk); });
    child.stderr.on("data", (chunk) => { stderr = append(stderr, chunk); });
    child.on("error", (error) => { failure = error; });
    const timer = setTimeout(() => {
      failure = new Error(`Independent document oracle exceeded ${timeoutMs} ms.`);
      child.kill();
    }, timeoutMs);
    child.on("close", (status) => {
      clearTimeout(timer);
      if (failure) return reject(failure);
      if (status !== 0) {
        const diagnostic = stderr.toString("utf8").trim() || stdout.toString("utf8").trim() || `exit ${status}`;
        const boundedDiagnostic = diagnostic.length > 4096 ? `${diagnostic.slice(0, 4096)}\n[truncated]` : diagnostic;
        return reject(new Error(`Independent document oracle failed: ${boundedDiagnostic}`));
      }
      try { resolve(JSON.parse(stdout.toString("utf8"))); } catch (error) {
        reject(new Error(`Independent document oracle returned invalid JSON: ${error.message}`));
      }
    });
  });
}

export function evaluateDocumentReports({ inspect, validation }, expected) {
  const counts = inspect?.summary?.counts || {};
  const observed = {
    formatName: inspect?.summary?.format?.name ?? null,
    formatVersion: inspect?.summary?.format?.version ?? null,
    nodes: counts.nodes ?? null,
    bonds: counts.bonds ?? null,
    molecules: counts.molecules ?? null,
    objects: counts.objects ?? null,
    validationSchema: validation?.schema ?? null,
    validationOk: validation?.ok === true,
    issueCount: Array.isArray(validation?.issues) ? validation.issues.length : null,
  };
  const passed = observed.formatName === "chemsema"
    && observed.formatVersion === "0.2"
    && observed.validationSchema === "chemsema.validation-report.v1"
    && observed.validationOk
    && observed.issueCount === 0
    && Object.entries(expected).every(([name, value]) => observed[name] === value);
  return { passed, observed };
}

export function evaluateDocumentArrowProperties(bytes, expected) {
  let document;
  try {
    document = JSON.parse(Buffer.isBuffer(bytes) ? bytes.toString("utf8") : String(bytes));
  } catch {
    return { passed: false, observed: [] };
  }
  const scene = Array.isArray(document?.entities?.scene) ? document.entities.scene : [];
  const styles = document?.styles && typeof document.styles === "object" ? document.styles : {};
  const observed = expected.map((entry) => {
    const object = scene.find((candidate) => candidate?.id === entry.id && candidate?.type === "line");
    const arrow = object?.payload?.arrowHead || {};
    return {
      id: entry.id,
      found: !!object,
      kind: arrow.kind ?? null,
      curve: arrow.curve ?? null,
      length: arrow.length ?? null,
      head: arrow.head ?? null,
      tail: arrow.tail ?? null,
      bold: arrow.bold ?? null,
      noGo: arrow.noGo ?? null,
      stroke: styles[object?.styleRef]?.stroke ?? object?.payload?.stroke ?? null,
    };
  });
  const passed = observed.every((actual, index) => actual.found
    && Object.entries(expected[index]).every(([name, value]) => name === "id" || actual[name] === value));
  return { passed, observed };
}

export function validationLevelForDocumentBytes(bytes) {
  try {
    const document = JSON.parse(Buffer.isBuffer(bytes) ? bytes.toString("utf8") : String(bytes));
    const resources = document?.resources && typeof document.resources === "object"
      ? Object.values(document.resources)
      : [];
    const hasChemicalGraph = resources.some((resource) =>
      resource?.type === "molecule_fragment2d"
      && Array.isArray(resource?.data?.nodes)
      && resource.data.nodes.length > 0
    );
    return hasChemicalGraph ? "chemical" : "structural";
  } catch {
    return "structural";
  }
}

export async function inspectDocumentBytes(bytes, { cliPath = defaultCliPath } = {}) {
  if (!Buffer.isBuffer(bytes) || bytes.length === 0 || bytes.length > 64 * 1024 * 1024) {
    throw new Error("Independent document oracle requires a nonempty CCJS payload up to 64 MiB.");
  }
  const root = await mkdtemp(join(tmpdir(), "chemsema-document-oracle-"));
  try {
    const documentPath = join(root, "saved-document.ccjs");
    await writeFile(documentPath, bytes, { flag: "wx" });
    const validationLevel = validationLevelForDocumentBytes(bytes);
    const [inspect, validation] = await Promise.all([
      runCli(cliPath, ["inspect", documentPath, "--include", "summary,objects,molecules,resources"]),
      runCli(cliPath, ["validate", documentPath, "--level", validationLevel]),
    ]);
    return { inspect, validation, validationLevel };
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}
