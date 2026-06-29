$ErrorActionPreference = "Stop"
$cli = if ($env:CHEMCORE_CLI) { $env:CHEMCORE_CLI } else { "chemcore-cli" }
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $here
try {
  & $cli targets ..\..\..\figure1.cdxml --out targets.json --pretty
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
  Pop-Location
}
