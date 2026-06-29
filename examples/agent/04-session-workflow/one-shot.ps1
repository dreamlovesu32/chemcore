$ErrorActionPreference = "Stop"
$cli = if ($env:CHEMCORE_CLI) { $env:CHEMCORE_CLI } else { "chemcore-cli" }
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $here
try {
  $transcript = Get-Content .\session.jsonl -Raw | & $cli session ..\..\..\figure1.cdxml
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  $transcript | Set-Content -Path .\transcript.jsonl -Encoding UTF8
} finally {
  Pop-Location
}
