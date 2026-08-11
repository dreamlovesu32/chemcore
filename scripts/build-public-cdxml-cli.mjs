import { execFileSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { repositoryState, sha256File } from "./public-cdxml-provenance.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repository = repositoryState(repoRoot);
const cliPath = path.join(
  repoRoot,
  "target",
  "release",
  process.platform === "win32" ? "chemsema-cli.exe" : "chemsema-cli",
);

execFileSync("cargo", ["build", "--release", "-p", "chemsema-cli"], {
  cwd: repoRoot,
  env: {
    ...process.env,
    CHEMSEMA_BUILD_IDENTITY: repository.identity,
  },
  stdio: "inherit",
});

const version = JSON.parse(execFileSync(cliPath, ["version"], {
  cwd: repoRoot,
  encoding: "utf8",
}));
if (version.buildIdentity !== repository.identity) {
  throw new Error(
    `Built CLI identity ${version.buildIdentity ?? "(missing)"} does not match `
    + `repository identity ${repository.identity}`,
  );
}
console.log(JSON.stringify({
  cliPath,
  sha256: sha256File(cliPath),
  repositoryIdentity: repository.identity,
  dirty: repository.dirty,
}, null, 2));
