$ErrorActionPreference = "Stop"

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Resolve-Path (Join-Path $here "..\..\..")
$input = Join-Path $root "figure1.cdxml"
$cli = $env:CHEMCORE_CLI

if (-not $cli) {
  if (Get-Command cargo -ErrorAction SilentlyContinue) {
    Push-Location $root
    try {
      & cargo build -p chemcore-cli
      if ($LASTEXITCODE -ne 0) {
        exit $LASTEXITCODE
      }
    } finally {
      Pop-Location
    }
  }

  $candidates = @(
    (Join-Path $root "target\debug\chemcore-cli.exe"),
    (Join-Path $root "target\release\chemcore-cli.exe")
  )
  foreach ($candidate in $candidates) {
    if (Test-Path $candidate) {
      $cli = $candidate
      break
    }
  }
}

if (-not $cli) {
  $cli = "chemcore-cli"
}

function Invoke-ChemCore {
  param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
  & $cli @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "chemcore-cli failed: $($Arguments -join ' ')"
  }
}

function Invoke-ChemCoreJson {
  param([Parameter(ValueFromRemainingArguments = $true)][string[]]$Arguments)
  $json = & $cli @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "chemcore-cli failed: $($Arguments -join ' ')"
  }
  $json
}

function Assert-File {
  param([string]$Path)
  if (-not (Test-Path $Path)) {
    throw "Expected output was not written: $Path"
  }
  if ((Get-Item $Path).Length -le 0) {
    throw "Expected output is empty: $Path"
  }
}

Push-Location $here
try {
  Invoke-ChemCore version --out version.json --pretty
  Invoke-ChemCore capabilities --out capabilities.json --pretty
  Invoke-ChemCore targets $input --out targets.json --pretty
  Invoke-ChemCore context $input `
    --target object:obj_line_001 `
    --radius 70 `
    --capture-out context.png `
    --out context.json `
    --width 1400 `
    --pretty
  Invoke-ChemCore detail $input `
    --target object:obj_text_008 `
    --include-resource `
    --out condition-detail.json `
    --pretty

  $before = Invoke-ChemCoreJson capture $input `
    --bounds "38.15,93.4,330,205" `
    --out before.png `
    --width 1400 `
    --pretty
  $before | Set-Content -Path capture-before.json -Encoding UTF8

  Invoke-ChemCore run $input commands.json `
    --out output.cdxml `
    --results results.json `
    --pretty
  Invoke-ChemCore convert output.cdxml output.svg

  $after = Invoke-ChemCoreJson capture output.cdxml `
    --bounds "38.15,93.4,330,205" `
    --out after.png `
    --width 1400 `
    --pretty
  $after | Set-Content -Path capture-after.json -Encoding UTF8

  $copy = Invoke-ChemCoreJson copy output.cdxml `
    --target all `
    --payload office-payload.json `
    --no-copy `
    --pretty
  $copy | Set-Content -Path copy-result.json -Encoding UTF8

  foreach ($path in @(
      "version.json",
      "capabilities.json",
      "targets.json",
      "context.json",
      "context.png",
      "condition-detail.json",
      "before.png",
      "capture-before.json",
      "commands.json",
      "results.json",
      "output.cdxml",
      "output.svg",
      "after.png",
      "capture-after.json",
      "copy-result.json",
      "office-payload.json"
    )) {
    Assert-File $path
  }

  $version = Get-Content -Raw version.json | ConvertFrom-Json
  if ($version.protocols.session -ne "chemcore-cli-session-jsonl.v1") {
    throw "Unexpected session protocol id in version.json"
  }

  $results = Get-Content -Raw results.json | ConvertFrom-Json
  if (-not $results.ok) {
    throw "results.json reports ok=false"
  }
  if ($results.commandCount -ne 2 -or $results.executedCount -ne 2) {
    throw "Expected exactly two executed edit commands"
  }
  if (-not $results.save.ok) {
    throw "Edited CDXML save did not report success"
  }

  $beforeManifest = Get-Content -Raw capture-before.json | ConvertFrom-Json
  $afterManifest = Get-Content -Raw capture-after.json | ConvertFrom-Json
  if (-not $beforeManifest.output.verified -or -not $afterManifest.output.verified) {
    throw "Capture output was not verified"
  }

  $copyManifest = Get-Content -Raw copy-result.json | ConvertFrom-Json
  if (-not $copyManifest.payload.verified) {
    throw "Office payload was not verified"
  }

  Write-Host "Reaction POC completed."
} finally {
  Pop-Location
}
