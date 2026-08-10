param(
  [Parameter(Mandatory = $true)]
  [ValidateSet('host-attest', 'reset', 'start', 'guest-attest', 'prepare-guest', 'install-agent', 'configure-autologon', 'configure-desktop-baseline', 'install-candidate', 'launch-candidate', 'dismiss-known-blocker', 'activate-candidate', 'start-input-agent', 'stop-input-agent', 'start-cdp-agent', 'stop-cdp-agent', 'uia-query', 'cdp-bridge', 'fetch-artifacts', 'prepare-document-output', 'fetch-document-output', 'action-transaction', 'input-click', 'input-drag', 'input-key', 'input-text', 'agent-attest-service', 'agent-attest-interactive', 'stop')]
  [string]$Operation,

  [Parameter(Mandatory = $true)]
  [string]$VmId,

  [string]$CheckpointId,

  [string]$CredentialPath,
  [string]$GuestAccount,
  [string]$GuestTestRoot,
  [string]$HostAgentPath,
  [string]$HostCandidatePath,
  [string]$HostCdpScriptPath,
  [string]$CdpRequestBase64,
  [string]$ArtifactManifestBase64,
  [string]$HostArtifactRoot,
  [string]$DocumentOutputId,
  [string]$DocumentOutputName,
  [string]$ActionRequestBase64,
  [string]$AutomationName,
  [string]$AutomationId,
  [string]$AutomationControlType,
  [string]$AutomationScopeName,
  [int]$InputX,
  [int]$InputY,
  [int]$InputFromX,
  [int]$InputFromY,
  [int]$InputToX,
  [int]$InputToY,
  [int]$InputSteps = 8,
  [string]$InputKey,
  [string]$InputTextBase64,
  [string]$InputModifiers,
  [ValidateSet('left', 'right', 'middle')]
  [string]$InputButton = 'left'
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$script:Utf8WithoutBom = [Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = $script:Utf8WithoutBom
$OutputEncoding = $script:Utf8WithoutBom

function Write-Result([object]$Value) {
  $Value | ConvertTo-Json -Depth 16 -Compress
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
  $checkpoint = Get-VMSnapshot -VM $vm -ErrorAction SilentlyContinue | Where-Object { $_.Id.ToString() -eq $CheckpointId } | Select-Object -First 1
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
      checkpointId = if ($null -eq $checkpoint) { $null } else { $checkpoint.Id.ToString() }
      checkpointName = if ($null -eq $checkpoint) { $null } else { $checkpoint.Name }
    }
    credential = [ordered]@{
      configured = -not [string]::IsNullOrWhiteSpace($CredentialPath)
      exists = if ([string]::IsNullOrWhiteSpace($CredentialPath)) { $false } else { Test-Path -LiteralPath $CredentialPath -PathType Leaf }
    }
  }
}

function Reset-Worker {
  $vm = Get-WorkerVm
  if ($vm.State -ne 'Off') { throw 'Worker must be off before deterministic reset.' }
  if ($vm.AutomaticCheckpointsEnabled) { throw 'Automatic checkpoints must be disabled for deterministic reset.' }
  $checkpoint = Get-VMSnapshot -VM $vm -ErrorAction Stop | Where-Object { $_.Id.ToString() -eq $CheckpointId } | Select-Object -First 1
  if ($null -eq $checkpoint) { throw 'Configured deterministic baseline checkpoint is unavailable.' }
  Restore-VMSnapshot -VMSnapshot $checkpoint -Confirm:$false
  $restored = Get-VM -Id $vm.Id
  if ($restored.State -ne 'Off') { throw 'Deterministic reset did not leave the worker off.' }
  [ordered]@{
    schema = 'chemsema.gui.worker-attestation.v1'
    operation = 'reset'
    vmId = $restored.Id.ToString()
    vmName = $restored.Name
    state = $restored.State.ToString()
    checkpoint = [ordered]@{ id = $checkpoint.Id.ToString(); name = $checkpoint.Name }
  }
}

function Get-GuestCredential {
  if ([string]::IsNullOrWhiteSpace($CredentialPath) -or -not (Test-Path -LiteralPath $CredentialPath -PathType Leaf)) {
    throw 'The encrypted PowerShell Direct credential file is unavailable.'
  }
  Import-Clixml -LiteralPath $CredentialPath
}

function Invoke-Guest([scriptblock]$ScriptBlock, [object[]]$ArgumentList = @()) {
  $persistentSession = Get-Variable -Name ChemSemaGuiPersistentSession -Scope Global -ValueOnly -ErrorAction SilentlyContinue
  if ($null -ne $persistentSession) {
    if ($persistentSession.State -ne 'Opened') { throw 'Persistent PowerShell Direct session is not open.' }
    return Invoke-Command -Session $persistentSession -ScriptBlock $ScriptBlock -ArgumentList $ArgumentList
  }
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

function Install-Candidate {
  if ([string]::IsNullOrWhiteSpace($HostCandidatePath) -or -not (Test-Path -LiteralPath $HostCandidatePath -PathType Leaf)) {
    throw 'The built desktop candidate executable is unavailable.'
  }
  $credential = Get-GuestCredential
  $vm = Get-WorkerVm
  $hostHash = (Get-FileHash -LiteralPath $HostCandidatePath -Algorithm SHA256).Hash.ToLowerInvariant()
  $session = New-PSSession -VMId $vm.Id -Credential $credential
  try {
    $guestDirectory = Join-Path (Join-Path $GuestTestRoot 'candidate') $hostHash
    $guestPath = Join-Path $guestDirectory 'chemsema-desktop.exe'
    $existingHash = Invoke-Command -Session $session -ScriptBlock {
      param($Directory, $Path)
      New-Item -ItemType Directory -Path $Directory -Force | Out-Null
      if (Test-Path -LiteralPath $Path -PathType Leaf) {
        (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
      }
    } -ArgumentList @($guestDirectory, $guestPath)
    if ($null -ne $existingHash -and [string]$existingHash -ne $hostHash) {
      throw 'Existing content-addressed guest candidate does not match its directory hash.'
    }
    $reused = $null -ne $existingHash
    if (-not $reused) {
      Copy-Item -LiteralPath $HostCandidatePath -Destination $guestPath -ToSession $session
    }
    $guestHash = Invoke-Command -Session $session -ScriptBlock {
      param($Path)
      (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    } -ArgumentList $guestPath
    if ($guestHash -ne $hostHash) {
      throw 'Guest desktop candidate SHA-256 does not match the host build.'
    }
    [ordered]@{
      schema = 'chemsema.gui.worker-attestation.v1'
      operation = 'install-candidate'
      vmId = $vm.Id.ToString()
      vmName = $vm.Name
      candidate = [ordered]@{
        guestPath = $guestPath
        sha256 = [string]$guestHash
        bytes = (Get-Item -LiteralPath $HostCandidatePath).Length
        reused = $reused
      }
    }
  }
  finally {
    Remove-PSSession -Session $session -ErrorAction SilentlyContinue
  }
}

function Start-Candidate {
  if ([string]::IsNullOrWhiteSpace($HostCandidatePath) -or -not (Test-Path -LiteralPath $HostCandidatePath -PathType Leaf)) {
    throw 'The built desktop candidate executable is unavailable.'
  }
  $hostHash = (Get-FileHash -LiteralPath $HostCandidatePath -Algorithm SHA256).Hash.ToLowerInvariant()
  $guestPath = Join-Path (Join-Path (Join-Path $GuestTestRoot 'candidate') $hostHash) 'chemsema-desktop.exe'
  $taskName = "ChemSema GUI Candidate $($hostHash.Substring(0, 12))"
  $result = Invoke-Guest -ScriptBlock {
    param($ExpectedAccount, $CandidatePath, $ExpectedHash, $TaskName)
    if (-not (Test-Path -LiteralPath $CandidatePath -PathType Leaf)) {
      throw 'The content-addressed desktop candidate is not installed.'
    }
    $actualHash = (Get-FileHash -LiteralPath $CandidatePath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualHash -ne $ExpectedHash) {
      throw 'Installed desktop candidate failed launch-time SHA-256 verification.'
    }
    $candidateProcesses = @(Get-Process chemsema-desktop -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $CandidatePath })
    $ownedProcessIds = [Collections.Generic.HashSet[int]]::new()
    foreach ($candidateProcess in $candidateProcesses) { [void]$ownedProcessIds.Add([int]$candidateProcess.Id) }
    do {
      $added = $false
      foreach ($child in @(Get-CimInstance Win32_Process | Where-Object { $ownedProcessIds.Contains([int]$_.ParentProcessId) })) {
        if ($ownedProcessIds.Add([int]$child.ProcessId)) { $added = $true }
      }
    } while ($added)
    foreach ($ownedId in @($ownedProcessIds | Sort-Object -Descending)) {
      Stop-Process -Id $ownedId -Force -ErrorAction SilentlyContinue
    }
    if ($ownedProcessIds.Count -gt 0) {
      $exitDeadline = [DateTime]::UtcNow.AddSeconds(20)
      do {
        $remaining = @(Get-Process -Id @($ownedProcessIds) -ErrorAction SilentlyContinue)
        if ($remaining.Count -eq 0) { break }
        Start-Sleep -Milliseconds 200
      } while ([DateTime]::UtcNow -lt $exitDeadline)
      if ($remaining.Count -ne 0) { throw 'Previous candidate process tree did not exit within 20 seconds.' }
    }
    $launchScript = Join-Path (Split-Path -Parent $CandidatePath) 'launch-gui-test-candidate.ps1'
    $launchSource = @'
param([string]$CandidatePath)
$logPath = Join-Path (Split-Path -Parent $CandidatePath) 'webview.log'
Remove-Item -LiteralPath $logPath -Force -ErrorAction SilentlyContinue
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS="--force-renderer-accessibility --remote-debugging-port=9223 --enable-logging --log-file=$logPath --v=1"
& $CandidatePath
'@
    [IO.File]::WriteAllText($launchScript, $launchSource, [Text.UTF8Encoding]::new($false))
    $launchArguments = "-NoLogo -NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File `"$launchScript`" -CandidatePath `"$CandidatePath`""
    $action = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument $launchArguments
    $principal = New-ScheduledTaskPrincipal -UserId "$env:COMPUTERNAME\$ExpectedAccount" -LogonType Interactive -RunLevel Limited
    $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Hours 12) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
    Register-ScheduledTask -TaskName $TaskName -Action $action -Principal $principal -Settings $settings -Force | Out-Null
    try {
      Start-ScheduledTask -TaskName $TaskName
      $deadline = [DateTime]::UtcNow.AddSeconds(60)
      do {
        $process = Get-Process chemsema-desktop -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $CandidatePath } | Select-Object -First 1
        if ($null -ne $process -and $process.SessionId -ne 0) { break }
        Start-Sleep -Milliseconds 250
      } while ([DateTime]::UtcNow -lt $deadline)
      if ($null -eq $process -or $process.SessionId -eq 0) {
        throw 'Desktop candidate did not start in the interactive session within 60 seconds.'
      }
      [ordered]@{
        guestPath = $CandidatePath
        sha256 = $actualHash
        processId = [int]$process.Id
        sessionId = [int]$process.SessionId
      }
    }
    finally {
      Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue
    }
  } -ArgumentList @($GuestAccount, $guestPath, $hostHash, $taskName)
  [ordered]@{
    schema = 'chemsema.gui.worker-attestation.v1'
    operation = 'launch-candidate'
    vmId = (Get-WorkerVm).Id.ToString()
    vmName = (Get-WorkerVm).Name
    candidate = $result
  }
}

