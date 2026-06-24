import { spawnSync } from "node:child_process";
import { availableParallelism } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const appDir = join(rootDir, "apps", "chemcore-desktop");
const tauriCli = join(rootDir, "node_modules", "@tauri-apps", "cli", "tauri.js");
const jobs = process.env.CHEMCORE_BUILD_JOBS
  || process.env.CARGO_BUILD_JOBS
  || String(Math.max(1, availableParallelism()));

const result = spawnSync(
  process.execPath,
  [tauriCli, "dev", ...process.argv.slice(2)],
  {
    cwd: appDir,
    env: {
      ...process.env,
      CARGO_BUILD_JOBS: jobs,
    },
    stdio: "inherit",
    shell: false,
  },
);

if (result.error) {
  throw result.error;
}
process.exit(result.status ?? 0);
