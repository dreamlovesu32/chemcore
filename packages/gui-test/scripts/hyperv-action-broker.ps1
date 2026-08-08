param(
  [Parameter(Mandatory = $true)][string]$CoordinatorPath,
  [Parameter(Mandatory = $true)][string]$VmId,
  [Parameter(Mandatory = $true)][string]$CredentialPath
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::InputEncoding = [Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
$session = $null

function Write-BrokerMessage([System.Collections.IDictionary]$Message) {
  [Console]::Out.WriteLine(($Message | ConvertTo-Json -Depth 8 -Compress))
  [Console]::Out.Flush()
}

try {
  if (-not (Test-Path -LiteralPath $CoordinatorPath -PathType Leaf)) { throw 'Coordinator script is unavailable.' }
  if (-not (Test-Path -LiteralPath $CredentialPath -PathType Leaf)) { throw 'Broker credential is unavailable.' }
  $credential = Import-Clixml -LiteralPath $CredentialPath
  $session = New-PSSession -VMId ([Guid]$VmId) -Credential $credential
  if ($session.State -ne 'Opened') { throw 'Persistent PowerShell Direct session did not open.' }
  $global:ChemSemaGuiPersistentSession = $session
  Write-BrokerMessage ([ordered]@{ schema='chemsema.gui.host-action-broker.v1'; status='ready'; processId=$PID; vmId=$VmId })

  while ($null -ne ($line = [Console]::In.ReadLine())) {
    if ([string]::IsNullOrWhiteSpace($line)) { continue }
    $requestId = $null
    try {
      if ($line.Length -gt 131072) { throw 'Broker request exceeds 128 KiB.' }
      $request = $line | ConvertFrom-Json
      $requestId = [string]$request.id
      if ($request.schema -ne 'chemsema.gui.host-action-request.v1' -or $requestId -notmatch '^[0-9a-f]{32}$') { throw 'Broker request identity is invalid.' }
      $arguments = @($request.arguments)
      if ($arguments.Count -lt 2 -or $arguments.Count -gt 64 -or $arguments.Count % 2 -ne 0 -or @($arguments | Where-Object { $_ -isnot [string] -or $_.Length -gt 131072 }).Count -gt 0) { throw 'Broker arguments are invalid or unbounded.' }
      $allowedParameters = @('Operation','VmId','CheckpointId','CredentialPath','GuestAccount','GuestTestRoot','HostAgentPath','HostCandidatePath','HostCdpScriptPath','ActionRequestBase64')
      $parameters = @{}
      for ($index = 0; $index -lt $arguments.Count; $index += 2) {
        $name = [string]$arguments[$index]
        if ($name -notmatch '^-[A-Za-z][A-Za-z0-9]*$') { throw 'Broker parameter name is malformed.' }
        $name = $name.Substring(1)
        if ($name -notin $allowedParameters -or $parameters.ContainsKey($name)) { throw 'Broker parameter is unsupported or duplicated.' }
        $parameters[$name] = [string]$arguments[$index + 1]
      }
      if ($parameters.Operation -ne 'action-transaction') { throw 'Broker only accepts action-transaction operations.' }
      $output = & $CoordinatorPath @parameters 2>&1 | ForEach-Object { [string]$_ }
      Write-BrokerMessage ([ordered]@{ schema='chemsema.gui.host-action-response.v1'; id=$requestId; status=0; stdout=($output -join [Environment]::NewLine); stderr='' })
    }
    catch {
      Write-BrokerMessage ([ordered]@{ schema='chemsema.gui.host-action-response.v1'; id=$requestId; status=1; stdout=''; stderr=$_.Exception.Message })
    }
  }
}
catch {
  Write-BrokerMessage ([ordered]@{ schema='chemsema.gui.host-action-broker.v1'; status='failed'; message=$_.Exception.Message })
  exit 1
}
finally {
  Remove-Variable -Name ChemSemaGuiPersistentSession -Scope Global -ErrorAction SilentlyContinue
  if ($null -ne $session) { Remove-PSSession -Session $session -ErrorAction SilentlyContinue }
}
