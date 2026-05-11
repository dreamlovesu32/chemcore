import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

function parseArgs(argv) {
  const args = { inputs: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--out-dir") args.outDir = argv[++i];
    else if (arg === "--help" || arg === "-h") args.help = true;
    else args.inputs.push(arg);
  }
  return args;
}

export async function renderEmfPreviews(jobs) {
  if (!jobs.length) return [];
  const normalized = jobs.map((job) => ({
    input: path.resolve(job.input),
    output: path.resolve(job.output),
  }));
  for (const job of normalized) {
    await fs.mkdir(path.dirname(job.output), { recursive: true });
  }

  const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), "chemcore-render-emf-"));
  const jobsPath = path.join(tempDir, "jobs.json");
  const scriptPath = path.join(tempDir, "render.ps1");
  await fs.writeFile(jobsPath, JSON.stringify(normalized, null, 2), "utf8");
  await fs.writeFile(
    scriptPath,
    String.raw`
param([string]$JobsPath)
$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing
$jobs = Get-Content -Raw -LiteralPath $JobsPath | ConvertFrom-Json
foreach ($job in $jobs) {
  $meta = $null
  $bmp = $null
  $graphics = $null
  try {
    $meta = New-Object System.Drawing.Imaging.Metafile($job.input)
    $width = [Math]::Max(1, [int][Math]::Ceiling($meta.Width))
    $height = [Math]::Max(1, [int][Math]::Ceiling($meta.Height))
    $bmp = New-Object System.Drawing.Bitmap($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bmp)
    $graphics.Clear([System.Drawing.Color]::White)
    $graphics.DrawImage($meta, 0, 0, $width, $height)
    $bmp.Save($job.output, [System.Drawing.Imaging.ImageFormat]::Png)
    Write-Host "[EMF-PNG] $($job.input) -> $($job.output) ($width x $height)"
  }
  finally {
    if ($graphics) { $graphics.Dispose() }
    if ($bmp) { $bmp.Dispose() }
    if ($meta) { $meta.Dispose() }
  }
}
`,
    "utf8"
  );

  const result = spawnSync(
    "powershell.exe",
    ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", scriptPath, "-JobsPath", jobsPath],
    { encoding: "utf8" }
  );
  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  if (result.status !== 0) {
    throw new Error(`EMF preview rendering failed with exit code ${result.status}.`);
  }
  return normalized;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help || !args.inputs.length) {
    console.log("Usage: node scripts/render-emf-preview.mjs [--out-dir dir] <file.emf>...");
    return;
  }
  const jobs = args.inputs.map((input) => {
    const output = args.outDir
      ? path.join(args.outDir, `${path.basename(input)}.png`)
      : `${input}.png`;
    return { input, output };
  });
  await renderEmfPreviews(jobs);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  });
}
