import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

function parseArgs(argv) {
  const args = {
    outDir: "tmp/chemdraw-oracle",
    formats: ["svg", "emf"],
    inputs: [],
  };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--out") {
      args.outDir = argv[++i];
    } else if (arg === "--formats") {
      args.formats = argv[++i].split(",").map((value) => value.trim()).filter(Boolean);
    } else if (arg === "--help" || arg === "-h") {
      args.help = true;
    } else {
      args.inputs.push(arg);
    }
  }
  return args;
}

function safeStem(inputPath) {
  return path.basename(inputPath, path.extname(inputPath)).replace(/[<>:"/\\|?*\x00-\x1f]/g, "_");
}

function runPowershell(scriptPath, jobsPath) {
  const candidates = [
    "powershell.exe",
    "C:\\Windows\\SysWOW64\\WindowsPowerShell\\v1.0\\powershell.exe",
  ];
  let lastResult = null;
  for (const executable of candidates) {
    const result = spawnSync(
      executable,
      ["-NoProfile", "-ExecutionPolicy", "Bypass", "-File", scriptPath, "-JobsPath", jobsPath],
      { encoding: "utf8" }
    );
    if (result.stdout) {
      process.stdout.write(result.stdout);
    }
    if (result.stderr) {
      process.stderr.write(result.stderr);
    }
    if (result.status === 0) {
      return result;
    }
    lastResult = result;
    const combinedOutput = `${result.stdout ?? ""}\n${result.stderr ?? ""}`;
    const shouldFallback =
      executable !== candidates[candidates.length - 1] &&
      /0x80040112|Class is not licensed for use/i.test(combinedOutput);
    if (!shouldFallback) {
      return result;
    }
    console.warn(`[CHEMDRAW] ${executable} failed with COM license gating, retrying with 32-bit PowerShell.`);
  }
  return lastResult;
}

async function defaultInputs() {
  const candidates = [
    "figure1.cdxml",
    "figure2.cdxml",
  ];
  const existing = [];
  for (const candidate of candidates) {
    try {
      await fs.access(candidate);
      existing.push(candidate);
    } catch {
      // Skip optional fixtures so the script can run from source checkouts.
    }
  }
  return existing;
}

export async function generateChemDrawOracle(options = {}) {
  const outDir = path.resolve(options.outDir ?? "tmp/chemdraw-oracle");
  const formats = options.formats ?? ["svg", "emf"];
  const inputs = (options.inputs?.length ? options.inputs : await defaultInputs()).map((input) =>
    path.resolve(input)
  );
  if (!inputs.length) {
    throw new Error("No input CDXML/CDX files were provided or found.");
  }

  await fs.mkdir(outDir, { recursive: true });
  const jobs = inputs.map((input) => {
    const stem = safeStem(input);
    const outputs = Object.fromEntries(
      formats.map((format) => [format, path.join(outDir, `${stem}.chemdraw.${format}`)])
    );
    return { input, outputs };
  });

  const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), "chemcore-chemdraw-oracle-"));
  const jobsPath = path.join(tempDir, "jobs.json");
  const scriptPath = path.join(tempDir, "run.ps1");
  await fs.writeFile(jobsPath, JSON.stringify(jobs, null, 2), "utf8");
  await fs.writeFile(
    scriptPath,
    String.raw`
param([string]$JobsPath)
$ErrorActionPreference = "Stop"
$jobs = Get-Content -Raw -Encoding UTF8 -LiteralPath $JobsPath | ConvertFrom-Json
$app = $null
$doc = $null
try {
  $app = New-Object -ComObject ChemDraw.Application
  $app.Visible = $false
  foreach ($job in $jobs) {
    Write-Host "[CHEMDRAW] open $($job.input)"
    $doc = $app.Documents.Open($job.input)
    foreach ($property in $job.outputs.PSObject.Properties) {
      $format = $property.Name
      $output = $property.Value
      $parent = Split-Path -Parent $output
      if ($parent) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
      }
      Remove-Item -LiteralPath $output -ErrorAction SilentlyContinue
      Write-Host "[CHEMDRAW] save $format $output"
      $doc.SaveAs($output) | Out-Null
      if (!(Test-Path -LiteralPath $output)) {
        throw "ChemDraw did not create $output"
      }
    }
    $doc.Close() | Out-Null
    [System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($doc) | Out-Null
    $doc = $null
  }
}
finally {
  if ($doc) {
    try { $doc.Close() | Out-Null } catch {}
    [System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($doc) | Out-Null
  }
  if ($app) {
    try { $app.Quit() | Out-Null } catch {}
    [System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($app) | Out-Null
  }
}
`,
    "utf8"
  );

  const result = runPowershell(scriptPath, jobsPath);
  if (result.status !== 0) {
    throw new Error(`ChemDraw oracle generation failed with exit code ${result.status}.`);
  }

  return jobs;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help) {
    console.log("Usage: node scripts/chemdraw-oracle.mjs [--out dir] [--formats svg,emf] <file.cdxml>...");
    return;
  }
  const jobs = await generateChemDrawOracle(args);
  for (const job of jobs) {
    console.log(JSON.stringify(job));
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  });
}
