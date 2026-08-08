#!/usr/bin/env node
import { readFile, readdir, writeFile, mkdir } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { auditCoverage } from "./coverage/audit.mjs";
import { FakeDriver } from "./drivers/fake.mjs";
import { PlaywrightBrowserDriver } from "./drivers/playwright-browser.mjs";
import { ProductionBlackBoxDriver } from "./drivers/production-black-box.mjs";
import { writeEvidenceBundle } from "./evidence/write-bundle.mjs";
import { planImpactedScenarios } from "./impact/select.mjs";
import { guiTestsDir, scenarioDir } from "./protocol/paths.mjs";
import { readValidatedDocument } from "./protocol/validate.mjs";
import { runScenario } from "./runner/run-scenario.mjs";
import { HyperVCoordinator } from "./workers/hyperv.mjs";

async function jsonFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const nested = await Promise.all(entries.map(async (entry) => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      return jsonFiles(path);
    }
    return entry.isFile() && entry.name.endsWith(".json") ? [path] : [];
  }));
  return nested.flat().sort();
}

function parseOptions(args) {
  const positional = [];
  const options = {};
  for (let index = 0; index < args.length; index += 1) {
    const value = args[index];
    if (value.startsWith("--")) {
      const key = value.slice(2);
      const next = args[index + 1];
      if (!next || next.startsWith("--")) {
        options[key] = true;
      } else {
        options[key] = next;
        index += 1;
      }
    } else {
      positional.push(value);
    }
  }
  return { positional, options };
}

async function listScenarios() {
  const rows = [];
  for (const path of await jsonFiles(scenarioDir)) {
    const scenario = await readValidatedDocument(path);
    rows.push({ id: scenario.id, risk: scenario.risk, drivers: scenario.drivers.join(","), title: scenario.title });
  }
  console.table(rows);
}

async function loadScenarios() {
  const paths = await jsonFiles(scenarioDir);
  return { paths, scenarios: await Promise.all(paths.map((path) => readValidatedDocument(path))) };
}

async function audit() {
  const registry = await readValidatedDocument(join(guiTestsDir, "coverage", "registry-v1.json"));
  const { paths, scenarios } = await loadScenarios();
  const result = await auditCoverage({ registry, scenarios, scenarioPaths: paths });
  console.log(JSON.stringify(result, null, 2));
  if (!result.valid) {
    process.exitCode = 1;
  }
}

async function impact(paths, options) {
  if (!paths.length) {
    throw new Error("impact requires one or more changed repository paths.");
  }
  const graph = await readValidatedDocument(resolve(options.graph || join(guiTestsDir, "coverage", "impact-v1.json")));
  const plan = planImpactedScenarios(graph, paths);
  console.log(JSON.stringify(plan, null, 2));
}

async function worker(args, options) {
  const [operation] = args;
  if (!operation || !["host-attest", "reset", "start", "guest-attest", "prepare-guest", "install-agent", "configure-autologon", "configure-desktop-baseline", "install-candidate", "launch-candidate", "dismiss-known-blocker", "activate-candidate", "start-cdp-agent", "stop-cdp-agent", "uia-query", "cdp-state", "input-click", "input-drag", "input-key", "agent-attest-service", "agent-attest-interactive", "stop"].includes(operation)) {
    throw new Error("worker requires a supported worker operation.");
  }
  const profile = await readValidatedDocument(resolve(options.profile || join(guiTestsDir, "environments", "windows-gui-worker-current.json")));
  const coordinator = new HyperVCoordinator(profile);
  let result;
  switch (operation) {
    case "host-attest": result = await coordinator.attestHost(); break;
    case "reset": result = await coordinator.reset(); break;
    case "start": result = await coordinator.start(); break;
    case "guest-attest": result = await coordinator.attestGuest({ requireInteractive: options.interactive === true }); break;
    case "prepare-guest": result = await coordinator.prepareGuest(); break;
    case "install-agent": result = await coordinator.installAgent(); break;
    case "configure-autologon": result = await coordinator.configureAutologon(); break;
    case "configure-desktop-baseline": result = await coordinator.configureDesktopBaseline(); break;
    case "install-candidate": result = await coordinator.installCandidate(); break;
    case "launch-candidate": result = await coordinator.launchCandidate(); break;
    case "dismiss-known-blocker": result = await coordinator.dismissKnownBlocker(); break;
    case "activate-candidate": result = await coordinator.activateCandidate(); break;
    case "start-cdp-agent": result = await coordinator.startCdpAgent(); break;
    case "stop-cdp-agent": result = await coordinator.stopCdpAgent(); break;
    case "uia-query": result = await coordinator.queryUia(options.name, { scopeName: options["scope-name"] }); break;
    case "cdp-state": result = await coordinator.cdpBridge({ mode: "state" }); break;
    case "input-click": result = await coordinator.candidateInput("click", { x: Number(options.x), y: Number(options.y) }, { button: options.button }); break;
    case "input-drag": result = await coordinator.candidateInput("drag", { from: [Number(options["from-x"]), Number(options["from-y"])], to: [Number(options["to-x"]), Number(options["to-y"])] }, { button: options.button, steps: Number(options.steps || 8) }); break;
    case "input-key": result = await coordinator.candidateInput("key", { key: options.key }); break;
    case "agent-attest-service": result = await coordinator.attestServiceAgent(); break;
    case "agent-attest-interactive": result = await coordinator.attestInteractiveAgent(); break;
    case "stop": result = await coordinator.stop(); break;
  }
  console.log(JSON.stringify(result, null, 2));
}

