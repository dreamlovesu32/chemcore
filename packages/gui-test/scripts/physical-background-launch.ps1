param(
  [Parameter(Mandatory = $true)][string]$Executable,
  [Parameter(Mandatory = $true)][string]$ArgumentsBase64,
  [Parameter(Mandatory = $true)][string]$StdoutPath,
  [Parameter(Mandatory = $true)][string]$StderrPath,
  [Parameter(Mandatory = $true)][string]$ReceiptPath
)

$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
$decodedArguments = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($ArgumentsBase64)) |
  ConvertFrom-Json
$arguments = @()
foreach ($argument in $decodedArguments) { $arguments += [string]$argument }
if ($arguments.Count -lt 1 -or $arguments.Count -gt 64 -or
    @($arguments | Where-Object { ([string]$_).Length -gt 32768 -or [string]$_ -match '[\r\n"]' }).Count -gt 0) {
  throw 'Background process arguments are invalid.'
}
$quotedArguments = @($arguments | ForEach-Object {
  if ([string]$_ -match '\s') { '"' + [string]$_ + '"' } else { [string]$_ }
})
$process = Start-Process -FilePath $Executable -ArgumentList $quotedArguments -PassThru -WindowStyle Hidden `
  -RedirectStandardOutput $StdoutPath -RedirectStandardError $StderrPath
$receipt = [ordered]@{
  schema = 'chemsema.gui.physical-background-process.v1'
  processId = $process.Id
} | ConvertTo-Json -Compress
[IO.File]::WriteAllText($ReceiptPath, $receipt, [Text.UTF8Encoding]::new($false))
