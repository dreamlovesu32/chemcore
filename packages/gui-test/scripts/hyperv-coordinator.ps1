param(
  [Parameter(Mandatory = $true)]
  [ValidateSet('host-attest', 'start', 'guest-attest', 'prepare-guest', 'install-agent', 'agent-attest-service', 'stop')]
  [string]$Operation,

  [Parameter(Mandatory = $true)]
  [string]$VmId,

  [string]$CredentialPath,
  [string]$GuestAccount,
  [string]$GuestTestRoot,
  [string]$HostAgentPath
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Write-Result([object]$Value) {
  $Value | ConvertTo-Json -Depth 10 -Compress
}

function Get-WorkerVm {
  Get-VM -Id ([Guid]$VmId)
}

function Get-HostAttestation {
  $vm = Get-WorkerVm
  $memory = Get-VMMemory -VM $vm
  $processor = Get-VMProcessor -VM $vm
  $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
  $hyperVAdministratorsSid = 'S-1-5-32-578'
  [ordered]@{
    schema = 'chemsema.gui.worker-attestation.v1'
    operation = 'host-attest'
    host = [ordered]@{
      computerName = $env:COMPUTERNAME
      user = $identity.Name
      hyperVAdministrator = @($identity.Groups.Value) -contains $hyperVAdministratorsSid
      vmms = (Get-Service vmms).Status.ToString()
      vmcompute = (Get-Service vmcompute).Status.ToString()
    }
    vm = [ordered]@{
      id = $vm.Id.ToString()
      name = $vm.Name
      state = $vm.State.ToString()
      generation = $vm.Generation
      cpuUnits = $processor.Count
      dynamicMemory = $memory.DynamicMemoryEnabled
      memoryMinimumBytes = $memory.Minimum
      memoryStartupBytes = $memory.Startup
      memoryMaximumBytes = $memory.Maximum
      automaticCheckpoints = $vm.AutomaticCheckpointsEnabled
    }
    credential = [ordered]@{
      configured = -not [string]::IsNullOrWhiteSpace($CredentialPath)
      exists = if ([string]::IsNullOrWhiteSpace($CredentialPath)) { $false } else { Test-Path -LiteralPath $CredentialPath -PathType Leaf }
    }
  }
}

function Get-GuestCredential {
  if ([string]::IsNullOrWhiteSpace($CredentialPath) -or -not (Test-Path -LiteralPath $CredentialPath -PathType Leaf)) {
    throw 'The encrypted PowerShell Direct credential file is unavailable.'
  }
  Import-Clixml -LiteralPath $CredentialPath
}

function Invoke-Guest([scriptblock]$ScriptBlock, [object[]]$ArgumentList = @()) {
  $credential = Get-GuestCredential
  $vm = Get-WorkerVm
  Invoke-Command -VMId $vm.Id -Credential $credential -ScriptBlock $ScriptBlock -ArgumentList $ArgumentList
}

function Start-Worker {
  $vm = Get-WorkerVm
  $startedByCoordinator = $false
  if ($vm.State -ne 'Running') {
    Start-VM -VM $vm | Out-Null
    $startedByCoordinator = $true
  }
  $deadline = [DateTime]::UtcNow.AddSeconds(90)
  do {
    $vm = Get-WorkerVm
    if ($vm.State -eq 'Running') { break }
    Start-Sleep -Milliseconds 500
  } while ([DateTime]::UtcNow -lt $deadline)
  if ($vm.State -ne 'Running') {
    throw "Worker VM '$VmId' did not reach Running within 90 seconds."
  }
  [ordered]@{
    schema = 'chemsema.gui.worker-attestation.v1'
    operation = 'start'
    vmId = $vm.Id.ToString()
    vmName = $vm.Name
    state = $vm.State.ToString()
    startedByCoordinator = $startedByCoordinator
  }
}

function Get-GuestAttestation {
  $result = Invoke-Guest -ScriptBlock {
    param($ExpectedAccount, $ExpectedTestRoot)
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $interactiveUser = (Get-CimInstance Win32_ComputerSystem).UserName
    $os = Get-CimInstance Win32_OperatingSystem
    $sessionLines = @(& quser.exe 2>$null | ForEach-Object { [string]$_ })
    $adapter = @(Get-NetIPConfiguration | Where-Object { $_.IPv4Address } | ForEach-Object {
      [ordered]@{
        alias = $_.InterfaceAlias
        addresses = @($_.IPv4Address.IPAddress)
        gateway = $_.IPv4DefaultGateway.NextHop
        dns = @($_.DNSServer.ServerAddresses)
      }
    })
    [ordered]@{
      computerName = $env:COMPUTERNAME
      identity = $identity.Name
      sessionId = [Diagnostics.Process]::GetCurrentProcess().SessionId
      interactiveUser = $interactiveUser
      interactiveAccountMatches = -not [string]::IsNullOrWhiteSpace($interactiveUser) -and $interactiveUser.EndsWith("\$ExpectedAccount", [StringComparison]::OrdinalIgnoreCase)
      os = [ordered]@{
        caption = $os.Caption
        version = $os.Version
        buildNumber = $os.BuildNumber
      }
      testRoot = [ordered]@{
        expected = $ExpectedTestRoot
        exists = Test-Path -LiteralPath $ExpectedTestRoot -PathType Container
      }
      vmicvmsession = (Get-Service vmicvmsession).Status.ToString()
      sessions = $sessionLines
      network = $adapter
    }
  } -ArgumentList @($GuestAccount, $GuestTestRoot)

  [ordered]@{
    schema = 'chemsema.gui.worker-attestation.v1'
    operation = 'guest-attest'
    vmId = (Get-WorkerVm).Id.ToString()
    vmName = (Get-WorkerVm).Name
    guest = $result
  }
}

function Prepare-Guest {
  $result = Invoke-Guest -ScriptBlock {
    param($ExpectedAccount, $ExpectedTestRoot)
    if ([string]::IsNullOrWhiteSpace($ExpectedTestRoot) -or $ExpectedTestRoot -notmatch '^[A-Za-z]:\\[^\\]+') {
      throw 'Guest test root must be a bounded absolute path below a drive root.'
    }
    $resolvedParent = [IO.Path]::GetFullPath((Split-Path -Parent $ExpectedTestRoot))
    $resolvedRoot = [IO.Path]::GetFullPath($ExpectedTestRoot)
    if ($resolvedRoot -eq ([IO.Path]::GetPathRoot($resolvedRoot)) -or -not $resolvedRoot.StartsWith($resolvedParent, [StringComparison]::OrdinalIgnoreCase)) {
      throw 'Guest test root failed bounded-path validation.'
    }
    New-Item -ItemType Directory -Path $resolvedRoot -Force | Out-Null
    [ordered]@{
      identity = [Security.Principal.WindowsIdentity]::GetCurrent().Name
      expectedAccount = $ExpectedAccount
      testRoot = $resolvedRoot
      exists = Test-Path -LiteralPath $resolvedRoot -PathType Container
    }
  } -ArgumentList @($GuestAccount, $GuestTestRoot)

  [ordered]@{
    schema = 'chemsema.gui.worker-attestation.v1'
    operation = 'prepare-guest'
    vmId = (Get-WorkerVm).Id.ToString()
    vmName = (Get-WorkerVm).Name
    guest = $result
  }
}

function Install-Agent {
  if ([string]::IsNullOrWhiteSpace($HostAgentPath) -or -not (Test-Path -LiteralPath $HostAgentPath -PathType Leaf)) {
    throw 'The built guest agent executable is unavailable.'
  }
  $credential = Get-GuestCredential
  $vm = Get-WorkerVm
  $session = New-PSSession -VMId $vm.Id -Credential $credential
  try {
    $guestAgentDirectory = Join-Path $GuestTestRoot 'agent'
    $guestAgentPath = Join-Path $guestAgentDirectory 'chemsema-gui-test-agent.exe'
    Invoke-Command -Session $session -ScriptBlock {
      param($Directory)
      New-Item -ItemType Directory -Path $Directory -Force | Out-Null
    } -ArgumentList $guestAgentDirectory
    Copy-Item -LiteralPath $HostAgentPath -Destination $guestAgentPath -ToSession $session -Force
    $guestHash = Invoke-Command -Session $session -ScriptBlock {
      param($Path)
      (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    } -ArgumentList $guestAgentPath
    $hostHash = (Get-FileHash -LiteralPath $HostAgentPath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($guestHash -ne $hostHash) {
      throw 'Guest agent SHA-256 does not match the host build.'
    }
    [ordered]@{
      schema = 'chemsema.gui.worker-attestation.v1'
      operation = 'install-agent'
      vmId = $vm.Id.ToString()
      vmName = $vm.Name
      agent = [ordered]@{
        guestPath = $guestAgentPath
        sha256 = [string]$guestHash
        bytes = (Get-Item -LiteralPath $HostAgentPath).Length
      }
    }
  }
  finally {
    Remove-PSSession -Session $session -ErrorAction SilentlyContinue
  }
}

function Get-ServiceAgentAttestation {
  $agentPath = Join-Path (Join-Path $GuestTestRoot 'agent') 'chemsema-gui-test-agent.exe'
  $result = Invoke-Guest -ScriptBlock {
    param($Path)
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
      throw 'Guest agent is not installed.'
    }
    $json = & $Path attest
    if ($LASTEXITCODE -ne 0) {
      throw 'Guest agent attest command failed.'
    }
    $json | ConvertFrom-Json
  } -ArgumentList @($agentPath)
  $cleanForeground = if ($null -eq $result.foreground) {
    $null
  }
  else {
    [ordered]@{
      windowHandle = [UInt64]$result.foreground.windowHandle
      processId = [UInt32]$result.foreground.processId
      sessionId = [UInt32]$result.foreground.sessionId
      executable = [string]$result.foreground.executable
      title = [string]$result.foreground.title
      className = [string]$result.foreground.className
      rect = @($result.foreground.rect | ForEach-Object { [int]$_ })
    }
  }
  $cleanAgent = [ordered]@{
    schema = [string]$result.schema
    agentVersion = [string]$result.agentVersion
    processId = [UInt32]$result.processId
    sessionId = [UInt32]$result.sessionId
    account = [string]$result.account
    inputDesktop = if ($null -eq $result.inputDesktop) { $null } else { [string]$result.inputDesktop }
    interactiveReady = [bool]$result.interactiveReady
    foreground = $cleanForeground
  }
  [ordered]@{
    schema = 'chemsema.gui.worker-attestation.v1'
    operation = 'agent-attest-service'
    vmId = (Get-WorkerVm).Id.ToString()
    vmName = (Get-WorkerVm).Name
    agent = $cleanAgent
  }
}

function Stop-Worker {
  $vm = Get-WorkerVm
  $stoppedByCoordinator = $false
  if ($vm.State -eq 'Running') {
    Stop-VM -VM $vm
    $stoppedByCoordinator = $true
    $deadline = [DateTime]::UtcNow.AddSeconds(120)
    do {
      Start-Sleep -Milliseconds 500
      $vm = Get-WorkerVm
    } while ($vm.State -ne 'Off' -and [DateTime]::UtcNow -lt $deadline)
    if ($vm.State -ne 'Off') {
      throw "Worker VM '$VmId' did not shut down cleanly within 120 seconds."
    }
  }
  [ordered]@{
    schema = 'chemsema.gui.worker-attestation.v1'
    operation = 'stop'
    vmId = $vm.Id.ToString()
    vmName = $vm.Name
    state = $vm.State.ToString()
    stoppedByCoordinator = $stoppedByCoordinator
  }
}

switch ($Operation) {
  'host-attest' { Write-Result (Get-HostAttestation) }
  'start' { Write-Result (Start-Worker) }
  'guest-attest' { Write-Result (Get-GuestAttestation) }
  'prepare-guest' { Write-Result (Prepare-Guest) }
  'install-agent' { Write-Result (Install-Agent) }
  'agent-attest-service' { Write-Result (Get-ServiceAgentAttestation) }
  'stop' { Write-Result (Stop-Worker) }
}