function Activate-Candidate {
  if ([string]::IsNullOrWhiteSpace($HostCandidatePath) -or -not (Test-Path -LiteralPath $HostCandidatePath -PathType Leaf)) {
    throw 'The built desktop candidate executable is unavailable.'
  }
  $hostHash = (Get-FileHash -LiteralPath $HostCandidatePath -Algorithm SHA256).Hash.ToLowerInvariant()
  $guestPath = Join-Path (Join-Path (Join-Path $GuestTestRoot 'candidate') $hostHash) 'chemsema-desktop.exe'
  $agentPath = Join-Path (Join-Path $GuestTestRoot 'agent') 'chemsema-gui-test-agent.exe'
  $result = Invoke-Guest -ScriptBlock {
    param($ExpectedAccount, $TestRoot, $AgentPath, $CandidatePath)
    $process = Get-Process chemsema-desktop -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $CandidatePath -and $_.SessionId -ne 0 } | Select-Object -First 1
    if ($null -eq $process) {
      throw 'The authorized desktop candidate is not running in an interactive session.'
    }
    $runRoot = Join-Path $TestRoot 'runs'
    $runDirectory = Join-Path $runRoot ("activate-" + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $runDirectory -Force | Out-Null
    $guardPath = Join-Path $runDirectory 'guard.json'
    $resultPath = Join-Path $runDirectory 'result.json'
    $guardJson = [ordered]@{
      expectedAgentSessionId = [int]$process.SessionId
      expectedProcessId = [int]$process.Id
      expectedExecutable = $CandidatePath
      allowedRunRoot = $runRoot
      runDirectory = $runDirectory
    } | ConvertTo-Json
    [IO.File]::WriteAllText($guardPath, $guardJson, [Text.UTF8Encoding]::new($false))
    $taskName = "ChemSema GUI Activate $($process.Id)"
    $arguments = "activate --guard `"$guardPath`" --output `"$resultPath`""
    $action = New-ScheduledTaskAction -Execute $AgentPath -Argument $arguments
    $principal = New-ScheduledTaskPrincipal -UserId "$env:COMPUTERNAME\$ExpectedAccount" -LogonType Interactive -RunLevel Highest
    $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Minutes 2) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
    Register-ScheduledTask -TaskName $taskName -Action $action -Principal $principal -Settings $settings -Force | Out-Null
    try {
      Start-ScheduledTask -TaskName $taskName
      $deadline = [DateTime]::UtcNow.AddSeconds(12)
      do {
        if (Test-Path -LiteralPath $resultPath -PathType Leaf) { break }
        $task = Get-ScheduledTask -TaskName $taskName
        $info = Get-ScheduledTaskInfo -TaskName $taskName
        if ($task.State -eq 'Ready' -and $info.LastRunTime -gt [DateTime]::MinValue -and $info.LastTaskResult -ne 267009) { break }
        Start-Sleep -Milliseconds 250
      } while ([DateTime]::UtcNow -lt $deadline)
      if (-not (Test-Path -LiteralPath $resultPath -PathType Leaf)) {
        throw "Interactive activation agent failed with task result $($info.LastTaskResult) within 12 seconds."
      }
      $agentResult = Get-Content -Raw -Encoding UTF8 -LiteralPath $resultPath | ConvertFrom-Json
      if ($agentResult.status -eq 'failed') {
        throw "Interactive activation agent rejected the request: $($agentResult.message)"
      }
      $agentResult
    }
    finally {
      Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue
    }
  } -ArgumentList @($GuestAccount, $GuestTestRoot, $agentPath, $guestPath)
  [ordered]@{
    schema = 'chemsema.gui.worker-attestation.v1'
    operation = 'activate-candidate'
    vmId = (Get-WorkerVm).Id.ToString()
    vmName = (Get-WorkerVm).Name
    candidate = [ordered]@{
      guestPath = $guestPath
      sha256 = $hostHash
    }
    agent = $result
  }
}

