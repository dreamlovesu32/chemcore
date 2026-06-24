import { spawn } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = dirname(dirname(fileURLToPath(import.meta.url)));
const children = [];

function start(name, command, args) {
  const child = spawn(command, args, {
    cwd: rootDir,
    env: process.env,
    stdio: "inherit",
    shell: false,
  });
  children.push(child);
  child.on("exit", (code, signal) => {
    if (shuttingDown) {
      return;
    }
    console.error(`[dev:viewer] ${name} exited with ${signal || code}`);
    shutdown(code || 1);
  });
}

let shuttingDown = false;
function shutdown(code = 0) {
  shuttingDown = true;
  for (const child of children) {
    if (!child.killed) {
      child.kill();
    }
  }
  process.exit(code);
}

process.on("SIGINT", () => shutdown(0));
process.on("SIGTERM", () => shutdown(0));

start("server", process.execPath, [join(rootDir, "scripts", "desktop-dev-server.mjs")]);
start("engine", process.execPath, [join(rootDir, "scripts", "dev-engine-wasm.mjs")]);

console.log("[dev:viewer] open http://127.0.0.1:8767/viewer/");
