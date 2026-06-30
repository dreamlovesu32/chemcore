$ErrorActionPreference = "Stop"

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$root = Resolve-Path (Join-Path $here "..\..")
$oldCli = $env:CHEMCORE_CLI
$setCli = $false

if (-not $env:CHEMCORE_CLI) {
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
      $env:CHEMCORE_CLI = $candidate
      $setCli = $true
      break
    }
  }
}

if (-not $env:CHEMCORE_CLI) {
  $env:CHEMCORE_CLI = "chemcore-cli"
  $setCli = $true
}

$examples = @(
  "01-discover-targets",
  "02-context-crop",
  "03-edit-reaction-scheme",
  "04-session-workflow",
  "05-office-copy",
  "06-reaction-poc"
)

Push-Location $root
try {
  foreach ($example in $examples) {
    $script = Join-Path $here (Join-Path $example "one-shot.ps1")
    if (-not (Test-Path $script)) {
      $script = Join-Path $here (Join-Path $example "run.ps1")
    }
    if (-not (Test-Path $script)) {
      throw "No example entrypoint found for $example"
    }
    Write-Host "==> $example"
    & powershell -ExecutionPolicy Bypass -File $script
    if ($LASTEXITCODE -ne 0) {
      exit $LASTEXITCODE
    }
  }
  Write-Host "Agent examples completed."
} finally {
  Pop-Location
  if ($setCli) {
    if ($null -eq $oldCli) {
      Remove-Item Env:\CHEMCORE_CLI -ErrorAction SilentlyContinue
    } else {
      $env:CHEMCORE_CLI = $oldCli
    }
  }
}
