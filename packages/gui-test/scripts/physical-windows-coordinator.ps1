param(
  [Parameter(Mandatory = $true)]
  [ValidateSet('host-attest', 'reset', 'start', 'guest-attest', 'prepare-guest', 'install-agent', 'configure-desktop-baseline', 'install-candidate', 'launch-candidate', 'dismiss-known-blocker', 'activate-candidate', 'start-input-agent', 'stop-input-agent', 'start-cdp-agent', 'stop-cdp-agent', 'uia-query', 'cdp-bridge', 'fetch-artifacts', 'prepare-document-output', 'fetch-document-output', 'input-click', 'input-drag', 'input-key', 'input-text', 'agent-attest-interactive', 'stop')]
  [string]$Operation,
  [Parameter(Mandatory = $true)][string]$WorkerId,
  [Parameter(Mandatory = $true)][string]$ExpectedAccount,
  [Parameter(Mandatory = $true)][string]$TestRoot,
  [Parameter(Mandatory = $true)][string]$StateRoot,
  [Parameter(Mandatory = $true)][int]$CoordinatorPid,
  [string]$HostAgentPath,
  [string]$HostCandidatePath,
  [string]$HostCdpScriptPath,
  [string]$CdpRequestBase64,
  [string]$ArtifactManifestBase64,
  [string]$HostArtifactRoot,
  [string]$DocumentOutputId,
  [string]$DocumentOutputName,
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
  [ValidateSet('left', 'right', 'middle')][string]$InputButton = 'left'
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$script:Utf8 = [Text.UTF8Encoding]::new($false)
[Console]::OutputEncoding = $script:Utf8
$OutputEncoding = $script:Utf8

function Write-Result([object]$Value) { $Value | ConvertTo-Json -Depth 18 -Compress }
function Full([string]$Path) { [IO.Path]::GetFullPath($Path).TrimEnd('\') }
function Assert-Root([string]$Path) {
  $resolved = Full $Path
  if ($resolved -eq [IO.Path]::GetPathRoot($resolved)) { throw "Refusing broad physical worker root '$resolved'." }
  $resolved
}
function Assert-Child([string]$Root, [string]$Path) {
  $rootPath = (Assert-Root $Root) + '\'
  $childPath = [IO.Path]::GetFullPath($Path)
  if (-not $childPath.StartsWith($rootPath, [StringComparison]::OrdinalIgnoreCase)) { throw "Path '$childPath' escaped '$rootPath'." }
  $childPath
}
function State-Path([string]$Name) { Assert-Child $StateRoot (Join-Path $StateRoot $Name) }
function Test-Process([int]$Id) { $null -ne (Get-Process -Id $Id -ErrorAction SilentlyContinue) }
function Read-State([string]$Name) {
  $path = State-Path $Name
  if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { return $null }
  Get-Content -Raw -Encoding UTF8 -LiteralPath $path | ConvertFrom-Json
}
function Write-State([string]$Name, [object]$Value) {
  New-Item -ItemType Directory -Force -Path (Assert-Root $StateRoot) | Out-Null
  $path = State-Path $Name
  $temporary = "$path.tmp"
  [IO.File]::WriteAllText($temporary, ($Value | ConvertTo-Json -Depth 12 -Compress), $script:Utf8)
  Move-Item -LiteralPath $temporary -Destination $path -Force
}
function Remove-State([string]$Name) { Remove-Item -LiteralPath (State-Path $Name) -Force -ErrorAction SilentlyContinue }
function Current-Identity { [Security.Principal.WindowsIdentity]::GetCurrent().Name }
function Current-Session { [Diagnostics.Process]::GetCurrentProcess().SessionId }

function Assert-Identity {
  $identity = Current-Identity
  if (-not $identity.Equals($ExpectedAccount, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Physical worker account '$identity' does not match '$ExpectedAccount'."
  }
  if ((Current-Session) -eq 0) { throw 'Physical worker is not running in an interactive user session.' }
  $identity
}

function Invoke-Agent([string[]]$Arguments) {
  $agent = Read-State 'agent.json'
  if ($null -eq $agent -or -not (Test-Path -LiteralPath $agent.path -PathType Leaf)) { throw 'Physical input agent is not installed.' }
  $actual = (Get-FileHash -LiteralPath $agent.path -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actual -ne [string]$agent.sha256) { throw 'Installed physical input agent failed SHA-256 verification.' }
  $resultPath = State-Path ("agent-result-" + [Guid]::NewGuid().ToString('N') + '.json')
  try {
    $fullArguments = @($Arguments) + @('--output', $resultPath)
    $process = Start-Process -FilePath $agent.path -ArgumentList $fullArguments -WindowStyle Hidden -Wait -PassThru
    if ($process.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $resultPath -PathType Leaf)) {
      throw "Physical input agent failed with exit code $($process.ExitCode)."
    }
    $result = Get-Content -Raw -Encoding UTF8 -LiteralPath $resultPath | ConvertFrom-Json
    if ($result.status -eq 'failed') { throw "Physical input agent rejected the request: $($result.message)" }
    $result
  } finally {
    Remove-Item -LiteralPath $resultPath -Force -ErrorAction SilentlyContinue
  }
}

function Candidate-State {
  $candidate = Read-State 'candidate.json'
  if ($null -eq $candidate -or -not (Test-Path -LiteralPath $candidate.path -PathType Leaf)) { throw 'Physical desktop candidate is not installed.' }
  $actual = (Get-FileHash -LiteralPath $candidate.path -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actual -ne [string]$candidate.sha256) { throw 'Installed physical candidate failed SHA-256 verification.' }
  $candidate
}

function Candidate-Process {
  $candidate = Candidate-State
  $processState = Read-State 'candidate-process.json'
  if ($null -eq $processState -or -not (Test-Process ([int]$processState.processId))) { throw 'Physical desktop candidate is not running.' }
  $process = Get-Process -Id ([int]$processState.processId) -ErrorAction Stop
  if (-not $process.Path.Equals([string]$candidate.path, [StringComparison]::OrdinalIgnoreCase) -or $process.SessionId -eq 0) {
    throw 'Physical candidate process identity or interactive session is invalid.'
  }
  [ordered]@{ process=$process; candidate=$candidate }
}

function Stop-OwnedProcess([string]$StateName, [string[]]$AllowedExecutables) {
  $state = Read-State $StateName
  if ($null -eq $state -or -not (Test-Process ([int]$state.processId))) { Remove-State $StateName; return $false }
  $process = Get-Process -Id ([int]$state.processId) -ErrorAction Stop
  $allowed = $false
  foreach ($path in $AllowedExecutables) {
    if (-not [string]::IsNullOrWhiteSpace($path) -and $process.Path.Equals($path, [StringComparison]::OrdinalIgnoreCase)) { $allowed = $true }
  }
  if (-not $allowed) { throw "Refusing to stop PID $($process.Id): executable is not test-owned." }
  Stop-Process -Id $process.Id -Force
  $process.WaitForExit(10000) | Out-Null
  if (Test-Process $process.Id) { throw "Test-owned PID $($process.Id) did not stop." }
  Remove-State $StateName
  $true
}

function Get-HostAttestation {
  $identity = Assert-Identity
  $session = Current-Session
  $os = Get-CimInstance Win32_OperatingSystem
  $explorer = @(Get-Process explorer -ErrorAction SilentlyContinue | Where-Object { $_.SessionId -eq $session }).Count -gt 0
  $availableGiB = [Math]::Round([double]$os.FreePhysicalMemory / 1MB, 3)
  $commitPercent = if ([double]$os.TotalVirtualMemorySize -gt 0) {
    [Math]::Round((1 - ([double]$os.FreeVirtualMemory / [double]$os.TotalVirtualMemorySize)) * 100, 2)
  } else { 100 }
  [ordered]@{
    schema='chemsema.gui.worker-attestation.v1'; operation='host-attest'; workerId=$WorkerId
    host=[ordered]@{ platform='windows-physical'; account=$identity; sessionId=$session; interactiveSession=($session -ne 0); explorerInSession=$explorer; computerName=$env:COMPUTERNAME; osVersion=[Environment]::OSVersion.Version.ToString() }
    resources=[ordered]@{ logicalProcessors=[Environment]::ProcessorCount; totalMemoryGiB=[Math]::Round([double]$os.TotalVisibleMemorySize / 1MB, 3); availableMemoryGiB=$availableGiB; commitPercent=$commitPercent }
  }
}

function Reset-Worker {
  Assert-Identity | Out-Null
  New-Item -ItemType Directory -Force -Path (Assert-Root $TestRoot),(Assert-Root $StateRoot) | Out-Null
  $candidate = Read-State 'candidate.json'
  $agent = Read-State 'agent.json'
  $stopped = @()
  if (Stop-OwnedProcess 'input-process.json' @([string]$agent.path)) { $stopped += 'input-agent' }
  if (Stop-OwnedProcess 'cdp-process.json' @((Get-Command powershell.exe).Source)) { $stopped += 'cdp-agent' }
  if (Stop-OwnedProcess 'candidate-process.json' @([string]$candidate.path)) { $stopped += 'candidate' }
  if (Stop-OwnedProcess 'keep-awake.json' @((Get-Command powershell.exe).Source)) { $stopped += 'keep-awake' }
  foreach ($name in @('input-channel','cdp-channel','runs')) {
    $path = Assert-Child $TestRoot (Join-Path $TestRoot $name)
    if (Test-Path -LiteralPath $path -PathType Container) { Remove-Item -LiteralPath $path -Recurse -Force }
  }
  [ordered]@{ schema='chemsema.gui.worker-attestation.v1'; operation='reset'; workerId=$WorkerId; state='ready'; scope='test-owned-processes-only'; stopped=$stopped }
}

function Start-KeepAwake {
  $existing = Read-State 'keep-awake.json'
  if ($null -ne $existing -and (Test-Process ([int]$existing.processId))) { return $existing }
  $scriptPath = State-Path 'keep-awake.ps1'
  $heartbeat = State-Path 'keep-awake-heartbeat.json'
  $source = @'
param([string]$Heartbeat)
$ErrorActionPreference='Stop'
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class ChemSemaExecutionState {
  [DllImport("kernel32.dll")] public static extern uint SetThreadExecutionState(uint esFlags);
}
"@
while ($true) {
  [void][ChemSemaExecutionState]::SetThreadExecutionState([Convert]::ToUInt32('80000003', 16))
  [IO.File]::WriteAllText($Heartbeat, ('{"schema":"chemsema.gui.keep-awake.v1","pid":' + $PID + ',"at":"' + [DateTime]::UtcNow.ToString('o') + '"}'), [Text.UTF8Encoding]::new($false))
  Start-Sleep -Seconds 15
}
'@
  [IO.File]::WriteAllText($scriptPath, $source, $script:Utf8)
  $process = Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoLogo','-NoProfile','-NonInteractive','-WindowStyle','Hidden','-ExecutionPolicy','Bypass','-File',$scriptPath,'-Heartbeat',$heartbeat) -WindowStyle Hidden -PassThru
  $deadline = [DateTime]::UtcNow.AddSeconds(10)
  while (-not (Test-Path -LiteralPath $heartbeat -PathType Leaf) -and [DateTime]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 100 }
  if (-not (Test-Path -LiteralPath $heartbeat -PathType Leaf)) { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue; throw 'Keep-awake helper did not become ready.' }
  $state = [ordered]@{ processId=[int]$process.Id; executable=$process.Path; heartbeat=$heartbeat }
  Write-State 'keep-awake.json' $state
  $state
}

function Start-Worker {
  Assert-Identity | Out-Null
  New-Item -ItemType Directory -Force -Path (Assert-Root $TestRoot),(Assert-Root $StateRoot) | Out-Null
  $lease = Read-State 'lease.json'
  if ($null -ne $lease -and [int]$lease.coordinatorPid -ne $CoordinatorPid -and (Test-Process ([int]$lease.coordinatorPid))) {
    throw "Physical worker lease is owned by live coordinator PID $($lease.coordinatorPid)."
  }
  $lease = [ordered]@{ schema='chemsema.gui.physical-lease.v1'; workerId=$WorkerId; coordinatorPid=$CoordinatorPid; account=(Current-Identity); acquiredAt=[DateTime]::UtcNow.ToString('o'); owned=$true }
  Write-State 'lease.json' $lease
  $keepAwake = Start-KeepAwake
  [ordered]@{ schema='chemsema.gui.worker-attestation.v1'; operation='start'; workerId=$WorkerId; lease=$lease; keepAwake=[ordered]@{ running=(Test-Process ([int]$keepAwake.processId)); processId=[int]$keepAwake.processId; heartbeat=$keepAwake.heartbeat } }
}

function Get-GuestAttestation {
  $identity = Assert-Identity
  $agent = $null
  try { $agent = Invoke-Agent @('attest') } catch {}
  [ordered]@{ schema='chemsema.gui.worker-attestation.v1'; operation='guest-attest'; workerId=$WorkerId; guest=[ordered]@{ identity=$identity; interactiveAccountMatches=($null -ne $agent -and $agent.interactiveReady -and $agent.inputDesktop -eq 'Default'); sessionId=(Current-Session); inputDesktop=if($null -ne $agent){$agent.inputDesktop}else{$null} } }
}

function Prepare-Guest {
  Assert-Identity | Out-Null
  foreach ($path in @($TestRoot,$StateRoot,(Join-Path $TestRoot 'agent'),(Join-Path $TestRoot 'candidate'),(Join-Path $TestRoot 'documents'),(Join-Path $TestRoot 'artifacts'),(Join-Path $TestRoot 'runs'))) {
    $resolvedPath = if ($path -eq $TestRoot -or $path -eq $StateRoot) { Assert-Root $path } else { Assert-Child $TestRoot $path }
    New-Item -ItemType Directory -Force -Path $resolvedPath | Out-Null
  }
  [ordered]@{ schema='chemsema.gui.worker-attestation.v1'; operation='prepare-guest'; workerId=$WorkerId; testRoot=(Full $TestRoot); stateRoot=(Full $StateRoot) }
}

function Install-ContentAddressed([string]$Source, [string]$Kind, [string]$Name, [string]$StateName) {
  if ([string]::IsNullOrWhiteSpace($Source) -or -not (Test-Path -LiteralPath $Source -PathType Leaf)) { throw "$Kind source is unavailable." }
  $sha = (Get-FileHash -LiteralPath $Source -Algorithm SHA256).Hash.ToLowerInvariant()
  $directory = Assert-Child $TestRoot (Join-Path (Join-Path $TestRoot $Kind) $sha)
  New-Item -ItemType Directory -Force -Path $directory | Out-Null
  $destination = Assert-Child $TestRoot (Join-Path $directory $Name)
  $reused = Test-Path -LiteralPath $destination -PathType Leaf
  if (-not $reused) { Copy-Item -LiteralPath $Source -Destination $destination }
  $actual = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actual -ne $sha) { throw "$Kind installation failed SHA-256 verification." }
  $state = [ordered]@{ path=$destination; sha256=$sha; bytes=[int64](Get-Item -LiteralPath $destination).Length }
  Write-State $StateName $state
  [ordered]@{ path=$destination; sha256=$sha; bytes=$state.bytes; reused=$reused }
}

function Install-Agent { Prepare-Guest | Out-Null; $value=Install-ContentAddressed $HostAgentPath 'agent' 'chemsema-gui-test-agent.exe' 'agent.json'; [ordered]@{schema='chemsema.gui.worker-attestation.v1';operation='install-agent';workerId=$WorkerId;agent=$value} }
function Install-Candidate { Prepare-Guest | Out-Null; $value=Install-ContentAddressed $HostCandidatePath 'candidate' 'chemsema-desktop.exe' 'candidate.json'; [ordered]@{schema='chemsema.gui.worker-attestation.v1';operation='install-candidate';workerId=$WorkerId;candidate=[ordered]@{guestPath=$value.path;sha256=$value.sha256;bytes=$value.bytes;reused=$value.reused}} }

function Configure-DesktopBaseline {
  $keepAwake = Start-KeepAwake
  [ordered]@{ schema='chemsema.gui.worker-attestation.v1'; operation='configure-desktop-baseline'; workerId=$WorkerId; baseline=[ordered]@{ scope='current-physical-account'; changed=$false; keepAwakeRunning=(Test-Process ([int]$keepAwake.processId)); account=(Assert-Identity) } }
}

function Start-Candidate {
  Assert-Identity | Out-Null
  $owned = Candidate-State
  Stop-OwnedProcess 'candidate-process.json' @([string]$owned.path) | Out-Null
  $logPath = Assert-Child $TestRoot (Join-Path (Split-Path -Parent $owned.path) 'webview.log')
  Remove-Item -LiteralPath $logPath -Force -ErrorAction SilentlyContinue
  $previous = $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS
  try {
    $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--force-renderer-accessibility --remote-debugging-port=9223 --enable-logging --log-file=$logPath --v=1"
    $process = Start-Process -FilePath $owned.path -PassThru
  } finally { $env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = $previous }
  $deadline = [DateTime]::UtcNow.AddSeconds(60)
  do {
    Start-Sleep -Milliseconds 100
    $process.Refresh()
    if (-not $process.HasExited -and $process.MainWindowHandle -ne 0) { break }
  } while ([DateTime]::UtcNow -lt $deadline)
  if ($process.HasExited -or $process.SessionId -eq 0 -or $process.MainWindowHandle -eq 0) { throw 'Physical candidate did not open an interactive window within 60 seconds.' }
  Write-State 'candidate-process.json' ([ordered]@{ processId=[int]$process.Id; executable=$owned.path; sessionId=[int]$process.SessionId; startedAt=[DateTime]::UtcNow.ToString('o') })
  [ordered]@{ schema='chemsema.gui.worker-attestation.v1'; operation='launch-candidate'; workerId=$WorkerId; candidate=[ordered]@{ guestPath=$owned.path; sha256=$owned.sha256; processId=[int]$process.Id; sessionId=[int]$process.SessionId } }
}

function New-Guard {
  $owned = Candidate-Process
  $runRoot = Assert-Child $TestRoot (Join-Path $TestRoot 'runs')
  New-Item -ItemType Directory -Force -Path $runRoot | Out-Null
  $runDirectory = Assert-Child $TestRoot (Join-Path $runRoot ([Guid]::NewGuid().ToString('N')))
  New-Item -ItemType Directory -Force -Path $runDirectory | Out-Null
  $guardPath = Join-Path $runDirectory 'guard.json'
  $guard = [ordered]@{ expectedAccount=$ExpectedAccount; expectedAgentSessionId=[int]$owned.process.SessionId; expectedProcessId=[int]$owned.process.Id; expectedExecutable=$owned.candidate.path; allowedRunRoot=$runRoot; runDirectory=$runDirectory }
  [IO.File]::WriteAllText($guardPath, ($guard | ConvertTo-Json -Compress), $script:Utf8)
  [ordered]@{ path=$guardPath; owned=$owned; runDirectory=$runDirectory }
}

function Activate-Candidate {
  $guard = New-Guard
  $agent = Invoke-Agent @('activate','--guard',$guard.path)
  [ordered]@{ schema='chemsema.gui.worker-attestation.v1'; operation='activate-candidate'; workerId=$WorkerId; candidate=[ordered]@{guestPath=$guard.owned.candidate.path;sha256=$guard.owned.candidate.sha256}; agent=$agent }
}

function Dismiss-KnownBlocker {
  $agent = Invoke-Agent @('dismiss-known-blocker')
  [ordered]@{ schema='chemsema.gui.worker-attestation.v1'; operation='dismiss-known-blocker'; workerId=$WorkerId; agent=$agent }
}

function Start-InputAgent {
  $agent = Read-State 'agent.json'
  if ($null -eq $agent) { throw 'Physical input agent is not installed.' }
  Stop-OwnedProcess 'input-process.json' @([string]$agent.path) | Out-Null
  $channel = Assert-Child $TestRoot (Join-Path $TestRoot 'input-channel')
  if (Test-Path -LiteralPath $channel) { Remove-Item -LiteralPath $channel -Recurse -Force }
  New-Item -ItemType Directory -Force -Path $channel | Out-Null
  $process = Start-Process -FilePath $agent.path -ArgumentList @('serve','--allowed-root',$TestRoot,'--channel-root',$channel) -WindowStyle Hidden -PassThru
  $ready = Join-Path $channel 'ready.json'
  $deadline = [DateTime]::UtcNow.AddSeconds(20)
  while (-not (Test-Path -LiteralPath $ready -PathType Leaf) -and [DateTime]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 50 }
  if (-not (Test-Path -LiteralPath $ready -PathType Leaf)) { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue; throw 'Physical input agent did not become ready.' }
  Write-State 'input-process.json' ([ordered]@{processId=[int]$process.Id;executable=$agent.path;channel=$channel})
  $receipt = Get-Content -Raw -Encoding UTF8 -LiteralPath $ready | ConvertFrom-Json
  [ordered]@{ schema='chemsema.gui.worker-attestation.v1'; operation='start-input-agent'; workerId=$WorkerId; agent=$receipt }
}

function Stop-InputAgent {
  $state=Read-State 'input-process.json'; if($null -ne $state){New-Item -ItemType File -Force -Path (Join-Path $state.channel 'shutdown') | Out-Null}
  $agent=Read-State 'agent.json'; $stopped=Stop-OwnedProcess 'input-process.json' @([string]$agent.path)
  [ordered]@{schema='chemsema.gui.worker-attestation.v1';operation='stop-input-agent';workerId=$WorkerId;agent=[ordered]@{status='stopped';stopped=$stopped}}
}

function Send-ChannelRequest([string]$Channel, [object]$Envelope, [int]$TimeoutSeconds) {
  $id=[string]$Envelope.id; $inbox=Join-Path $Channel 'inbox'; $outbox=Join-Path $Channel 'outbox'
  $temporary=Join-Path $inbox "$id.tmp"; $request=Join-Path $inbox "$id.json"; $response=Join-Path $outbox "$id.json"
  [IO.File]::WriteAllText($temporary, ($Envelope | ConvertTo-Json -Depth 12 -Compress), $script:Utf8); Move-Item -LiteralPath $temporary -Destination $request
  $deadline=[DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
  while(-not(Test-Path -LiteralPath $response -PathType Leaf)-and [DateTime]::UtcNow-lt$deadline){Start-Sleep -Milliseconds 20}
  if(-not(Test-Path -LiteralPath $response -PathType Leaf)){throw "Channel returned no receipt within $TimeoutSeconds seconds."}
  Get-Content -Raw -Encoding UTF8 -LiteralPath $response | ConvertFrom-Json
}

function Invoke-CandidateInput([string]$Kind) {
  $guard=New-Guard; $state=Read-State 'input-process.json'; if($null-eq$state-or-not(Test-Process ([int]$state.processId))){throw 'Persistent physical input agent is not ready.'}
  $args=@($Kind,'--guard',$guard.path)
  if($Kind-eq'click'){$args+=@('--x',[string]$InputX,'--y',[string]$InputY,'--button',$InputButton)}
  elseif($Kind-eq'drag'){$args+=@('--from-x',[string]$InputFromX,'--from-y',[string]$InputFromY,'--to-x',[string]$InputToX,'--to-y',[string]$InputToY,'--steps',[string]$InputSteps,'--button',$InputButton)}
  elseif($Kind-eq'key'){$args+=@('--key',$InputKey)}
  elseif($Kind-eq'text'){$args+=@('--text-base64',$InputTextBase64)}
  if(-not[string]::IsNullOrWhiteSpace($InputModifiers)){$args+=@('--modifiers',$InputModifiers)}
  $id=[Guid]::NewGuid().ToString('N'); $receipt=Send-ChannelRequest $state.channel ([ordered]@{schema='chemsema.gui.guest-agent-request.v1';id=$id;args=$args}) 20
  if($receipt.schema-ne'chemsema.gui.guest-agent-response.v1'-or$receipt.id-ne$id-or$receipt.status-ne'passed'){throw "Physical input request failed: $($receipt.message)"}
  [ordered]@{schema='chemsema.gui.worker-attestation.v1';operation="input-$Kind";workerId=$WorkerId;candidate=[ordered]@{guestPath=$guard.owned.candidate.path;sha256=$guard.owned.candidate.sha256};agent=$receipt.result}
}

function Start-CdpAgent {
  if(-not(Test-Path -LiteralPath $HostCdpScriptPath -PathType Leaf)){throw 'CDP bridge script is unavailable.'}
  Stop-OwnedProcess 'cdp-process.json' @((Get-Command powershell.exe).Source) | Out-Null
  $channel=Assert-Child $TestRoot (Join-Path $TestRoot 'cdp-channel'); if(Test-Path -LiteralPath $channel){Remove-Item -LiteralPath $channel -Recurse -Force}; New-Item -ItemType Directory -Force -Path $channel|Out-Null
  $process=Start-Process -FilePath 'powershell.exe' -ArgumentList @('-NoLogo','-NoProfile','-NonInteractive','-WindowStyle','Hidden','-ExecutionPolicy','Bypass','-File',$HostCdpScriptPath,'-AllowedRoot',$TestRoot,'-ChannelRoot',$channel) -WindowStyle Hidden -PassThru
  $ready=Join-Path $channel 'ready.json';$deadline=[DateTime]::UtcNow.AddSeconds(20);while(-not(Test-Path -LiteralPath $ready -PathType Leaf)-and[DateTime]::UtcNow-lt$deadline){Start-Sleep -Milliseconds 50}
  if(-not(Test-Path -LiteralPath $ready -PathType Leaf)){Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue;throw 'Physical CDP agent did not become ready.'}
  Write-State 'cdp-process.json' ([ordered]@{processId=[int]$process.Id;executable=$process.Path;channel=$channel})
  $receipt=Get-Content -Raw -Encoding UTF8 -LiteralPath $ready|ConvertFrom-Json
  [ordered]@{schema='chemsema.gui.worker-attestation.v1';operation='start-cdp-agent';workerId=$WorkerId;agent=$receipt}
}

function Stop-CdpAgent {$state=Read-State 'cdp-process.json';if($null-ne$state){New-Item -ItemType File -Force -Path(Join-Path $state.channel 'shutdown')|Out-Null};$stopped=Stop-OwnedProcess 'cdp-process.json' @((Get-Command powershell.exe).Source);[ordered]@{schema='chemsema.gui.worker-attestation.v1';operation='stop-cdp-agent';workerId=$WorkerId;agent=[ordered]@{status='stopped';stopped=$stopped}}}
function Invoke-CdpBridge {$state=Read-State 'cdp-process.json';if($null-eq$state-or-not(Test-Process ([int]$state.processId))){throw 'Persistent physical CDP agent is not ready.'};$decoded=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($CdpRequestBase64))|ConvertFrom-Json;$timeout=if($decoded.mode-eq'artifact-export'){90}else{20};$id=[Guid]::NewGuid().ToString('N');$receipt=Send-ChannelRequest $state.channel ([ordered]@{schema='chemsema.gui.cdp-request.v1';id=$id;requestBase64=$CdpRequestBase64}) $timeout;if($receipt.schema-ne'chemsema.gui.cdp-response.v1'-or$receipt.id-ne$id-or$receipt.status-ne'passed'){throw "Physical CDP request failed: $($receipt.message)"};[ordered]@{schema='chemsema.gui.worker-attestation.v1';operation='cdp-bridge';workerId=$WorkerId;bridge=$receipt.bridge}}

function Receive-Artifacts {
  $manifest=[Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($ArtifactManifestBase64))|ConvertFrom-Json;if($manifest.schema-ne'chemsema.gui.guest-artifact-export.v1'-or[string]$manifest.artifactId-notmatch'^[a-f0-9]{32}$'){throw 'Artifact manifest is invalid.'}
  $hostRoot=Assert-Root $HostArtifactRoot;if(-not(Test-Path -LiteralPath $hostRoot -PathType Container)){throw 'Host artifact staging root is absent.'};$exportRoot=(Assert-Child $TestRoot (Join-Path (Join-Path $TestRoot 'artifacts') ([string]$manifest.artifactId)))+'\';$seen=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal);$received=@()
  foreach($artifact in @($manifest.artifacts)){$name=[string]$artifact.name;$source=[IO.Path]::GetFullPath([string]$artifact.guestPath);if($name-notmatch'^[a-z0-9][a-z0-9._-]{0,127}$'-or-not$seen.Add($name)-or-not$source.StartsWith($exportRoot,[StringComparison]::OrdinalIgnoreCase)){throw 'Artifact escaped its authorized root or has an unsafe name.'};$size=[int64](Get-Item -LiteralPath $source).Length;$sha=(Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant();if($size-ne[int64]$artifact.size-or$sha-ne[string]$artifact.sha256-or$size-gt(64*1024*1024)){throw "Artifact $name changed before transfer."};$destination=Join-Path $hostRoot $name;Copy-Item -LiteralPath $source -Destination $destination;$copySha=(Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant();if($copySha-ne$sha){throw "Artifact $name failed copied SHA-256 verification."};$received+=[ordered]@{name=$name;mediaType=[string]$artifact.mediaType;hostPath=$destination;size=$size;sha256=$sha}}
  [ordered]@{schema='chemsema.gui.worker-attestation.v1';operation='fetch-artifacts';workerId=$WorkerId;transfer=[ordered]@{schema='chemsema.gui.host-artifact-transfer.v1';artifactId=[string]$manifest.artifactId;artifacts=$received}}
}

function Document-Identity {if($DocumentOutputId-notmatch'^[a-f0-9]{32}$'-or$DocumentOutputName-notmatch'^[a-z0-9][a-z0-9._-]{0,95}\.ccjs$'-or[IO.Path]::GetFileName($DocumentOutputName)-ne$DocumentOutputName){throw 'Document output identity is invalid.'};$directory=Assert-Child $TestRoot (Join-Path (Join-Path $TestRoot 'documents') $DocumentOutputId);$path=Assert-Child $TestRoot (Join-Path $directory $DocumentOutputName);[ordered]@{directory=$directory;path=$path}}
function Prepare-DocumentOutput {$identity=Document-Identity;if(Test-Path -LiteralPath $identity.directory){Remove-Item -LiteralPath $identity.directory -Recurse -Force};New-Item -ItemType Directory -Force -Path $identity.directory|Out-Null;[ordered]@{schema='chemsema.gui.worker-attestation.v1';operation='prepare-document-output';workerId=$WorkerId;output=[ordered]@{id=$DocumentOutputId;name=$DocumentOutputName;guestPath=$identity.path;exists=$false}}}
function Fetch-DocumentOutput {$identity=Document-Identity;$deadline=[DateTime]::UtcNow.AddSeconds(30);while((-not(Test-Path -LiteralPath $identity.path -PathType Leaf)-or(Get-Item -LiteralPath $identity.path).Length-eq0)-and[DateTime]::UtcNow-lt$deadline){Start-Sleep -Milliseconds 50};if(-not(Test-Path -LiteralPath $identity.path -PathType Leaf)){throw 'Document output was not created.'};$item=Get-Item -LiteralPath $identity.path;if($item.Length-le0-or$item.Length-gt(64*1024*1024)){throw 'Document output size is invalid.'};$sha=(Get-FileHash -LiteralPath $identity.path -Algorithm SHA256).Hash.ToLowerInvariant();$hostRoot=Assert-Root $HostArtifactRoot;$destination=Join-Path $hostRoot $DocumentOutputName;Copy-Item -LiteralPath $identity.path -Destination $destination;if((Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant()-ne$sha){throw 'Document output failed copied SHA-256 verification.'};[ordered]@{schema='chemsema.gui.worker-attestation.v1';operation='fetch-document-output';workerId=$WorkerId;output=[ordered]@{id=$DocumentOutputId;name=$DocumentOutputName;guestPath=$identity.path;hostPath=$destination;size=[int64]$item.Length;sha256=$sha}}}

function Query-Uia {
  $owned=Candidate-Process;if([string]::IsNullOrWhiteSpace($AutomationName)-and[string]::IsNullOrWhiteSpace($AutomationId)){throw 'UIA query needs an exact name or id.'};Add-Type -AssemblyName UIAutomationClient;Add-Type -AssemblyName UIAutomationTypes
  $condition=[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::ProcessIdProperty,[int]$owned.process.Id);$roots=@([Windows.Automation.AutomationElement]::RootElement.FindAll([Windows.Automation.TreeScope]::Children,$condition)|Where-Object{-not$_.Current.IsOffscreen-and$_.Current.BoundingRectangle.Width-gt0-and$_.Current.BoundingRectangle.Height-gt0});$topLevels=@($roots|ForEach-Object{$r=$_.Current.BoundingRectangle;[ordered]@{name=$_.Current.Name;automationId=$_.Current.AutomationId;className=$_.Current.ClassName;offscreen=$_.Current.IsOffscreen;rect=@([int]$r.Left,[int]$r.Top,[int]$r.Right,[int]$r.Bottom)}});$conditions=@();if(-not[string]::IsNullOrWhiteSpace($AutomationName)-and$AutomationName-ne'*'){$conditions+=[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::NameProperty,$AutomationName)};if(-not[string]::IsNullOrWhiteSpace($AutomationId)){$conditions+=[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::AutomationIdProperty,$AutomationId)};$query=if($conditions.Count-eq0){[Windows.Automation.Condition]::TrueCondition}elseif($conditions.Count-eq1){$conditions[0]}else{[Windows.Automation.AndCondition]::new([Windows.Automation.Condition[]]$conditions)};$matches=@()
  foreach($root in $roots){$search=@($root);if(-not[string]::IsNullOrWhiteSpace($AutomationScopeName)){$scope=[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::NameProperty,$AutomationScopeName);$search=@($root.FindAll([Windows.Automation.TreeScope]::Descendants,$scope))};foreach($base in $search){foreach($element in @($base.FindAll([Windows.Automation.TreeScope]::Descendants,$query))){if($matches.Count-ge200){break};$r=$element.Current.BoundingRectangle;if($r.Width-le0-or$r.Height-le0-or(-not[string]::IsNullOrWhiteSpace($AutomationControlType)-and$element.Current.ControlType.ProgrammaticName-ne$AutomationControlType)){continue};$matches+=[ordered]@{name=$element.Current.Name;automationId=$element.Current.AutomationId;className=$element.Current.ClassName;controlType=$element.Current.ControlType.ProgrammaticName;enabled=$element.Current.IsEnabled;offscreen=$element.Current.IsOffscreen;hasKeyboardFocus=$element.Current.HasKeyboardFocus;topLevelName=$root.Current.Name;topLevelClassName=$root.Current.ClassName;rect=@([int]$r.Left,[int]$r.Top,[int]$r.Right,[int]$r.Bottom)}}}}
  [ordered]@{schema='chemsema.gui.worker-attestation.v1';operation='uia-query';workerId=$WorkerId;query=[ordered]@{schema='chemsema.gui.uia-query.v1';processId=[int]$owned.process.Id;name=$AutomationName;automationId=$AutomationId;controlType=$AutomationControlType;topLevels=$topLevels;matches=$matches}}
}

function Get-InteractiveAttestation {$agent=Invoke-Agent @('attest');[ordered]@{schema='chemsema.gui.worker-attestation.v1';operation='agent-attest-interactive';workerId=$WorkerId;agent=$agent}}
function Stop-Worker {$candidate=Read-State 'candidate.json';$agent=Read-State 'agent.json';$stopped=@();if(Stop-OwnedProcess 'input-process.json' @([string]$agent.path)){$stopped+='input-agent'};if(Stop-OwnedProcess 'cdp-process.json' @((Get-Command powershell.exe).Source)){$stopped+='cdp-agent'};if(Stop-OwnedProcess 'candidate-process.json' @([string]$candidate.path)){$stopped+='candidate'};if(Stop-OwnedProcess 'keep-awake.json' @((Get-Command powershell.exe).Source)){$stopped+='keep-awake'};Remove-State 'lease.json';[ordered]@{schema='chemsema.gui.worker-attestation.v1';operation='stop';workerId=$WorkerId;state='stopped';scope='test-owned-processes-only';stopped=$stopped}}

switch($Operation){
  'host-attest'{Write-Result(Get-HostAttestation)} 'reset'{Write-Result(Reset-Worker)} 'start'{Write-Result(Start-Worker)} 'guest-attest'{Write-Result(Get-GuestAttestation)} 'prepare-guest'{Write-Result(Prepare-Guest)}
  'install-agent'{Write-Result(Install-Agent)} 'configure-desktop-baseline'{Write-Result(Configure-DesktopBaseline)} 'install-candidate'{Write-Result(Install-Candidate)} 'launch-candidate'{Write-Result(Start-Candidate)}
  'dismiss-known-blocker'{Write-Result(Dismiss-KnownBlocker)} 'activate-candidate'{Write-Result(Activate-Candidate)} 'start-input-agent'{Write-Result(Start-InputAgent)} 'stop-input-agent'{Write-Result(Stop-InputAgent)}
  'start-cdp-agent'{Write-Result(Start-CdpAgent)} 'stop-cdp-agent'{Write-Result(Stop-CdpAgent)} 'uia-query'{Write-Result(Query-Uia)} 'cdp-bridge'{Write-Result(Invoke-CdpBridge)} 'fetch-artifacts'{Write-Result(Receive-Artifacts)}
  'prepare-document-output'{Write-Result(Prepare-DocumentOutput)} 'fetch-document-output'{Write-Result(Fetch-DocumentOutput)} 'input-click'{Write-Result(Invoke-CandidateInput 'click')} 'input-drag'{Write-Result(Invoke-CandidateInput 'drag')}
  'input-key'{Write-Result(Invoke-CandidateInput 'key')} 'input-text'{Write-Result(Invoke-CandidateInput 'text')} 'agent-attest-interactive'{Write-Result(Get-InteractiveAttestation)} 'stop'{Write-Result(Stop-Worker)}
}
