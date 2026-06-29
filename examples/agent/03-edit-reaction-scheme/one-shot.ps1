$ErrorActionPreference = "Stop"
$cli = if ($env:CHEMCORE_CLI) { $env:CHEMCORE_CLI } else { "chemcore-cli" }
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
& $cli new (Join-Path $here "commands.json") `
  --out (Join-Path $here "output.ccjs") `
  --results (Join-Path $here "expected-results.json") `
  --pretty
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$captureJson = & $cli capture (Join-Path $here "output.ccjs") `
  --target all `
  --out (Join-Path $here "crop.png") `
  --width 1400 `
  --pretty
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$captureJson | Set-Content -Path (Join-Path $here "capture.json") -Encoding UTF8
