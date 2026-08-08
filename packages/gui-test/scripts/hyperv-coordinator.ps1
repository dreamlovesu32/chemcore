param(
  [Parameter(Mandatory = $true)]
  [ValidateSet('host-attest', 'reset', 'start', 'guest-attest', 'prepare-guest', 'install-agent', 'configure-autologon', 'configure-desktop-baseline', 'install-candidate', 'launch-candidate', 'dismiss-known-blocker', 'activate-candidate', 'start-input-agent', 'stop-input-agent', 'start-cdp-agent', 'stop-cdp-agent', 'uia-query', 'cdp-bridge', 'action-transaction', 'input-click', 'input-drag', 'input-key', 'agent-attest-service', 'agent-attest-interactive', 'stop')]
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
  [string]$ActionRequestBase64,
  [string]$AutomationName,
  [string]$AutomationScopeName,
  [int]$InputX,
  [int]$InputY,
  [int]$InputFromX,
  [int]$InputFromY,
  [int]$InputToX,
  [int]$InputToY,
  [int]$InputSteps = 8,
  [string]$InputKey,
  [ValidateSet('left', 'right', 'middle')]
  [string]$InputButton = 'left'
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

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
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--force-renderer-accessibility --remote-debugging-port=9223'
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
      $agentResult = Get-Content -Raw -LiteralPath $resultPath | ConvertFrom-Json
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
      $agentResult = Get-Content -Raw -LiteralPath $ResultPath | ConvertFrom-Json
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
  if ([string]::IsNullOrWhiteSpace($AutomationName)) { throw 'UI Automation query requires an exact accessible name.' }
  $hostHash = (Get-FileHash -LiteralPath $HostCandidatePath -Algorithm SHA256).Hash.ToLowerInvariant()
  $guestPath = Join-Path (Join-Path (Join-Path $GuestTestRoot 'candidate') $hostHash) 'chemsema-desktop.exe'
  $result = Invoke-Guest -ScriptBlock {
    param($ExpectedAccount, $CandidatePath, $Name, $ScopeName, $TestRoot)
    $process = Get-Process chemsema-desktop -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $CandidatePath -and $_.SessionId -ne 0 } | Select-Object -First 1
    if ($null -eq $process) { throw 'The authorized desktop candidate is not running.' }
    $runDirectory = Join-Path (Join-Path $TestRoot 'runs') ("uia-" + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $runDirectory -Force | Out-Null
    $scriptPath = Join-Path $runDirectory 'query.ps1'
    $resultPath = Join-Path $runDirectory 'result.json'
    $script = @'
param([int]$TargetProcessId, [string]$ExactName, [string]$ScopeName, [string]$OutputPath)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes
try {
$processCondition=[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::ProcessIdProperty,$TargetProcessId)
$roots=[Windows.Automation.AutomationElement]::RootElement.FindAll([Windows.Automation.TreeScope]::Children,$processCondition)
$root=@($roots | Where-Object { -not $_.Current.IsOffscreen -and $_.Current.BoundingRectangle.Width -gt 0 -and $_.Current.BoundingRectangle.Height -gt 0 } | Select-Object -First 1)[0]
if($null -eq $root){throw 'Candidate top-level UI Automation element is absent.'}
$searchRoot=$root
if(-not [string]::IsNullOrWhiteSpace($ScopeName)){
  $scopeCondition=[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::NameProperty,$ScopeName)
  $searchRoot=$root.FindFirst([Windows.Automation.TreeScope]::Descendants,$scopeCondition)
  if($null -eq $searchRoot){throw 'Requested UI Automation scope is absent.'}
}

$nameCondition=if($ExactName -eq '*'){[Windows.Automation.Condition]::TrueCondition}else{[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::NameProperty,$ExactName)}
$elements=$searchRoot.FindAll([Windows.Automation.TreeScope]::Descendants,$nameCondition)
$matches=@($elements | Select-Object -First 200 | ForEach-Object {
  $rect=$_.Current.BoundingRectangle
  [ordered]@{
    name=$_.Current.Name
    automationId=$_.Current.AutomationId
    controlType=$_.Current.ControlType.ProgrammaticName
    enabled=$_.Current.IsEnabled
    offscreen=$_.Current.IsOffscreen
    rect=@([int][Math]::Round($rect.Left),[int][Math]::Round($rect.Top),[int][Math]::Round($rect.Right),[int][Math]::Round($rect.Bottom))
  }
})
$json=[ordered]@{schema='chemsema.gui.uia-query.v1';processId=$TargetProcessId;name=$ExactName;matches=$matches}|ConvertTo-Json -Depth 6
[IO.File]::WriteAllText($OutputPath,$json,[Text.UTF8Encoding]::new($false))
} catch {
  $json=[ordered]@{schema='chemsema.gui.uia-query.v1';status='failed';message=$_.Exception.Message}|ConvertTo-Json
  [IO.File]::WriteAllText($OutputPath,$json,[Text.UTF8Encoding]::new($false))
  exit 1
}
'@
    [IO.File]::WriteAllText($scriptPath, $script, [Text.UTF8Encoding]::new($false))
    $taskName = "ChemSema GUI UIA Query $($process.Id)"
    $arguments = "-NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File `"$scriptPath`" -TargetProcessId $($process.Id) -ExactName `"$Name`" -ScopeName `"$ScopeName`" -OutputPath `"$resultPath`""
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
      $queryResult = Get-Content -Raw -LiteralPath $resultPath | ConvertFrom-Json
      if ($queryResult.status -eq 'failed') { throw "Interactive UI Automation query failed: $($queryResult.message)" }
      $queryResult
    }
    finally { Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue }
  } -ArgumentList @($GuestAccount, $guestPath, $AutomationName, $AutomationScopeName, $GuestTestRoot)
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
  $result = Invoke-Guest -ScriptBlock {
    param($TestRoot, $RequestBase64)
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
    $deadline = [DateTime]::UtcNow.AddSeconds(20)
    while (-not (Test-Path -LiteralPath $responsePath -PathType Leaf) -and [DateTime]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 20 }
    if (-not (Test-Path -LiteralPath $responsePath -PathType Leaf)) { throw 'Persistent CDP agent returned no receipt within 20 seconds.' }
    $response = Get-Content -Raw -LiteralPath $responsePath | ConvertFrom-Json
    if ($response.schema -ne 'chemsema.gui.cdp-response.v1' -or $response.id -ne $requestId) { throw 'Persistent CDP response identity is invalid.' }
    if ($response.status -ne 'passed') { throw "Persistent CDP request failed: $($response.message)" }
    $response.bridge
  } -ArgumentList @($GuestTestRoot, $CdpRequestBase64)
  [ordered]@{
    schema = 'chemsema.gui.worker-attestation.v1'
    operation = 'cdp-bridge'
    vmId = (Get-WorkerVm).Id.ToString()
    vmName = (Get-WorkerVm).Name
    bridge = $result
  }
}

function Start-PersistentCdpAgent {
  if ([string]::IsNullOrWhiteSpace($HostCdpScriptPath) -or -not (Test-Path -LiteralPath $HostCdpScriptPath -PathType Leaf)) {
    throw 'The guest CDP bridge script is unavailable.'
  }
  $source = Get-Content -Raw -LiteralPath $HostCdpScriptPath
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
    Get-Content -Raw -LiteralPath $readyPath | ConvertFrom-Json
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
    Get-Content -Raw -LiteralPath $readyPath | ConvertFrom-Json
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

function Invoke-CandidateInput([ValidateSet('click', 'drag', 'key')][string]$Kind) {
  $hostHash = (Get-FileHash -LiteralPath $HostCandidatePath -Algorithm SHA256).Hash.ToLowerInvariant()
  $guestPath = Join-Path (Join-Path (Join-Path $GuestTestRoot 'candidate') $hostHash) 'chemsema-desktop.exe'
  $result = Invoke-Guest -ScriptBlock {
    param($CandidatePath, $TestRoot, $Kind, $X, $Y, $FromX, $FromY, $ToX, $ToY, $Steps, $Button, $Key)
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
    } else {
      if ([string]::IsNullOrWhiteSpace($Key)) { throw 'Keyboard input requires a shortcut.' }
      @('key', '--guard', $guardPath, '--key', $Key)
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
    $response = Get-Content -Raw -LiteralPath $responsePath | ConvertFrom-Json
    if ($response.id -ne $requestId -or $response.schema -ne 'chemsema.gui.guest-agent-response.v1') { throw 'Persistent input response identity is invalid.' }
    if ($response.status -ne 'passed') { throw "Interactive input was rejected: $($response.message)" }
    $response.result
  } -ArgumentList @($guestPath, $GuestTestRoot, $Kind, $InputX, $InputY, $InputFromX, $InputFromY, $InputToX, $InputToY, $InputSteps, $InputButton, $InputKey)
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
  if ([int]$request.completion.timeoutMs + 4000 -gt [int]$request.budgetMs) { throw 'Action transaction completion timeout does not leave the required 4000 ms target-resolution and transport reserve.' }
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
      $response = Get-Content -Raw -LiteralPath $responsePath | ConvertFrom-Json
      if ($response.schema -ne $ResponseSchema -or $response.id -ne $requestId) { throw "$ChannelName response identity is invalid." }
      if ($response.status -ne 'passed') { throw "$ChannelName request failed: $($response.message)" }
      $response
    }

    function Observe-Cdp([System.Collections.IDictionary]$CdpRequest) {
      $json = $CdpRequest | ConvertTo-Json -Depth 10 -Compress
      $encoded = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes($json))
      $response = Send-ChannelRequest 'cdp-channel' 'chemsema.gui.cdp-request.v1' 'chemsema.gui.cdp-response.v1' ([ordered]@{ requestBase64=$encoded }) 8000
      $response.bridge.value
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
    $inputArguments = if ($kind -eq 'click') {
      @('click', '--guard', $guardPath, '--x', [string][int]$Request.input.x, '--y', [string][int]$Request.input.y, '--button', $button)
    } elseif ($kind -eq 'drag') {
      @('drag', '--guard', $guardPath, '--from-x', [string][int]$Request.input.from[0], '--from-y', [string][int]$Request.input.from[1], '--to-x', [string][int]$Request.input.to[0], '--to-y', [string][int]$Request.input.to[1], '--steps', [string][int]$Request.input.steps, '--button', $button)
    } else {
      @('key', '--guard', $guardPath, '--key', [string]$Request.input.key)
    }

    $before = Observe-Cdp ([ordered]@{ mode='state' })
    $inputResponse = Send-ChannelRequest 'input-channel' 'chemsema.gui.guest-agent-request.v1' 'chemsema.gui.guest-agent-response.v1' ([ordered]@{ args=$inputArguments }) 8000
    $completionKind = [string]$Request.completion.kind
    if ($completionKind -notin @('actionable', 'quiescent', 'dom-count')) { throw 'Unsupported action transaction completion kind.' }
    if ($completionKind -eq 'dom-count') {
      $completionDeadline = [DateTime]::UtcNow.AddMilliseconds([int]$Request.completion.timeoutMs)
      do {
        $observed = Observe-Cdp ([ordered]@{ mode='count-state'; selector=[string]$Request.completion.selector })
        $passed = if ($Request.completion.operator -eq 'eq') { [int]$observed.count -eq [int]$Request.completion.value } else { [int]$observed.count -ge [int]$Request.completion.value }
        if ($passed) { break }
        Start-Sleep -Milliseconds 20
      } while ([DateTime]::UtcNow -lt $completionDeadline)
      if (-not $passed) { throw "DOM count is $($observed.count); expected $($Request.completion.operator) $($Request.completion.value)." }
      $after = $observed.state
      $completion = [ordered]@{ observed=[int]$observed.count }
    } else {
      $after = Observe-Cdp ([ordered]@{ mode='state' })
      $completion = if ($completionKind -eq 'actionable') { [ordered]@{ actionable=$true } } else { [ordered]@{ quiescent=$true } }
    }
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
      Get-Content -Raw -LiteralPath $ResultPath | ConvertFrom-Json
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
  'action-transaction' { Write-Result (Invoke-ActionTransaction) }
  'start-cdp-agent' { Write-Result (Start-PersistentCdpAgent) }
  'stop-cdp-agent' { Write-Result (Stop-PersistentCdpAgent) }
  'input-click' { Write-Result (Invoke-CandidateInput 'click') }
  'input-drag' { Write-Result (Invoke-CandidateInput 'drag') }
  'input-key' { Write-Result (Invoke-CandidateInput 'key') }
  'agent-attest-service' { Write-Result (Get-ServiceAgentAttestation) }
  'agent-attest-interactive' { Write-Result (Get-InteractiveAgentAttestation) }
  'stop' { Write-Result (Stop-Worker) }
}
