import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export const guiTestPackageDir = dirname(dirname(dirname(fileURLToPath(import.meta.url))));
export const repositoryRoot = dirname(dirname(guiTestPackageDir));
export const guiTestsDir = join(repositoryRoot, "tests", "gui");
export const schemaDir = join(guiTestsDir, "schemas");
export const scenarioDir = join(guiTestsDir, "scenarios");

export const schemaFiles = Object.freeze({
  "chemsema.gui.scenario.v1": join(schemaDir, "scenario-v1.schema.json"),
  "chemsema.gui.run.v1": join(schemaDir, "run-v1.schema.json"),
  "chemsema.gui.impact.v1": join(schemaDir, "impact-v1.schema.json"),
  "chemsema.gui.coverage.v1": join(schemaDir, "coverage-v1.schema.json"),
  "chemsema.gui.artifact-manifest.v1": join(schemaDir, "artifact-manifest-v1.schema.json"),
  "chemsema.gui.worker-profile.v1": join(schemaDir, "worker-profile-v1.schema.json"),
  "chemsema.gui.guest-agent.v1": join(schemaDir, "guest-agent-v1.schema.json"),
});