async function validatePaths(paths) {
  if (!paths.length) {
    throw new Error("validate requires at least one JSON path.");
  }
  for (const path of paths) {
    await readValidatedDocument(resolve(path));
    console.log(`[gui-platform] valid ${path}`);
  }
}

async function run(path, options) {
  const scenario = await readValidatedDocument(resolve(path));
  const driverName = options.driver || "fake";
  const driver = driverName === "fake"
    ? new FakeDriver()
    : driverName === "playwright-browser"
      ? new PlaywrightBrowserDriver()
      : driverName === "production-black-box"
        ? new ProductionBlackBoxDriver()
      : null;
  if (!driver) {
    throw new Error(`Driver ${driverName} is not implemented.`);
  }
  const candidate = options.url ? { url: options.url } : {};
  const report = await runScenario({ scenario, driver, candidate });
  const reportPath = resolve(options.report || join("tmp", "gui-platform", `${scenario.id}-${driverName}.json`));
  await mkdir(dirname(reportPath), { recursive: true });
  await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
  const evidenceRoot = resolve(options["evidence-root"] || join("tmp", "gui-platform", "evidence"));
  const bundle = await writeEvidenceBundle({ report, root: evidenceRoot });
  console.log(`[gui-platform] ${report.status} ${scenario.id} via ${driverName}`);
  console.log(`[gui-platform] report ${reportPath}`);
  console.log(`[gui-platform] evidence ${bundle.manifestPath}`);
  if (report.status !== "passed") {
    process.exitCode = 1;
  }
}

function usage() {
  console.log(`ChemSema GUI test platform

Usage:
  npm run gui-platform -- list
  npm run gui-platform -- validate <json> [...json]
  npm run gui-platform -- audit
  npm run gui-platform -- impact <changed-path> [...changed-path] [--graph path]
  npm run gui-platform -- worker <host-attest|start|guest-attest|prepare-guest|install-agent|configure-autologon|install-candidate|launch-candidate|dismiss-known-blocker|activate-candidate|start-cdp-agent|stop-cdp-agent|uia-query|input-click|input-drag|input-key|agent-attest-service|agent-attest-interactive|stop> [--profile path]
  npm run gui-platform -- run <scenario.json> [--driver fake|playwright-browser|production-black-box] [--report path] [--url url]
`);
}

const [command, ...rawArgs] = process.argv.slice(2);
const { positional, options } = parseOptions(rawArgs);
try {
  if (command === "list") {
    await listScenarios();
  } else if (command === "validate") {
    await validatePaths(positional);
  } else if (command === "audit") {
    await audit();
  } else if (command === "impact") {
    await impact(positional, options);
  } else if (command === "worker") {
    await worker(positional, options);
  } else if (command === "run") {
    if (positional.length !== 1) {
      throw new Error("run requires exactly one scenario JSON path.");
    }
    await run(positional[0], options);
  } else {
    usage();
    if (command) {
      process.exitCode = 1;
    }
  }
} catch (error) {
  console.error(`[gui-platform] ${error.message}`);
  process.exitCode = 1;
}
