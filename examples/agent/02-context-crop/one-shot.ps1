$ErrorActionPreference = "Stop"
$cli = if ($env:CHEMCORE_CLI) { $env:CHEMCORE_CLI } else { "chemcore-cli" }
$here = Split-Path -Parent $MyInvocation.MyCommand.Path
Push-Location $here
try {
  & $cli context ..\..\..\figure1.cdxml `
    --target object:obj_line_001 `
    --radius 45 `
    --expand-left 10 `
    --expand-right 10 `
    --expand-top 34 `
    --expand-bottom 34 `
    --capture-out context.png `
    --out context.json `
    --width 1400 `
    --pretty
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
  Pop-Location
}