function Dismiss-KnownBlocker {
  $agentPath = Join-Path (Join-Path $GuestTestRoot 'agent') 'chemsema-gui-test-agent.exe'
  $resultPath = Join-Path $GuestTestRoot 'dismiss-known-blocker.json'
  $taskName = 'ChemSema GUI Dismiss Known Blocker'
  $result = Invoke-Guest -ScriptBlock {
    param($ExpectedAccount, $AgentPath, $ResultPath, $TaskName)
    Remove-Item -LiteralPath $ResultPath -Force -ErrorAction SilentlyContinue
    $arguments = "dismiss-known-blocker --output `"$ResultPath`""
    $action = New-ScheduledTaskAction -Execute $AgentPath -Argument $arguments
    $principal = New-ScheduledTaskPrincipal -UserId "$env:COMPUTERNAME\$ExpectedAccount" -LogonType Interactive -RunLevel Highest
    Register-ScheduledTask -TaskName $TaskName -Action $action -Principal $principal -Force | Out-Null
    try {
      Start-ScheduledTask -TaskName $TaskName
      $deadline = [DateTime]::UtcNow.AddSeconds(20)
      do {
        if (Test-Path -LiteralPath $ResultPath -PathType Leaf) { break }
        Start-Sleep -Milliseconds 250
      } while ([DateTime]::UtcNow -lt $deadline)
      if (-not (Test-Path -LiteralPath $ResultPath -PathType Leaf)) { throw 'Blocker dismissal agent returned no receipt.' }
      $agentResult = Get-Content -Raw -Encoding UTF8 -LiteralPath $ResultPath | ConvertFrom-Json
      if ($agentResult.status -eq 'failed') { throw "Blocker dismissal was rejected: $($agentResult.message)" }
      $agentResult
    }
    finally {
      Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue
    }
  } -ArgumentList @($GuestAccount, $agentPath, $resultPath, $taskName)
  [ordered]@{
    schema = 'chemsema.gui.worker-attestation.v1'
    operation = 'dismiss-known-blocker'
    vmId = (Get-WorkerVm).Id.ToString()
    vmName = (Get-WorkerVm).Name
    agent = $result
  }
}

function Query-Uia {
  if ([string]::IsNullOrWhiteSpace($AutomationName) -and [string]::IsNullOrWhiteSpace($AutomationId)) { throw 'UI Automation query requires an exact accessible name or automation id.' }
  $hostHash = (Get-FileHash -LiteralPath $HostCandidatePath -Algorithm SHA256).Hash.ToLowerInvariant()
  $guestPath = Join-Path (Join-Path (Join-Path $GuestTestRoot 'candidate') $hostHash) 'chemsema-desktop.exe'
  $result = Invoke-Guest -ScriptBlock {
    param($ExpectedAccount, $CandidatePath, $Name, $AutomationId, $ControlType, $ScopeName, $TestRoot)
    $process = Get-Process chemsema-desktop -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $CandidatePath -and $_.SessionId -ne 0 } | Select-Object -First 1
    if ($null -eq $process) { throw 'The authorized desktop candidate is not running.' }
    $runDirectory = Join-Path (Join-Path $TestRoot 'runs') ("uia-" + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $runDirectory -Force | Out-Null
    $scriptPath = Join-Path $runDirectory 'query.ps1'
    $resultPath = Join-Path $runDirectory 'result.json'
    $script = @'
param([int]$TargetProcessId, [string]$ExactName, [string]$ExactAutomationId, [string]$ExpectedControlType, [string]$ScopeName, [string]$OutputPath)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
try {
$processCondition=[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::ProcessIdProperty,$TargetProcessId)
$roots=@([Windows.Automation.AutomationElement]::RootElement.FindAll([Windows.Automation.TreeScope]::Children,$processCondition) |
  Where-Object { -not $_.Current.IsOffscreen -and $_.Current.BoundingRectangle.Width -gt 0 -and $_.Current.BoundingRectangle.Height -gt 0 })
if($roots.Count -eq 0){throw 'Candidate top-level UI Automation element is absent.'}
$topLevels=@($roots | ForEach-Object {
  $rect=$_.Current.BoundingRectangle
  [ordered]@{
    name=$_.Current.Name
    automationId=$_.Current.AutomationId
    className=$_.Current.ClassName
    offscreen=$_.Current.IsOffscreen
    rect=@([int][Math]::Round($rect.Left),[int][Math]::Round($rect.Top),[int][Math]::Round($rect.Right),[int][Math]::Round($rect.Bottom))
  }
})
$scopeCondition=if([string]::IsNullOrWhiteSpace($ScopeName)){$null}else{[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::NameProperty,$ScopeName)}
$conditions=@()
if(-not [string]::IsNullOrWhiteSpace($ExactName) -and $ExactName -ne '*'){$conditions += [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::NameProperty,$ExactName)}
if(-not [string]::IsNullOrWhiteSpace($ExactAutomationId)){$conditions += [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::AutomationIdProperty,$ExactAutomationId)}
$nameCondition=if($conditions.Count -eq 0){[Windows.Automation.Condition]::TrueCondition}elseif($conditions.Count -eq 1){$conditions[0]}else{[Windows.Automation.AndCondition]::new([Windows.Automation.Condition[]]$conditions)}
$matches=@()
foreach($root in $roots){
  $searchRoots=if($null -eq $scopeCondition){@($root)}else{@($root.FindAll([Windows.Automation.TreeScope]::Descendants,$scopeCondition))}
  foreach($searchRoot in $searchRoots){
    $elements=$searchRoot.FindAll([Windows.Automation.TreeScope]::Descendants,$nameCondition)
    foreach($element in $elements){
      if($matches.Count -ge 200){break}
      $rect=$element.Current.BoundingRectangle
      $coordinates=@($rect.Left,$rect.Top,$rect.Right,$rect.Bottom)
      if($coordinates | Where-Object { [double]::IsNaN($_) -or [double]::IsInfinity($_) }){continue}
      if($rect.Width -le 0 -or $rect.Height -le 0){continue}
      if(-not [string]::IsNullOrWhiteSpace($ExpectedControlType) -and $element.Current.ControlType.ProgrammaticName -ne $ExpectedControlType){continue}
      $matches += [ordered]@{
        name=$element.Current.Name
        automationId=$element.Current.AutomationId
        className=$element.Current.ClassName
        controlType=$element.Current.ControlType.ProgrammaticName
        enabled=$element.Current.IsEnabled
        offscreen=$element.Current.IsOffscreen
        hasKeyboardFocus=$element.Current.HasKeyboardFocus
        topLevelName=$root.Current.Name
        topLevelClassName=$root.Current.ClassName
        rect=@([int][Math]::Round($rect.Left),[int][Math]::Round($rect.Top),[int][Math]::Round($rect.Right),[int][Math]::Round($rect.Bottom))
      }
    }
  }
}
$json=[ordered]@{schema='chemsema.gui.uia-query.v1';processId=$TargetProcessId;name=$ExactName;automationId=$ExactAutomationId;controlType=$ExpectedControlType;topLevels=$topLevels;matches=$matches}|ConvertTo-Json -Depth 6
[IO.File]::WriteAllText($OutputPath,$json,[Text.UTF8Encoding]::new($false))
} catch {
  $json=[ordered]@{schema='chemsema.gui.uia-query.v1';status='failed';message=$_.Exception.Message}|ConvertTo-Json
  [IO.File]::WriteAllText($OutputPath,$json,[Text.UTF8Encoding]::new($false))
  exit 1
}
'@
    [IO.File]::WriteAllText($scriptPath, $script, [Text.UTF8Encoding]::new($false))
    $taskName = "ChemSema GUI UIA Query $($process.Id)"
    $arguments = "-NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File `"$scriptPath`" -TargetProcessId $($process.Id) -ExactName `"$Name`" -ExactAutomationId `"$AutomationId`" -ExpectedControlType `"$ControlType`" -ScopeName `"$ScopeName`" -OutputPath `"$resultPath`""
    $action = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument $arguments
    $principal = New-ScheduledTaskPrincipal -UserId "$env:COMPUTERNAME\$ExpectedAccount" -LogonType Interactive -RunLevel Highest
    Register-ScheduledTask -TaskName $taskName -Action $action -Principal $principal -Force | Out-Null
    try {
      Start-ScheduledTask -TaskName $taskName
      $deadline = [DateTime]::UtcNow.AddSeconds(30)
      do {
        if (Test-Path -LiteralPath $resultPath -PathType Leaf) { break }
        Start-Sleep -Milliseconds 250
      } while ([DateTime]::UtcNow -lt $deadline)
      if (-not (Test-Path -LiteralPath $resultPath -PathType Leaf)) { throw 'Interactive UI Automation query returned no receipt.' }
      $queryResult = Get-Content -Raw -Encoding UTF8 -LiteralPath $resultPath | ConvertFrom-Json
      if ($queryResult.status -eq 'failed') { throw "Interactive UI Automation query failed: $($queryResult.message)" }
      $queryResult
    }
    finally { Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue }
  } -ArgumentList @($GuestAccount, $guestPath, $AutomationName, $AutomationId, $AutomationControlType, $AutomationScopeName, $GuestTestRoot)
  [ordered]@{
    schema = 'chemsema.gui.worker-attestation.v1'
    operation = 'uia-query'
    vmId = (Get-WorkerVm).Id.ToString()
    vmName = (Get-WorkerVm).Name
    query = $result
  }
}

function Invoke-CdpBridge {
  if ([string]::IsNullOrWhiteSpace($CdpRequestBase64)) { throw 'The CDP bridge request is absent.' }
  $decodedRequest = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($CdpRequestBase64)) | ConvertFrom-Json
  $receiptTimeoutSeconds = if ($decodedRequest.mode -eq 'artifact-export') { 90 } else { 20 }
  $result = Invoke-Guest -ScriptBlock {
    param($TestRoot, $RequestBase64, $ReceiptTimeoutSeconds)
    $channelRoot = Join-Path $TestRoot 'cdp-channel'
    if (-not (Test-Path -LiteralPath (Join-Path $channelRoot 'ready.json') -PathType Leaf)) { throw 'Persistent CDP agent is not ready.' }
    $requestId = [Guid]::NewGuid().ToString('N')
    $inbox = Join-Path $channelRoot 'inbox'
    $outbox = Join-Path $channelRoot 'outbox'
    $requestPath = Join-Path $inbox "$requestId.json"
    $temporaryRequest = Join-Path $inbox "$requestId.tmp"
    $responsePath = Join-Path $outbox "$requestId.json"
    $request = [ordered]@{ schema='chemsema.gui.cdp-request.v1'; id=$requestId; requestBase64=$RequestBase64 }
    [IO.File]::WriteAllText($temporaryRequest, ($request | ConvertTo-Json -Compress), [Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $temporaryRequest -Destination $requestPath
    $deadline = [DateTime]::UtcNow.AddSeconds($ReceiptTimeoutSeconds)
    while (-not (Test-Path -LiteralPath $responsePath -PathType Leaf) -and [DateTime]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 20 }
    if (-not (Test-Path -LiteralPath $responsePath -PathType Leaf)) { throw "Persistent CDP agent returned no receipt within $ReceiptTimeoutSeconds seconds." }
    $response = Get-Content -Raw -Encoding UTF8 -LiteralPath $responsePath | ConvertFrom-Json
    if ($response.schema -ne 'chemsema.gui.cdp-response.v1' -or $response.id -ne $requestId) { throw 'Persistent CDP response identity is invalid.' }
    if ($response.status -ne 'passed') { throw "Persistent CDP request failed: $($response.message)" }
    $response.bridge
  } -ArgumentList @($GuestTestRoot, $CdpRequestBase64, $receiptTimeoutSeconds)
  [ordered]@{
    schema = 'chemsema.gui.worker-attestation.v1'
    operation = 'cdp-bridge'
    vmId = (Get-WorkerVm).Id.ToString()
    vmName = (Get-WorkerVm).Name
    bridge = $result
  }
}

function Receive-GuestArtifacts {
  if ([string]::IsNullOrWhiteSpace($ArtifactManifestBase64)) { throw 'The guest artifact manifest is absent.' }
  if ([string]::IsNullOrWhiteSpace($HostArtifactRoot)) { throw 'The host artifact staging root is absent.' }
  $manifest = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($ArtifactManifestBase64)) | ConvertFrom-Json
  if ($manifest.schema -ne 'chemsema.gui.guest-artifact-export.v1' -or [string]$manifest.artifactId -notmatch '^[a-f0-9]{32}$') {
    throw 'The guest artifact export manifest is invalid.'
  }
  $resolvedHostRoot = [IO.Path]::GetFullPath($HostArtifactRoot)
  if ($resolvedHostRoot -eq [IO.Path]::GetPathRoot($resolvedHostRoot) -or -not (Test-Path -LiteralPath $resolvedHostRoot -PathType Container)) {
    throw 'The host artifact staging root is not a bounded existing directory.'
  }
  $guestExportRoot = [IO.Path]::GetFullPath((Join-Path (Join-Path $GuestTestRoot 'artifacts') ([string]$manifest.artifactId))).TrimEnd('\') + '\'
  $seenNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
  $validated = @()
  foreach ($artifact in @($manifest.artifacts)) {
    $name = [string]$artifact.name
    $guestPath = [IO.Path]::GetFullPath([string]$artifact.guestPath)
    if ($name -notmatch '^[a-z0-9][a-z0-9._-]{0,127}$' -or -not $seenNames.Add($name)) { throw 'Guest artifact names must be safe and unique.' }
    if ([string]$artifact.mediaType -notmatch '^[^/]+/[^/]+$') { throw "Guest artifact $name has an invalid media type." }
    if (-not ($guestPath + '').StartsWith($guestExportRoot, [StringComparison]::OrdinalIgnoreCase) -or [IO.Path]::GetFileName($guestPath) -ne $name) {
      throw "Guest artifact $name escaped its authorized export root."
    }
    if ([int64]$artifact.size -lt 0 -or [int64]$artifact.size -gt (64 * 1024 * 1024) -or [string]$artifact.sha256 -notmatch '^[a-f0-9]{64}$') {
      throw "Guest artifact $name has invalid size or content identity."
    }
    $validated += [ordered]@{ name=$name; mediaType=[string]$artifact.mediaType; guestPath=$guestPath; size=[int64]$artifact.size; sha256=[string]$artifact.sha256 }
  }
  if ($validated.Count -eq 0 -or $validated.Count -gt 16) { throw 'Guest artifact export must contain between one and sixteen payloads.' }

  $credential = Get-GuestCredential
  $vm = Get-WorkerVm
  $session = New-PSSession -VMId $vm.Id -Credential $credential
  try {
    $received = @()
    foreach ($artifact in $validated) {
      $guestIdentity = Invoke-Command -Session $session -ScriptBlock {
        param($Path)
        if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { throw 'Guest artifact file is absent.' }
        $item = Get-Item -LiteralPath $Path
        [ordered]@{ size=[int64]$item.Length; sha256=(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant() }
      } -ArgumentList $artifact.guestPath
      if ([int64]$guestIdentity.size -ne $artifact.size -or [string]$guestIdentity.sha256 -ne $artifact.sha256) {
        throw "Guest artifact $($artifact.name) changed before transfer."
      }
      $hostPath = Join-Path $resolvedHostRoot $artifact.name
      Copy-Item -LiteralPath $artifact.guestPath -Destination $hostPath -FromSession $session
      $hostItem = Get-Item -LiteralPath $hostPath
      $hostHash = (Get-FileHash -LiteralPath $hostPath -Algorithm SHA256).Hash.ToLowerInvariant()
      if ([int64]$hostItem.Length -ne $artifact.size -or $hostHash -ne $artifact.sha256) {
        throw "Host artifact $($artifact.name) failed SHA-256 verification after transfer."
      }
      $received += [ordered]@{ name=$artifact.name; mediaType=$artifact.mediaType; hostPath=$hostPath; size=[int64]$hostItem.Length; sha256=$hostHash }
    }
    [ordered]@{
      schema = 'chemsema.gui.worker-attestation.v1'
      operation = 'fetch-artifacts'
      vmId = $vm.Id.ToString()
      vmName = $vm.Name
      transfer = [ordered]@{ schema='chemsema.gui.host-artifact-transfer.v1'; artifactId=[string]$manifest.artifactId; artifacts=$received }
    }
  } finally {
    Remove-PSSession -Session $session -ErrorAction SilentlyContinue
  }
}

function Assert-DocumentOutputIdentity {
  if ($DocumentOutputId -notmatch '^[a-f0-9]{32}$') { throw 'Document output identity must be 32 lowercase hexadecimal characters.' }
  if ($DocumentOutputName -notmatch '^[a-z0-9][a-z0-9._-]{0,95}\.ccjs$' -or [IO.Path]::GetFileName($DocumentOutputName) -ne $DocumentOutputName) {
    throw 'Document output name must be a bounded safe CCJS filename.'
  }
  $documentsRoot = [IO.Path]::GetFullPath((Join-Path $GuestTestRoot 'documents')).TrimEnd('\') + '\'
  $directory = [IO.Path]::GetFullPath((Join-Path $documentsRoot $DocumentOutputId))
  $guestPath = [IO.Path]::GetFullPath((Join-Path $directory $DocumentOutputName))
  if (-not ($directory + '\').StartsWith($documentsRoot, [StringComparison]::OrdinalIgnoreCase) -or
      -not $guestPath.StartsWith(($directory.TrimEnd('\') + '\'), [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Document output path escaped the dedicated guest test root.'
  }
  [ordered]@{ documentsRoot=$documentsRoot; directory=$directory; guestPath=$guestPath }
}

function Prepare-DocumentOutput {
  $identity = Assert-DocumentOutputIdentity
  $prepared = Invoke-Guest -ScriptBlock {
    param($TestRoot, $Directory, $Path)
    $documentsRoot = [IO.Path]::GetFullPath((Join-Path $TestRoot 'documents')).TrimEnd('\') + '\'
    $resolvedDirectory = [IO.Path]::GetFullPath($Directory)
    $resolvedPath = [IO.Path]::GetFullPath($Path)
    if (-not ($resolvedDirectory + '\').StartsWith($documentsRoot, [StringComparison]::OrdinalIgnoreCase) -or
        -not $resolvedPath.StartsWith(($resolvedDirectory.TrimEnd('\') + '\'), [StringComparison]::OrdinalIgnoreCase)) {
      throw 'Guest document output path escaped the dedicated root.'
    }
    if (Test-Path -LiteralPath $resolvedDirectory) { Remove-Item -LiteralPath $resolvedDirectory -Recurse -Force }
    New-Item -ItemType Directory -Path $resolvedDirectory -Force | Out-Null
    [ordered]@{ guestPath=$resolvedPath; exists=(Test-Path -LiteralPath $resolvedPath -PathType Leaf) }
  } -ArgumentList @($GuestTestRoot, $identity.directory, $identity.guestPath)
  [ordered]@{
    schema = 'chemsema.gui.worker-attestation.v1'
    operation = 'prepare-document-output'
    vmId = (Get-WorkerVm).Id.ToString()
    vmName = (Get-WorkerVm).Name
    output = [ordered]@{ id=$DocumentOutputId; name=$DocumentOutputName; guestPath=[string]$prepared.guestPath; exists=[bool]$prepared.exists }
  }
}

function Receive-GuestDocumentOutput {
  if ([string]::IsNullOrWhiteSpace($HostArtifactRoot)) { throw 'The host document staging root is absent.' }
  $identity = Assert-DocumentOutputIdentity
  $resolvedHostRoot = [IO.Path]::GetFullPath($HostArtifactRoot)
  if ($resolvedHostRoot -eq [IO.Path]::GetPathRoot($resolvedHostRoot) -or -not (Test-Path -LiteralPath $resolvedHostRoot -PathType Container)) {
    throw 'The host document staging root is not a bounded existing directory.'
  }
  $credential = Get-GuestCredential
  $vm = Get-WorkerVm
  $session = New-PSSession -VMId $vm.Id -Credential $credential
  try {
    $guestIdentity = Invoke-Command -Session $session -ScriptBlock {
      param($TestRoot, $Path)
      $documentsRoot = [IO.Path]::GetFullPath((Join-Path $TestRoot 'documents')).TrimEnd('\') + '\'
      $resolvedPath = [IO.Path]::GetFullPath($Path)
      if (-not $resolvedPath.StartsWith($documentsRoot, [StringComparison]::OrdinalIgnoreCase)) { throw 'Guest document path escaped the dedicated root.' }
      $deadline = [DateTime]::UtcNow.AddSeconds(30)
      do {
        if (Test-Path -LiteralPath $resolvedPath -PathType Leaf) {
          $item = Get-Item -LiteralPath $resolvedPath
          if ([int64]$item.Length -gt 0) { break }
        }
        Start-Sleep -Milliseconds 50
      } while ([DateTime]::UtcNow -lt $deadline)
      if (-not (Test-Path -LiteralPath $resolvedPath -PathType Leaf)) { throw 'Guest document output was not created within 30 seconds.' }
      $item = Get-Item -LiteralPath $resolvedPath
      if ([int64]$item.Length -le 0 -or [int64]$item.Length -gt (64 * 1024 * 1024)) { throw 'Guest document output has an invalid size.' }
      [ordered]@{ size=[int64]$item.Length; sha256=(Get-FileHash -LiteralPath $resolvedPath -Algorithm SHA256).Hash.ToLowerInvariant() }
    } -ArgumentList @($GuestTestRoot, $identity.guestPath)
    $hostPath = Join-Path $resolvedHostRoot $DocumentOutputName
    Copy-Item -LiteralPath $identity.guestPath -Destination $hostPath -FromSession $session
    $hostItem = Get-Item -LiteralPath $hostPath
    $hostHash = (Get-FileHash -LiteralPath $hostPath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ([int64]$hostItem.Length -ne [int64]$guestIdentity.size -or $hostHash -ne [string]$guestIdentity.sha256) {
      throw 'Host document output failed SHA-256 verification after transfer.'
    }
    [ordered]@{
      schema = 'chemsema.gui.worker-attestation.v1'
      operation = 'fetch-document-output'
      vmId = $vm.Id.ToString()
      vmName = $vm.Name
      output = [ordered]@{ id=$DocumentOutputId; name=$DocumentOutputName; guestPath=$identity.guestPath; hostPath=$hostPath; size=[int64]$hostItem.Length; sha256=$hostHash }
    }
  } finally {
    Remove-PSSession -Session $session -ErrorAction SilentlyContinue
  }
}

function Start-PersistentCdpAgent {
  if ([string]::IsNullOrWhiteSpace($HostCdpScriptPath) -or -not (Test-Path -LiteralPath $HostCdpScriptPath -PathType Leaf)) {
    throw 'The guest CDP bridge script is unavailable.'
  }
  $source = Get-Content -Raw -Encoding UTF8 -LiteralPath $HostCdpScriptPath
  $result = Invoke-Guest -ScriptBlock {
    param($ExpectedAccount, $TestRoot, $ScriptSource)
    $channelRoot = Join-Path $TestRoot 'cdp-channel'
    if (-not $channelRoot.StartsWith(($TestRoot.TrimEnd('\') + '\'), [StringComparison]::OrdinalIgnoreCase)) { throw 'CDP channel path is outside test root.' }
    if (Test-Path -LiteralPath $channelRoot) { Remove-Item -LiteralPath $channelRoot -Recurse -Force }
    New-Item -ItemType Directory -Path $channelRoot -Force | Out-Null
    $agentDirectory = Join-Path $TestRoot 'agent'
    New-Item -ItemType Directory -Path $agentDirectory -Force | Out-Null
    $scriptPath = Join-Path $agentDirectory 'guest-cdp.ps1'
    [IO.File]::WriteAllText($scriptPath, $ScriptSource, [Text.UTF8Encoding]::new($false))
    $taskName = 'ChemSema GUI Persistent CDP Agent'
    $arguments = "-NoLogo -NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File `"$scriptPath`" -AllowedRoot `"$TestRoot`" -ChannelRoot `"$channelRoot`""
    $action = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument $arguments
    $principal = New-ScheduledTaskPrincipal -UserId 'SYSTEM' -LogonType ServiceAccount -RunLevel Highest
    $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Hours 12) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -Hidden
    Register-ScheduledTask -TaskName $taskName -Action $action -Principal $principal -Settings $settings -Force | Out-Null
    Start-ScheduledTask -TaskName $taskName
    $readyPath = Join-Path $channelRoot 'ready.json'
    $deadline = [DateTime]::UtcNow.AddSeconds(20)
    while (-not (Test-Path -LiteralPath $readyPath -PathType Leaf) -and [DateTime]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 50 }
    if (-not (Test-Path -LiteralPath $readyPath -PathType Leaf)) { throw 'Persistent CDP agent did not become ready.' }
    Get-Content -Raw -Encoding UTF8 -LiteralPath $readyPath | ConvertFrom-Json
  } -ArgumentList @($GuestAccount, $GuestTestRoot, $source)
  $agent = [ordered]@{
    schema = [string]$result.schema
    status = [string]$result.status
    processId = [int]$result.processId
    sessionId = [int]$result.sessionId
    account = [string]$result.account
    port = [int]$result.port
  }
  [ordered]@{ schema='chemsema.gui.worker-attestation.v1'; operation='start-cdp-agent'; vmId=(Get-WorkerVm).Id.ToString(); vmName=(Get-WorkerVm).Name; agent=$agent }
}

function Stop-PersistentCdpAgent {
  $result = Invoke-Guest -ScriptBlock {
    param($TestRoot)
    $channelRoot = Join-Path $TestRoot 'cdp-channel'
    if (Test-Path -LiteralPath $channelRoot -PathType Container) {
      New-Item -ItemType File -Path (Join-Path $channelRoot 'shutdown') -Force | Out-Null
    }
    $taskName = 'ChemSema GUI Persistent CDP Agent'
    $deadline = [DateTime]::UtcNow.AddSeconds(10)
    while ((Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue).State -eq 'Running' -and [DateTime]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 50 }
    Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue
    [ordered]@{ status='stopped' }
  } -ArgumentList @($GuestTestRoot)
  [ordered]@{ schema='chemsema.gui.worker-attestation.v1'; operation='stop-cdp-agent'; vmId=(Get-WorkerVm).Id.ToString(); vmName=(Get-WorkerVm).Name; agent=$result }
}

function Start-PersistentInputAgent {
  $agentPath = Join-Path (Join-Path $GuestTestRoot 'agent') 'chemsema-gui-test-agent.exe'
  $result = Invoke-Guest -ScriptBlock {
    param($ExpectedAccount, $AgentPath, $TestRoot)
    $channelRoot = Join-Path $TestRoot 'input-channel'
    if (-not $channelRoot.StartsWith(($TestRoot.TrimEnd('\') + '\'), [StringComparison]::OrdinalIgnoreCase)) { throw 'Input channel path is outside test root.' }
    if (Test-Path -LiteralPath $channelRoot) { Remove-Item -LiteralPath $channelRoot -Recurse -Force }
    New-Item -ItemType Directory -Path $channelRoot -Force | Out-Null
    $taskName = 'ChemSema GUI Persistent Input Agent'
    $arguments = "serve --allowed-root `"$TestRoot`" --channel-root `"$channelRoot`""
    $action = New-ScheduledTaskAction -Execute $AgentPath -Argument $arguments
    $principal = New-ScheduledTaskPrincipal -UserId "$env:COMPUTERNAME\$ExpectedAccount" -LogonType Interactive -RunLevel Highest
    $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Hours 12) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
    Register-ScheduledTask -TaskName $taskName -Action $action -Principal $principal -Settings $settings -Force | Out-Null
    Start-ScheduledTask -TaskName $taskName
    $readyPath = Join-Path $channelRoot 'ready.json'
    $deadline = [DateTime]::UtcNow.AddSeconds(20)
    while (-not (Test-Path -LiteralPath $readyPath -PathType Leaf) -and [DateTime]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 50 }
    if (-not (Test-Path -LiteralPath $readyPath -PathType Leaf)) { throw 'Persistent input agent did not become ready.' }
    Get-Content -Raw -Encoding UTF8 -LiteralPath $readyPath | ConvertFrom-Json
  } -ArgumentList @($GuestAccount, $agentPath, $GuestTestRoot)
  [ordered]@{ schema='chemsema.gui.worker-attestation.v1'; operation='start-input-agent'; vmId=(Get-WorkerVm).Id.ToString(); vmName=(Get-WorkerVm).Name; agent=$result }
}

