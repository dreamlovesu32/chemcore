#!/usr/bin/env node
import { readFile, readdir, writeFile, mkdir } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { FakeDriver } from "./drivers/fake.mjs";
import { PlaywrightBrowserDriver } from "./drivers/playwright-browser.mjs";
import { scenarioDir } from "./protocol/paths.mjs";
import { readValidatedDocument } from "./protocol/validate.mjs";
import { runScenario } from "./runner/run-scenario.mjs";

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
      : null;
  if (!driver) {
    throw new Error(`Driver ${driverName} is not implemented.`);
  }
  const candidate = options.url ? { url: options.url } : {};
  const report = await runScenario({ scenario, driver, candidate });
  const reportPath = resolve(options.report || join("tmp", "gui-platform", `${scenario.id}-${driverName}.json`));
  await mkdir(dirname(reportPath), { recursive: true });
  await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, "utf8");
  console.log(`[gui-platform] ${report.status} ${scenario.id} via ${driverName}`);
  console.log(`[gui-platform] report ${reportPath}`);
  if (report.status !== "passed") {
    process.exitCode = 1;
  }
}

function usage() {
  console.log(`ChemSema GUI test platform

Usage:
  npm run gui-platform -- list
  npm run gui-platform -- validate <json> [...json]
  npm run gui-platform -- run <scenario.json> [--driver fake|playwright-browser] [--report path] [--url url]
`);
}

const [command, ...rawArgs] = process.argv.slice(2);
const { positional, options } = parseOptions(rawArgs);
try {
  if (command === "list") {
    await listScenarios();
  } else if (command === "validate") {
    await validatePaths(positional);
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
