param(
  [Parameter(Mandatory = $true)]
  [ValidateSet('host-attest', 'start', 'guest-attest', 'prepare-guest', 'install-agent', 'configure-autologon', 'install-candidate', 'launch-candidate', 'dismiss-known-blocker', 'activate-candidate', 'uia-query', 'input-click', 'input-drag', 'agent-attest-service', 'agent-attest-interactive', 'stop')]
  [string]$Operation,

  [Parameter(Mandatory = $true)]
  [string]$VmId,

  [string]$CredentialPath,
  [string]$GuestAccount,
  [string]$GuestTestRoot,
  [string]$HostAgentPath,
  [string]$HostCandidatePath,
  [string]$AutomationName,
  [string]$AutomationScopeName,
  [int]$InputX,
  [int]$InputY,
  [int]$InputFromX,
  [int]$InputFromY,
  [int]$InputToX,
  [int]$InputToY,
  [int]$InputSteps = 8,
  [ValidateSet('left', 'right', 'middle')]
  [string]$InputButton = 'left'
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
    Invoke-Command -Session $session -ScriptBlock {
      param($Directory)
      New-Item -ItemType Directory -Path $Directory -Force | Out-Null
    } -ArgumentList $guestDirectory
    Copy-Item -LiteralPath $HostCandidatePath -Destination $guestPath -ToSession $session -Force
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
    Get-Process chemsema-desktop -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $CandidatePath } | Stop-Process -ErrorAction Stop
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
      $deadline = [DateTime]::UtcNow.AddSeconds(45)
      do {
        if (Test-Path -LiteralPath $resultPath -PathType Leaf) { break }
        $task = Get-ScheduledTask -TaskName $taskName
        $info = Get-ScheduledTaskInfo -TaskName $taskName
        if ($task.State -eq 'Ready' -and $info.LastRunTime -gt [DateTime]::MinValue -and $info.LastTaskResult -ne 267009) { break }
        Start-Sleep -Milliseconds 250
      } while ([DateTime]::UtcNow -lt $deadline)
      if (-not (Test-Path -LiteralPath $resultPath -PathType Leaf)) {
        throw "Interactive activation agent failed with task result $($info.LastTaskResult)."
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

function Invoke-CandidateInput([ValidateSet('click', 'drag')][string]$Kind) {
  $hostHash = (Get-FileHash -LiteralPath $HostCandidatePath -Algorithm SHA256).Hash.ToLowerInvariant()
  $guestPath = Join-Path (Join-Path (Join-Path $GuestTestRoot 'candidate') $hostHash) 'chemsema-desktop.exe'
  $agentPath = Join-Path (Join-Path $GuestTestRoot 'agent') 'chemsema-gui-test-agent.exe'
  $result = Invoke-Guest -ScriptBlock {
    param($ExpectedAccount, $CandidatePath, $AgentPath, $TestRoot, $Kind, $X, $Y, $FromX, $FromY, $ToX, $ToY, $Steps, $Button)
    $process = Get-Process chemsema-desktop -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $CandidatePath -and $_.SessionId -ne 0 } | Select-Object -First 1
    if ($null -eq $process) { throw 'The authorized desktop candidate is not running.' }
    $runRoot = Join-Path $TestRoot 'runs'
    $runDirectory = Join-Path $runRoot ("input-$Kind-" + [Guid]::NewGuid().ToString('N'))
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
    $inputArguments = if ($Kind -eq 'click') {
      "click --guard `"$guardPath`" --x $X --y $Y --button $Button --output `"$resultPath`""
    } else {
      "drag --guard `"$guardPath`" --from-x $FromX --from-y $FromY --to-x $ToX --to-y $ToY --steps $Steps --button $Button --output `"$resultPath`""
    }
    $taskName = "ChemSema GUI Input $Kind $($process.Id)"
    $action = New-ScheduledTaskAction -Execute $AgentPath -Argument $inputArguments
    $principal = New-ScheduledTaskPrincipal -UserId "$env:COMPUTERNAME\$ExpectedAccount" -LogonType Interactive -RunLevel Highest
    Register-ScheduledTask -TaskName $taskName -Action $action -Principal $principal -Force | Out-Null
    try {
      Start-ScheduledTask -TaskName $taskName
      $deadline = [DateTime]::UtcNow.AddSeconds(30)
      do {
        if (Test-Path -LiteralPath $resultPath -PathType Leaf) { break }
        Start-Sleep -Milliseconds 100
      } while ([DateTime]::UtcNow -lt $deadline)
      if (-not (Test-Path -LiteralPath $resultPath -PathType Leaf)) { throw 'Interactive input agent returned no receipt.' }
      $agentResult = Get-Content -Raw -LiteralPath $resultPath | ConvertFrom-Json
      if ($agentResult.status -eq 'failed') { throw "Interactive input was rejected: $($agentResult.message)" }
      $agentResult
    }
    finally { Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction SilentlyContinue }
  } -ArgumentList @($GuestAccount, $guestPath, $agentPath, $GuestTestRoot, $Kind, $InputX, $InputY, $InputFromX, $InputFromY, $InputToX, $InputToY, $InputSteps, $InputButton)
  [ordered]@{
    schema = 'chemsema.gui.worker-attestation.v1'
    operation = "input-$Kind"
    vmId = (Get-WorkerVm).Id.ToString()
    vmName = (Get-WorkerVm).Name
    candidate = [ordered]@{ guestPath = $guestPath; sha256 = $hostHash }
    agent = $result
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
  'start' { Write-Result (Start-Worker) }
  'guest-attest' { Write-Result (Get-GuestAttestation) }
  'prepare-guest' { Write-Result (Prepare-Guest) }
  'install-agent' { Write-Result (Install-Agent) }
  'configure-autologon' { Write-Result (Configure-Autologon) }
  'install-candidate' { Write-Result (Install-Candidate) }
  'launch-candidate' { Write-Result (Start-Candidate) }
  'dismiss-known-blocker' { Write-Result (Dismiss-KnownBlocker) }
  'activate-candidate' { Write-Result (Activate-Candidate) }
  'uia-query' { Write-Result (Query-Uia) }
  'input-click' { Write-Result (Invoke-CandidateInput 'click') }
  'input-drag' { Write-Result (Invoke-CandidateInput 'drag') }
  'agent-attest-service' { Write-Result (Get-ServiceAgentAttestation) }
  'agent-attest-interactive' { Write-Result (Get-InteractiveAgentAttestation) }
  'stop' { Write-Result (Stop-Worker) }
}