function Stop-PersistentInputAgent {
  $result = Invoke-Guest -ScriptBlock {
    param($TestRoot)
    $channelRoot = Join-Path $TestRoot 'input-channel'
    New-Item -ItemType File -Path (Join-Path $channelRoot 'shutdown') -Force | Out-Null
    $taskName = 'ChemSema GUI Persistent Input Agent'
    $deadline = [DateTime]::UtcNow.AddSeconds(10)
    while ((Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue).State -eq 'Running' -and [DateTime]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 50 }
    Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue
    [ordered]@{ status='stopped' }
  } -ArgumentList @($GuestTestRoot)
  [ordered]@{ schema='chemsema.gui.worker-attestation.v1'; operation='stop-input-agent'; vmId=(Get-WorkerVm).Id.ToString(); vmName=(Get-WorkerVm).Name; agent=$result }
}

function Invoke-CandidateInput([ValidateSet('click', 'drag', 'key', 'text')][string]$Kind) {
  $hostHash = (Get-FileHash -LiteralPath $HostCandidatePath -Algorithm SHA256).Hash.ToLowerInvariant()
  $guestPath = Join-Path (Join-Path (Join-Path $GuestTestRoot 'candidate') $hostHash) 'chemsema-desktop.exe'
  $result = Invoke-Guest -ScriptBlock {
    param($CandidatePath, $TestRoot, $Kind, $X, $Y, $FromX, $FromY, $ToX, $ToY, $Steps, $Button, $Key, $TextBase64, $Modifiers)
    $process = Get-Process chemsema-desktop -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $CandidatePath -and $_.SessionId -ne 0 } | Select-Object -First 1
    if ($null -eq $process) { throw 'The authorized desktop candidate is not running.' }
    $runRoot = Join-Path $TestRoot 'runs'
    $runDirectory = Join-Path $runRoot ("input-$Kind-" + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $runDirectory -Force | Out-Null
    $guardPath = Join-Path $runDirectory 'guard.json'
    $guardJson = [ordered]@{
      expectedAgentSessionId = [int]$process.SessionId
      expectedProcessId = [int]$process.Id
      expectedExecutable = $CandidatePath
      allowedRunRoot = $runRoot
      runDirectory = $runDirectory
    } | ConvertTo-Json
    [IO.File]::WriteAllText($guardPath, $guardJson, [Text.UTF8Encoding]::new($false))
    $inputArguments = if ($Kind -eq 'click') {
      @('click', '--guard', $guardPath, '--x', [string]$X, '--y', [string]$Y, '--button', $Button)
    } elseif ($Kind -eq 'drag') {
      @('drag', '--guard', $guardPath, '--from-x', [string]$FromX, '--from-y', [string]$FromY, '--to-x', [string]$ToX, '--to-y', [string]$ToY, '--steps', [string]$Steps, '--button', $Button)
    } elseif ($Kind -eq 'key') {
      if ([string]::IsNullOrWhiteSpace($Key)) { throw 'Keyboard input requires a shortcut.' }
      @('key', '--guard', $guardPath, '--key', $Key)
    } else {
      if ([string]::IsNullOrWhiteSpace($TextBase64) -or $TextBase64 -notmatch '^[A-Za-z0-9+/]+={0,2}$') { throw 'Text input requires bounded base64.' }
      @('text', '--guard', $guardPath, '--text-base64', $TextBase64)
    }
    if ($Kind -in @('click', 'drag') -and -not [string]::IsNullOrWhiteSpace($Modifiers)) {
      if ($Modifiers -notmatch '^(Shift|Control|Alt)(,(Shift|Control|Alt)){0,2}$') { throw 'Pointer modifiers are not allowlisted.' }
      $inputArguments += @('--modifiers', $Modifiers)
    }
    $channelRoot = Join-Path $TestRoot 'input-channel'
    $ready = Join-Path $channelRoot 'ready.json'
    if (-not (Test-Path -LiteralPath $ready -PathType Leaf)) { throw 'Persistent input agent is not ready.' }
    $requestId = [Guid]::NewGuid().ToString('N')
    $inbox = Join-Path $channelRoot 'inbox'
    $outbox = Join-Path $channelRoot 'outbox'
    $requestPath = Join-Path $inbox "$requestId.json"
    $temporaryRequest = Join-Path $inbox "$requestId.tmp"
    $responsePath = Join-Path $outbox "$requestId.json"
    $requestJson = [ordered]@{ schema='chemsema.gui.guest-agent-request.v1'; id=$requestId; args=$inputArguments } | ConvertTo-Json -Depth 5 -Compress
    [IO.File]::WriteAllText($temporaryRequest, $requestJson, [Text.UTF8Encoding]::new($false))
    Move-Item -LiteralPath $temporaryRequest -Destination $requestPath
    $deadline = [DateTime]::UtcNow.AddSeconds(8)
    while (-not (Test-Path -LiteralPath $responsePath -PathType Leaf) -and [DateTime]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 20 }
    if (-not (Test-Path -LiteralPath $responsePath -PathType Leaf)) { throw 'Persistent input agent returned no receipt within 8 seconds.' }
    $response = Get-Content -Raw -Encoding UTF8 -LiteralPath $responsePath | ConvertFrom-Json
    if ($response.id -ne $requestId -or $response.schema -ne 'chemsema.gui.guest-agent-response.v1') { throw 'Persistent input response identity is invalid.' }
    if ($response.status -ne 'passed') { throw "Interactive input was rejected: $($response.message)" }
    $response.result
  } -ArgumentList @($guestPath, $GuestTestRoot, $Kind, $InputX, $InputY, $InputFromX, $InputFromY, $InputToX, $InputToY, $InputSteps, $InputButton, $InputKey, $InputTextBase64, $InputModifiers)
  [ordered]@{
    schema = 'chemsema.gui.worker-attestation.v1'
    operation = "input-$Kind"
    vmId = (Get-WorkerVm).Id.ToString()
    vmName = (Get-WorkerVm).Name
    candidate = [ordered]@{ guestPath = $guestPath; sha256 = $hostHash }
    agent = $result
  }
}

