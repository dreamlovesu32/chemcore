import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const appDir = join(rootDir, "apps", "chemcore-desktop");
const tauriCli = join(rootDir, "node_modules", "@tauri-apps", "cli", "tauri.js");
const args = process.argv.slice(2);

if (!args.length) {
  console.error("Usage: node scripts/desktop-tauri.mjs <dev|build|info> [...args]");
  process.exit(1);
}

if (args[0] === "dev" || args[0] === "build") {
  for (const packageName of ["chemcore-office", "chemcore-cli"]) {
    const cargoArgs = ["build", "-p", packageName];
    if (args[0] === "build") {
      cargoArgs.push("--release");
    }
    const cargoResult = spawnSync("cargo", cargoArgs, {
      cwd: rootDir,
      stdio: "inherit",
      shell: false,
    });
    if (cargoResult.error) {
      throw cargoResult.error;
    }
    if (cargoResult.status !== 0) {
      process.exit(cargoResult.status ?? 1);
    }
  }
}

const result = spawnSync(process.execPath, [tauriCli, ...args], {
  cwd: appDir,
  stdio: "inherit",
  shell: false,
});

if (result.error) {
  throw result.error;
}
process.exit(result.status ?? 0);
