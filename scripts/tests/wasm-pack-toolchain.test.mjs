import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const repositoryRoot = dirname(dirname(dirname(fileURLToPath(import.meta.url))));

test("wasm-pack version, archive, build, CI, and documentation share one contract", async () => {
  const toolchain = JSON.parse(await readFile(join(repositoryRoot, "tools", "wasm-pack.json"), "utf8"));
  assert.equal(toolchain.schema, "chemsema.toolchain.wasm-pack.v1");
  assert.match(toolchain.version, /^\d+\.\d+\.\d+$/);

  const windowsRelease = toolchain.releases?.["windows-x64"];
  assert.ok(windowsRelease);
  assert.match(windowsRelease.asset, new RegExp(`^wasm-pack-v${toolchain.version.replaceAll(".", "\\.")}-x86_64-pc-windows-msvc\\.tar\\.gz$`));
  assert.match(windowsRelease.sha256, /^[0-9a-f]{64}$/);

  const [buildScript, workflow, readme, readmeZh] = await Promise.all([
    readFile(join(repositoryRoot, "scripts", "build-engine-wasm.mjs"), "utf8"),
    readFile(join(repositoryRoot, ".github", "workflows", "ci.yml"), "utf8"),
    readFile(join(repositoryRoot, "README.md"), "utf8"),
    readFile(join(repositoryRoot, "README.zh-CN.md"), "utf8"),
  ]);
  assert.match(buildScript, /tools["'],\s*["']wasm-pack\.json/);
  assert.match(buildScript, /actual !== wasmPackToolchain\.version/);
  assert.match(workflow, /Get-Content "tools\/wasm-pack\.json"/);
  assert.match(workflow, /Get-FileHash[^\n]+SHA256/);
  assert.match(workflow, /wasm-pack version mismatch/);
  assert.match(readme, /tools\/wasm-pack\.json/);
  assert.match(readmeZh, /tools\/wasm-pack\.json/);
});