function Invoke-ActionTransaction {
  if ([string]::IsNullOrWhiteSpace($ActionRequestBase64)) { throw 'The action transaction request is absent.' }
  $requestJson = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($ActionRequestBase64))
  $request = $requestJson | ConvertFrom-Json
  if ($request.schema -ne 'chemsema.gui.action-transaction.v1') { throw 'Unsupported action transaction schema.' }
  if ([string]$request.actionId -notmatch '^[A-Za-z0-9._-]{1,128}$') { throw 'Action transaction identity is invalid.' }
  if ([int]$request.completion.timeoutMs + 15000 -gt [int]$request.budgetMs) { throw 'Action transaction completion timeout does not leave the required 15000 ms target-resolution and transport reserve.' }
  $hostHash = (Get-FileHash -LiteralPath $HostCandidatePath -Algorithm SHA256).Hash.ToLowerInvariant()
  $guestPath = Join-Path (Join-Path (Join-Path $GuestTestRoot 'candidate') $hostHash) 'chemsema-desktop.exe'
  $transaction = Invoke-Guest -ScriptBlock {
    param($CandidatePath, $TestRoot, $Request)

    function Send-ChannelRequest([string]$ChannelName, [string]$RequestSchema, [string]$ResponseSchema, [System.Collections.IDictionary]$EnvelopeFields, [int]$TimeoutMs) {
      $channelRoot = Join-Path $TestRoot $ChannelName
      if (-not (Test-Path -LiteralPath (Join-Path $channelRoot 'ready.json') -PathType Leaf)) { throw "$ChannelName is not ready." }
      $requestId = [Guid]::NewGuid().ToString('N')
      $inbox = Join-Path $channelRoot 'inbox'
      $outbox = Join-Path $channelRoot 'outbox'
      $requestPath = Join-Path $inbox "$requestId.json"
      $temporaryRequest = Join-Path $inbox "$requestId.tmp"
      $responsePath = Join-Path $outbox "$requestId.json"
      $envelope = [ordered]@{ schema=$RequestSchema; id=$requestId }
      foreach ($entry in $EnvelopeFields.GetEnumerator()) { $envelope[$entry.Key] = $entry.Value }
      [IO.File]::WriteAllText($temporaryRequest, ($envelope | ConvertTo-Json -Depth 10 -Compress), [Text.UTF8Encoding]::new($false))
      Move-Item -LiteralPath $temporaryRequest -Destination $requestPath
      $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
      while (-not (Test-Path -LiteralPath $responsePath -PathType Leaf) -and [DateTime]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 20 }
      if (-not (Test-Path -LiteralPath $responsePath -PathType Leaf)) { throw "$ChannelName returned no receipt within $TimeoutMs ms." }
      $response = Get-Content -Raw -Encoding UTF8 -LiteralPath $responsePath | ConvertFrom-Json
      if ($response.schema -ne $ResponseSchema -or $response.id -ne $requestId) { throw "$ChannelName response identity is invalid." }
      if ($response.status -ne 'passed') { throw "$ChannelName request failed: $($response.message)" }
      $response
    }

    $channelReceiptTimeoutMs = 15000

    function Observe-Cdp([System.Collections.IDictionary]$CdpRequest) {
      $json = $CdpRequest | ConvertTo-Json -Depth 10 -Compress
      $encoded = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($json))
      $response = Send-ChannelRequest 'cdp-channel' 'chemsema.gui.cdp-request.v1' 'chemsema.gui.cdp-response.v1' ([ordered]@{ requestBase64=$encoded }) $channelReceiptTimeoutMs
      $response.bridge.value
    }

    function Mark-Trace([string]$Phase) {
      [void](Observe-Cdp ([ordered]@{ mode='trace-mark'; name="chemsema-action:$([string]$Request.actionId):$Phase" }))
    }

    $process = Get-Process chemsema-desktop -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $CandidatePath -and $_.SessionId -ne 0 } | Select-Object -First 1
    if ($null -eq $process) { throw 'The authorized desktop candidate is not running.' }
    $kind = [string]$Request.input.kind
    if ($kind -notin @('click', 'drag', 'key')) { throw 'Unsupported action transaction input kind.' }
    $runRoot = Join-Path $TestRoot 'runs'
    $runDirectory = Join-Path $runRoot ("transaction-$kind-" + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $runDirectory -Force | Out-Null
    $guardPath = Join-Path $runDirectory 'guard.json'
    $guardJson = [ordered]@{
      expectedAgentSessionId = [int]$process.SessionId
      expectedProcessId = [int]$process.Id
      expectedExecutable = $CandidatePath
      allowedRunRoot = $runRoot
      runDirectory = $runDirectory
    } | ConvertTo-Json
    [IO.File]::WriteAllText($guardPath, $guardJson, [Text.UTF8Encoding]::new($false))
    $button = [string]$Request.input.button
    if ($kind -ne 'key' -and $button -notin @('left', 'middle', 'right')) { throw 'Unsupported action transaction mouse button.' }
    $modifiers = @($Request.input.modifiers | Where-Object { $null -ne $_ -and -not [string]::IsNullOrWhiteSpace([string]$_) })
    if ($kind -ne 'key' -and ($modifiers.Count -gt 3 -or @($modifiers | Where-Object { $_ -notin @('Shift', 'Control', 'Alt') }).Count -gt 0 -or @($modifiers | Select-Object -Unique).Count -ne $modifiers.Count)) {
      throw 'Action transaction pointer modifiers are not unique allowlisted values.'
    }
    $inputArguments = if ($kind -eq 'click') {
      @('click', '--guard', $guardPath, '--x', [string][int]$Request.input.x, '--y', [string][int]$Request.input.y, '--button', $button)
    } elseif ($kind -eq 'drag') {
      @('drag', '--guard', $guardPath, '--from-x', [string][int]$Request.input.from[0], '--from-y', [string][int]$Request.input.from[1], '--to-x', [string][int]$Request.input.to[0], '--to-y', [string][int]$Request.input.to[1], '--steps', [string][int]$Request.input.steps, '--button', $button)
    } else {
      @('key', '--guard', $guardPath, '--key', [string]$Request.input.key)
    }
    if ($kind -in @('click', 'drag') -and $modifiers.Count -gt 0) { $inputArguments += @('--modifiers', ($modifiers -join ',')) }

    $completionKind = [string]$Request.completion.kind
    if ($completionKind -notin @('actionable', 'quiescent', 'dom-count', 'dom-distinct-count', 'dom-text', 'entity-rect-deltas')) { throw 'Unsupported action transaction completion kind.' }
    if ($completionKind -eq 'dom-text' -and ([string]::IsNullOrWhiteSpace([string]$Request.completion.selector) -or ([string]$Request.completion.selector).Length -gt 2048 -or $null -eq $Request.completion.text -or ([string]$Request.completion.text).Length -gt 4096)) {
      throw 'DOM text completion requires a selector of 1 to 2048 characters and text of at most 4096 characters.'
    }
    $entityExpectations = @()
    $beforeEntityObservation = $null
    Mark-Trace 'start'
    if ($completionKind -eq 'entity-rect-deltas') {
      $entityExpectations = @($Request.completion.entities)
      $entityIds = @($entityExpectations | ForEach-Object { [string]$_.entityId })
      if ($entityExpectations.Count -lt 1 -or $entityExpectations.Count -gt 16 -or @($entityIds | Select-Object -Unique).Count -ne $entityIds.Count) {
        throw 'Entity rectangle completion requires 1 to 16 unique entities.'
      }
      foreach ($expectation in $entityExpectations) {
        if ([string]::IsNullOrWhiteSpace([string]$expectation.entityId) -or ([string]$expectation.entityId).Length -gt 128 -or [string]$expectation.operator -notin @('stationary', 'moved') -or [double]$expectation.toleranceWorld -lt 0 -or [double]$expectation.toleranceWorld -gt 1000) {
          throw 'Entity rectangle completion contains an invalid expectation.'
        }
      }
      $beforeEntityObservation = Observe-Cdp ([ordered]@{ mode='entity-rects-state'; entityIds=$entityIds })
      foreach ($entity in @($beforeEntityObservation.entities)) {
        if ([int]$entity.matchCount -ne 1 -or -not [bool]$entity.visible -or @($entity.rect).Count -ne 4 -or @($entity.worldRect).Count -ne 4) {
          throw "Entity rectangle precondition failed for '$($entity.entityId)'."
        }
      }
      $before = $beforeEntityObservation.state
    } else {
      $before = Observe-Cdp ([ordered]@{ mode='state' })
    }
    Mark-Trace 'input-before'
    $inputResponse = Send-ChannelRequest 'input-channel' 'chemsema.gui.guest-agent-request.v1' 'chemsema.gui.guest-agent-response.v1' ([ordered]@{ args=$inputArguments }) 8000
    Mark-Trace 'input-after'
    if ($completionKind -in @('dom-count', 'dom-distinct-count')) {
      $completionDeadline = [DateTime]::UtcNow.AddMilliseconds([int]$Request.completion.timeoutMs)
      $completionRequest = [ordered]@{
        mode = if ($completionKind -eq 'dom-distinct-count') { 'distinct-count-state' } else { 'count-state' }
        selector = [string]$Request.completion.selector
      }
      if ($completionKind -eq 'dom-distinct-count') { $completionRequest['attribute'] = [string]$Request.completion.attribute }
      do {
        $observed = Observe-Cdp $completionRequest
        $passed = if ($Request.completion.operator -eq 'eq') { [int]$observed.count -eq [int]$Request.completion.value } else { [int]$observed.count -ge [int]$Request.completion.value }
        if ($passed) { break }
        Start-Sleep -Milliseconds 20
      } while ([DateTime]::UtcNow -lt $completionDeadline)
      if (-not $passed) { throw "DOM count is $($observed.count); expected $($Request.completion.operator) $($Request.completion.value)." }
      $after = $observed.state
      $completion = [ordered]@{ observed=[int]$observed.count }
    } elseif ($completionKind -eq 'dom-text') {
      $completionDeadline = [DateTime]::UtcNow.AddMilliseconds([int]$Request.completion.timeoutMs)
      $completionRequest = [ordered]@{ mode='text-state'; selector=[string]$Request.completion.selector }
      $expectedText = [string]$Request.completion.text
      do {
        $observed = Observe-Cdp $completionRequest
        $passed = [int]$observed.count -eq 1 -and [string]$observed.text -ceq $expectedText
        if ($passed) { break }
        Start-Sleep -Milliseconds 20
      } while ([DateTime]::UtcNow -lt $completionDeadline)
      if (-not $passed) {
        if ([int]$observed.count -ne 1) { throw "DOM text selector matched $($observed.count) elements; expected exactly 1." }
        throw "DOM text did not exactly match the expected $($expectedText.Length)-character value."
      }
      $after = $observed.state
      $completion = [ordered]@{ observedText=[string]$observed.text }
    } elseif ($completionKind -eq 'entity-rect-deltas') {
      $completionDeadline = [DateTime]::UtcNow.AddMilliseconds([int]$Request.completion.timeoutMs)
      do {
        $afterEntityObservation = Observe-Cdp ([ordered]@{ mode='entity-rects-state'; entityIds=$entityIds })
        $observedEntities = @()
        $passed = $true
        foreach ($expectation in $entityExpectations) {
          $entityId = [string]$expectation.entityId
          $beforeEntity = @($beforeEntityObservation.entities | Where-Object { [string]$_.entityId -eq $entityId }) | Select-Object -First 1
          $afterEntity = @($afterEntityObservation.entities | Where-Object { [string]$_.entityId -eq $entityId }) | Select-Object -First 1
          if ($null -eq $beforeEntity -or $null -eq $afterEntity -or [int]$afterEntity.matchCount -ne 1 -or -not [bool]$afterEntity.visible -or @($afterEntity.rect).Count -ne 4 -or @($afterEntity.worldRect).Count -ne 4) {
            throw "Entity rectangle postcondition failed for '$entityId'."
          }
          $maximumDelta = 0.0
          for ($index = 0; $index -lt 4; $index++) {
            $maximumDelta = [Math]::Max($maximumDelta, [Math]::Abs([double]$afterEntity.worldRect[$index] - [double]$beforeEntity.worldRect[$index]))
          }
          $expectationPassed = if ([string]$expectation.operator -eq 'stationary') {
            $maximumDelta -le [double]$expectation.toleranceWorld
          } else {
            $maximumDelta -gt [double]$expectation.toleranceWorld
          }
          if (-not $expectationPassed) { $passed = $false }
          $observedEntities += [ordered]@{
            entityId = $entityId
            operator = [string]$expectation.operator
            toleranceWorld = [double]$expectation.toleranceWorld
            maximumDeltaWorld = $maximumDelta
            beforeWorldRect = @($beforeEntity.worldRect)
            afterWorldRect = @($afterEntity.worldRect)
            beforeRect = @($beforeEntity.rect)
            afterRect = @($afterEntity.rect)
            passed = $expectationPassed
          }
        }
        if ($passed) { break }
        Start-Sleep -Milliseconds 20
      } while ([DateTime]::UtcNow -lt $completionDeadline)
      if (-not $passed) {
        $failedSummary = @($observedEntities | Where-Object { -not $_.passed } | ForEach-Object { "$($_.entityId):$($_.operator):$($_.maximumDeltaWorld) world units" }) -join ', '
        throw "Entity rectangle completion failed: $failedSummary."
      }
      $after = $afterEntityObservation.state
      $completion = [ordered]@{ entities=$observedEntities }
    } else {
      $after = Observe-Cdp ([ordered]@{ mode='state' })
      $completion = if ($completionKind -eq 'actionable') { [ordered]@{ actionable=$true } } else { [ordered]@{ quiescent=$true } }
    }
    Mark-Trace 'complete'
    [ordered]@{
      schema = 'chemsema.gui.action-transaction-receipt.v1'
      input = $inputResponse.result
      before = $before
      after = $after
      completion = $completion
    }
  } -ArgumentList @($guestPath, $GuestTestRoot, $request)
  $cleanTransaction = [ordered]@{
    schema = [string]$transaction.schema
    input = $transaction.input
    before = $transaction.before
    after = $transaction.after
    completion = $transaction.completion
  }
  [ordered]@{
    schema = 'chemsema.gui.worker-attestation.v1'
    operation = 'action-transaction'
    vmId = (Get-WorkerVm).Id.ToString()
    vmName = (Get-WorkerVm).Name
    candidate = [ordered]@{ guestPath=$guestPath; sha256=$hostHash }
    transaction = $cleanTransaction
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
      clientRect = @($result.foreground.clientRect | ForEach-Object { [int]$_ })
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

function Configure-Autologon {
  $credential = Get-GuestCredential
  $plainPassword = $credential.GetNetworkCredential().Password
  try {
    $agentPath = Join-Path (Join-Path $GuestTestRoot 'agent') 'chemsema-gui-test-agent.exe'
    $result = Invoke-Command -VMId (Get-WorkerVm).Id -Credential $credential -ScriptBlock {
      param($ExpectedAccount, $AgentPath, $Password)
      $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
      if (-not $identity.Name.EndsWith("\$ExpectedAccount", [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Autologon configuration identity does not match the dedicated test account.'
      }
      $principal = [Security.Principal.WindowsPrincipal]::new($identity)
      if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
        throw 'The dedicated test account must be an administrator to store the LSA autologon secret.'
      }
      if (-not (Test-Path -LiteralPath $AgentPath -PathType Leaf)) {
        throw 'Guest agent is not installed.'
      }
      $startInfo = [Diagnostics.ProcessStartInfo]::new()
      $startInfo.FileName = $AgentPath
      $startInfo.Arguments = 'store-autologon-secret'
      $startInfo.UseShellExecute = $false
      $startInfo.CreateNoWindow = $true
      $startInfo.RedirectStandardInput = $true
      $startInfo.RedirectStandardOutput = $true
      $startInfo.RedirectStandardError = $true
      $process = [Diagnostics.Process]::new()
      $process.StartInfo = $startInfo
      if (-not $process.Start()) { throw 'Failed to start guest agent secret writer.' }
      $process.StandardInput.Write($Password)
      $process.StandardInput.Close()
      $standardOutput = $process.StandardOutput.ReadToEnd()
      $standardError = $process.StandardError.ReadToEnd()
      $process.WaitForExit()
      $Password = $null
      if ($process.ExitCode -ne 0) {
        throw "Guest agent secret writer failed: $standardError"
      }
      $secretReceipt = $standardOutput | ConvertFrom-Json
      if ($secretReceipt.status -ne 'stored') {
        throw 'Guest agent did not confirm LSA secret storage.'
      }

      $winlogon = 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion\Winlogon'
      Set-ItemProperty -LiteralPath $winlogon -Name AutoAdminLogon -Value '1' -Type String
      Set-ItemProperty -LiteralPath $winlogon -Name ForceAutoLogon -Value '1' -Type String
      Set-ItemProperty -LiteralPath $winlogon -Name DefaultUserName -Value $ExpectedAccount -Type String
      Set-ItemProperty -LiteralPath $winlogon -Name DefaultDomainName -Value $env:COMPUTERNAME -Type String
      Remove-ItemProperty -LiteralPath $winlogon -Name DefaultPassword -ErrorAction SilentlyContinue
      [ordered]@{
        identity = $identity.Name
        autoAdminLogon = (Get-ItemPropertyValue -LiteralPath $winlogon -Name AutoAdminLogon)
        forceAutoLogon = (Get-ItemPropertyValue -LiteralPath $winlogon -Name ForceAutoLogon)
        defaultUserName = (Get-ItemPropertyValue -LiteralPath $winlogon -Name DefaultUserName)
        defaultDomainName = (Get-ItemPropertyValue -LiteralPath $winlogon -Name DefaultDomainName)
        plainRegistryPasswordPresent = $null -ne (Get-ItemProperty -LiteralPath $winlogon -Name DefaultPassword -ErrorAction SilentlyContinue)
        lsaSecretStored = $true
      }
    } -ArgumentList @($GuestAccount, $agentPath, $plainPassword)
    [ordered]@{
      schema = 'chemsema.gui.worker-attestation.v1'
      operation = 'configure-autologon'
      vmId = (Get-WorkerVm).Id.ToString()
      vmName = (Get-WorkerVm).Name
      autologon = $result
    }
  }
  finally {
    $plainPassword = $null
  }
}

function Configure-DesktopBaseline {
  $result = Invoke-Guest -ScriptBlock {
    param($ExpectedAccount)
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    if (-not $identity.Name.EndsWith("\$ExpectedAccount", [StringComparison]::OrdinalIgnoreCase)) {
      throw 'Desktop baseline identity does not match the dedicated test account.'
    }
    $script:baselineChanged = $false
    function Open-BaselineKey([string]$Path) {
      $allowedPrefix = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\'
      if (-not $Path.StartsWith($allowedPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Desktop baseline registry path is outside the dedicated user allowlist.'
      }
      $relativePath = $Path.Substring(6)
      [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey(
        $relativePath,
        [Microsoft.Win32.RegistryKeyPermissionCheck]::ReadWriteSubTree
      )
    }
    function Set-BaselineDword([string]$Path, [string]$Name, [int]$Value) {
      $key = Open-BaselineKey $Path
      if ($null -eq $key) { throw "Desktop baseline could not open ${Path}." }
      try {
        $current = $key.GetValue($Name, $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        if ($null -eq $current -or [int]$current -ne $Value) {
          try {
            $key.SetValue($Name, $Value, [Microsoft.Win32.RegistryValueKind]::DWord)
          }
          catch {
            throw "Desktop baseline cannot set ${Path}::${Name}: $($_.Exception.Message)"
          }
          $script:baselineChanged = $true
        }
      }
      finally {
        $key.Close()
      }
    }
    function Get-BaselineDword([string]$Path, [string]$Name) {
      $key = Open-BaselineKey $Path
      if ($null -eq $key) { throw "Desktop baseline could not open ${Path}." }
      try {
        $value = $key.GetValue($Name, $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        if ($null -eq $value) { throw "Desktop baseline value $Name was not persisted." }
        [int]$value
      }
      finally {
        $key.Close()
      }
    }
    $engagement = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\UserProfileEngagement'
    Set-BaselineDword $engagement 'ScoobeSystemSettingEnabled' 0
    $delivery = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager'
    Set-BaselineDword $delivery 'ContentDeliveryAllowed' 0
    Set-BaselineDword $delivery 'OemPreInstalledAppsEnabled' 0
    Set-BaselineDword $delivery 'PreInstalledAppsEnabled' 0
    Set-BaselineDword $delivery 'PreInstalledAppsEverEnabled' 0
    Set-BaselineDword $delivery 'SilentInstalledAppsEnabled' 0
    Set-BaselineDword $delivery 'SystemPaneSuggestionsEnabled' 0
    Set-BaselineDword $delivery 'RotatingLockScreenEnabled' 0
    Set-BaselineDword $delivery 'RotatingLockScreenOverlayEnabled' 0
    Set-BaselineDword $delivery 'SoftLandingEnabled' 0
    Set-BaselineDword $delivery 'SubscribedContent-310093Enabled' 0
    Set-BaselineDword $delivery 'SubscribedContent-338389Enabled' 0
    [ordered]@{
      identity = $identity.Name
      scope = 'dedicated-test-user'
      changed = $script:baselineChanged
      settings = [ordered]@{
        scoobeSystemSettingEnabled = Get-BaselineDword $engagement 'ScoobeSystemSettingEnabled'
        contentDeliveryAllowed = Get-BaselineDword $delivery 'ContentDeliveryAllowed'
        oemPreInstalledAppsEnabled = Get-BaselineDword $delivery 'OemPreInstalledAppsEnabled'
        preInstalledAppsEnabled = Get-BaselineDword $delivery 'PreInstalledAppsEnabled'
        preInstalledAppsEverEnabled = Get-BaselineDword $delivery 'PreInstalledAppsEverEnabled'
        silentInstalledAppsEnabled = Get-BaselineDword $delivery 'SilentInstalledAppsEnabled'
        systemPaneSuggestionsEnabled = Get-BaselineDword $delivery 'SystemPaneSuggestionsEnabled'
        rotatingLockScreenEnabled = Get-BaselineDword $delivery 'RotatingLockScreenEnabled'
        rotatingLockScreenOverlayEnabled = Get-BaselineDword $delivery 'RotatingLockScreenOverlayEnabled'
        contentDeliverySoftLandingEnabled = Get-BaselineDword $delivery 'SoftLandingEnabled'
        subscribedContent310093Enabled = Get-BaselineDword $delivery 'SubscribedContent-310093Enabled'
        subscribedContent338389Enabled = Get-BaselineDword $delivery 'SubscribedContent-338389Enabled'
      }
    }
  } -ArgumentList @($GuestAccount)
  [ordered]@{
    schema = 'chemsema.gui.worker-attestation.v1'
    operation = 'configure-desktop-baseline'
    vmId = (Get-WorkerVm).Id.ToString()
    vmName = (Get-WorkerVm).Name
    baseline = $result
  }
}

function Get-InteractiveAgentAttestation {
  $agentPath = Join-Path (Join-Path $GuestTestRoot 'agent') 'chemsema-gui-test-agent.exe'
  $resultPath = Join-Path $GuestTestRoot 'interactive-attestation.json'
  $taskName = 'ChemSema GUI Test Agent Attestation'
  $result = Invoke-Guest -ScriptBlock {
    param($ExpectedAccount, $AgentPath, $ResultPath, $TaskName)
    if (-not (Test-Path -LiteralPath $AgentPath -PathType Leaf)) {
      throw 'Guest agent is not installed.'
    }
    Remove-Item -LiteralPath $ResultPath -Force -ErrorAction SilentlyContinue
    $arguments = "attest --output `"$ResultPath`""
    $action = New-ScheduledTaskAction -Execute $AgentPath -Argument $arguments
    $principal = New-ScheduledTaskPrincipal -UserId "$env:COMPUTERNAME\$ExpectedAccount" -LogonType Interactive -RunLevel Highest
    $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Minutes 2) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
    Register-ScheduledTask -TaskName $TaskName -Action $action -Principal $principal -Settings $settings -Force | Out-Null
    try {
      Start-ScheduledTask -TaskName $TaskName
      $deadline = [DateTime]::UtcNow.AddSeconds(45)
      do {
        if (Test-Path -LiteralPath $ResultPath -PathType Leaf) { break }
        Start-Sleep -Milliseconds 250
      } while ([DateTime]::UtcNow -lt $deadline)
      if (-not (Test-Path -LiteralPath $ResultPath -PathType Leaf)) {
        throw 'Interactive guest agent did not produce attestation within 45 seconds.'
      }
      Get-Content -Raw -Encoding UTF8 -LiteralPath $ResultPath | ConvertFrom-Json
    }
    finally {
      Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false -ErrorAction SilentlyContinue
    }
  } -ArgumentList @($GuestAccount, $agentPath, $resultPath, $taskName)
  [ordered]@{
    schema = 'chemsema.gui.worker-attestation.v1'
    operation = 'agent-attest-interactive'
    vmId = (Get-WorkerVm).Id.ToString()
    vmName = (Get-WorkerVm).Name
    agent = $result
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
  'reset' { Write-Result (Reset-Worker) }
  'start' { Write-Result (Start-Worker) }
  'guest-attest' { Write-Result (Get-GuestAttestation) }
  'prepare-guest' { Write-Result (Prepare-Guest) }
  'install-agent' { Write-Result (Install-Agent) }
  'configure-autologon' { Write-Result (Configure-Autologon) }
  'configure-desktop-baseline' { Write-Result (Configure-DesktopBaseline) }
  'install-candidate' { Write-Result (Install-Candidate) }
  'launch-candidate' { Write-Result (Start-Candidate) }
  'dismiss-known-blocker' { Write-Result (Dismiss-KnownBlocker) }
  'activate-candidate' { Write-Result (Activate-Candidate) }
  'start-input-agent' { Write-Result (Start-PersistentInputAgent) }
  'stop-input-agent' { Write-Result (Stop-PersistentInputAgent) }
  'uia-query' { Write-Result (Query-Uia) }
  'cdp-bridge' { Write-Result (Invoke-CdpBridge) }
  'fetch-artifacts' { Write-Result (Receive-GuestArtifacts) }
  'prepare-document-output' { Write-Result (Prepare-DocumentOutput) }
  'fetch-document-output' { Write-Result (Receive-GuestDocumentOutput) }
  'action-transaction' { Write-Result (Invoke-ActionTransaction) }
  'start-cdp-agent' { Write-Result (Start-PersistentCdpAgent) }
  'stop-cdp-agent' { Write-Result (Stop-PersistentCdpAgent) }
  'input-click' { Write-Result (Invoke-CandidateInput 'click') }
  'input-drag' { Write-Result (Invoke-CandidateInput 'drag') }
  'input-key' { Write-Result (Invoke-CandidateInput 'key') }
  'input-text' { Write-Result (Invoke-CandidateInput 'text') }
  'agent-attest-service' { Write-Result (Get-ServiceAgentAttestation) }
  'agent-attest-interactive' { Write-Result (Get-InteractiveAgentAttestation) }
  'stop' { Write-Result (Stop-Worker) }
}
