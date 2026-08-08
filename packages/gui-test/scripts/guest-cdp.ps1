param(
  [string]$RequestBase64,
  [int]$Port = 9223,
  [string]$AllowedRoot,
  [string]$ChannelRoot
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

function Write-CdpResult([object]$Value) {
  $Value | ConvertTo-Json -Depth 12 -Compress
}

function Invoke-Cdp([Net.WebSockets.ClientWebSocket]$Socket, [string]$Method, [object]$Parameters) {
  $cancellation = [Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(15))
  try {
  $script:CdpRequestId += 1
  $requestId = $script:CdpRequestId
  $request = @{ id = $requestId; method = $Method; params = $Parameters } | ConvertTo-Json -Depth 12 -Compress
  $bytes = [Text.Encoding]::UTF8.GetBytes($request)
  [void]$Socket.SendAsync(
    [ArraySegment[byte]]::new($bytes),
    [Net.WebSockets.WebSocketMessageType]::Text,
    $true,
    $cancellation.Token
  ).GetAwaiter().GetResult()
  do {
    $stream = [IO.MemoryStream]::new()
    do {
      $buffer = New-Object byte[] 16384
      $received = $Socket.ReceiveAsync(
        [ArraySegment[byte]]::new($buffer),
        $cancellation.Token
      ).GetAwaiter().GetResult()
      if ($received.MessageType -eq [Net.WebSockets.WebSocketMessageType]::Close) {
        throw 'WebView2 CDP socket closed before returning a response.'
      }
      $stream.Write($buffer, 0, $received.Count)
    } while (-not $received.EndOfMessage)
    $message = [Text.Encoding]::UTF8.GetString($stream.ToArray()) | ConvertFrom-Json
  } while ($message.id -ne $requestId)
  if ($null -ne $message.error) {
    throw "CDP $Method failed: $($message.error.message)"
  }
  $message.result
  } finally {
    $cancellation.Dispose()
  }
}

function Invoke-CdpRequest([string]$EncodedRequest) {
try {
  if ([string]::IsNullOrWhiteSpace($EncodedRequest)) { throw 'The CDP request is absent.' }
  $requestJson = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($EncodedRequest))
  $request = $requestJson | ConvertFrom-Json
  if ($request.mode -notin @('locate', 'state', 'count', 'count-state', 'distinct-count', 'distinct-count-state')) {
    throw "Unsupported CDP bridge mode '$($request.mode)'."
  }
  $deadline = [DateTime]::UtcNow.AddSeconds(30)
  do {
    try {
      $targets = @(Invoke-RestMethod "http://127.0.0.1:$Port/json")
      $target = $targets | Where-Object { $_.type -eq 'page' -and $_.url -eq 'http://tauri.localhost/' } | Select-Object -First 1
      if ($null -ne $target) { break }
    } catch {
      $target = $null
    }
    Start-Sleep -Milliseconds 200
  } while ([DateTime]::UtcNow -lt $deadline)
  if ($null -eq $target) {
    throw 'The ChemSema WebView2 CDP page target is unavailable.'
  }
  $socket = [Net.WebSockets.ClientWebSocket]::new()
  try {
    $connectCancellation = [Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(15))
    try {
    [void]$socket.ConnectAsync(
      [Uri]$target.webSocketDebuggerUrl,
      $connectCancellation.Token
    ).GetAwaiter().GetResult()
    } finally {
      $connectCancellation.Dispose()
    }
    if ($request.mode -eq 'state') {
      $expression = @'
(() => ({
  runtimeState: document.body.dataset.runtimeState || null,
  revision: Number.isInteger(window.__chemsemaDebug?.state?.revision) ? window.__chemsemaDebug.state.revision : null,
  appScript: document.querySelector('script[type="module"]')?.src || null,
  engine: (() => {
    const debug = window.__chemsemaDebug;
    const session = debug?.state?.editorEngine;
    let documentBonds = null;
    let lastCommandResult = null;
    try {
      const parsed = JSON.parse(session?.documentJson?.() || 'null');
      documentBonds = Object.values(parsed?.resources || {}).reduce((count, resource) => count + (resource?.data?.bonds?.length || 0), 0);
    } catch {}
    try { lastCommandResult = JSON.parse(session?.lastCommandResultJson?.() || 'null'); } catch {}
    return {
      hostKind: debug?.engineHost?.kind || null,
      sessionType: session?.constructor?.name || null,
      editingRustDocument: !debug?.state?.currentPath && !!session,
      canUndo: session?.canUndo?.() ?? null,
      canRedo: session?.canRedo?.() ?? null,
      documentBonds,
      lastCommandResult,
      lastCommandSync: debug?.renderStats?.lastCommandSync || null
    };
  })(),
  window: { href: location.href, title: document.title, visibilityState: document.visibilityState, focused: document.hasFocus() },
  viewport: { width: innerWidth, height: innerHeight, devicePixelRatio },
  rendered: { bonds: document.querySelectorAll('[data-bond-id]').length, nodes: document.querySelectorAll('[data-node-id]').length }
}))()
'@
    } elseif ($request.mode -in @('count', 'count-state', 'distinct-count', 'distinct-count-state')) {
      $selectorBase64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes([string]$request.selector))
      $distinct = $request.mode -in @('distinct-count', 'distinct-count-state')
      if ($distinct -and $request.attribute -notin @('data-object-id', 'data-node-id', 'data-bond-id')) {
        throw "Unsupported distinct-count attribute '$($request.attribute)'."
      }
      $countExpression = if ($distinct) {
        "new Set([...document.querySelectorAll(new TextDecoder().decode(Uint8Array.from(atob('$selectorBase64'), c => c.charCodeAt(0))))].map(element => element.getAttribute('$([string]$request.attribute)')).filter(value => value !== null && value !== '')).size"
      } else {
        "document.querySelectorAll(new TextDecoder().decode(Uint8Array.from(atob('$selectorBase64'), c => c.charCodeAt(0)))).length"
      }
      if ($request.mode -in @('count', 'distinct-count')) {
        $expression = "(() => $countExpression)()"
      } else {
        $expression = @"
(() => ({
  count: $countExpression,
  state: {
    revision: Number.isInteger(window.__chemsemaDebug?.state?.revision) ? window.__chemsemaDebug.state.revision : null,
    appScript: document.querySelector('script[type="module"]')?.src || null,
    engine: (() => {
      const debug = window.__chemsemaDebug;
      const session = debug?.state?.editorEngine;
      let documentBonds = null;
      let lastCommandResult = null;
      try {
        const parsed = JSON.parse(session?.documentJson?.() || 'null');
        documentBonds = Object.values(parsed?.resources || {}).reduce((count, resource) => count + (resource?.data?.bonds?.length || 0), 0);
      } catch {}
      try { lastCommandResult = JSON.parse(session?.lastCommandResultJson?.() || 'null'); } catch {}
      return { hostKind: debug?.engineHost?.kind || null, sessionType: session?.constructor?.name || null, editingRustDocument: !debug?.state?.currentPath && !!session, canUndo: session?.canUndo?.() ?? null, canRedo: session?.canRedo?.() ?? null, documentBonds, lastCommandResult, lastCommandSync: debug?.renderStats?.lastCommandSync || null };
    })(),
    window: { href: location.href, title: document.title, visibilityState: document.visibilityState, focused: document.hasFocus() },
    rendered: { bonds: document.querySelectorAll('[data-bond-id]').length, nodes: document.querySelectorAll('[data-node-id]').length }
  }
}))()
"@
      }
    } else {
      $targetBase64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes(($request.target | ConvertTo-Json -Depth 8 -Compress)))
      $expression = @"
(() => {
  const target = JSON.parse(new TextDecoder().decode(Uint8Array.from(atob('$targetBase64'), c => c.charCodeAt(0))));
  const roleOf = (element) => element.getAttribute('role') || ({BUTTON:'button',ASIDE:'complementary',MAIN:'main'}[element.tagName] || null);
  const nameOf = (element) => element.getAttribute('aria-label') || element.getAttribute('title') || element.textContent?.trim() || '';
  const find = (query, root) => {
    if (query.strategy === 'automation-id' || query.strategy === 'test-id') {
      const element = root.querySelector('#' + CSS.escape(query.value));
      return element ? [element] : [];
    }
    if (query.strategy === 'role') {
      return [...root.querySelectorAll('*')].filter(element => roleOf(element) === query.value && (!query.name || nameOf(element) === query.name));
    }
    throw new Error('Unsupported CDP target strategy ' + query.strategy);
  };
  let root = document;
  if (target.scope) {
    const scopes = find({ strategy: 'role', value: target.scope.role, name: target.scope.name }, document);
    if (scopes.length !== 1) return { scopeCount: scopes.length, matches: [] };
    root = scopes[0];
  }
  const matches = find(target, root).map(element => {
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return {
      tag: element.tagName.toLowerCase(),
      role: roleOf(element),
      name: nameOf(element),
      automationId: element.id || null,
      disabled: !!element.disabled || element.getAttribute('aria-disabled') === 'true',
      visible: rect.width > 0 && rect.height > 0 && style.visibility !== 'hidden' && style.display !== 'none',
      rect: [rect.left, rect.top, rect.right, rect.bottom]
    };
  });
  return { scopeCount: target.scope ? 1 : null, matches };
})()
"@
    }
    $evaluation = Invoke-Cdp $socket 'Runtime.evaluate' @{
      expression = $expression
      returnByValue = $true
      awaitPromise = $true
    }
    if ($null -ne $evaluation.exceptionDetails) {
      throw "CDP evaluation failed: $($evaluation.exceptionDetails.exception.description)"
    }
    return [ordered]@{
      schema = 'chemsema.gui.cdp-bridge.v1'
      status = 'passed'
      mode = [string]$request.mode
      value = $evaluation.result.value
    }
  } finally {
    if ($null -ne $socket) { $socket.Dispose() }
  }
} catch {
  return [ordered]@{
    schema = 'chemsema.gui.cdp-bridge.v1'
    status = 'failed'
    message = $_.Exception.Message
  }
}
}

function Test-BoundedChannel([string]$Root, [string]$Channel) {
  if ([string]::IsNullOrWhiteSpace($Root) -or [string]::IsNullOrWhiteSpace($Channel)) { return $false }
  $rootPath = [IO.Path]::GetFullPath($Root).TrimEnd('\')
  $channelPath = [IO.Path]::GetFullPath($Channel)
  return $channelPath.StartsWith(($rootPath + '\'), [StringComparison]::OrdinalIgnoreCase)
}

function Start-CdpServer {
  if (-not (Test-BoundedChannel $AllowedRoot $ChannelRoot)) { throw 'Persistent CDP channel is outside the authorized test root.' }
  $inbox = Join-Path $ChannelRoot 'inbox'
  $outbox = Join-Path $ChannelRoot 'outbox'
  New-Item -ItemType Directory -Path $inbox -Force | Out-Null
  New-Item -ItemType Directory -Path $outbox -Force | Out-Null
  $identity = [Security.Principal.WindowsIdentity]::GetCurrent().Name
  $sessionId = [Diagnostics.Process]::GetCurrentProcess().SessionId
  $ready = [ordered]@{ schema='chemsema.gui.cdp-server.v1'; status='ready'; processId=$PID; sessionId=$sessionId; account=$identity; port=$Port }
  [IO.File]::WriteAllText((Join-Path $ChannelRoot 'ready.json'), ($ready | ConvertTo-Json -Compress), [Text.UTF8Encoding]::new($false))
  while (-not (Test-Path -LiteralPath (Join-Path $ChannelRoot 'shutdown') -PathType Leaf)) {
    $requests = @(Get-ChildItem -LiteralPath $inbox -Filter '*.json' -File -ErrorAction SilentlyContinue | Sort-Object Name)
    foreach ($requestPath in $requests) {
      $claimPath = [IO.Path]::ChangeExtension($requestPath.FullName, '.claim')
      try { Move-Item -LiteralPath $requestPath.FullName -Destination $claimPath -ErrorAction Stop } catch { continue }
      $id = [IO.Path]::GetFileNameWithoutExtension($claimPath)
      try {
        $envelope = Get-Content -Raw -LiteralPath $claimPath | ConvertFrom-Json
        if ($envelope.schema -ne 'chemsema.gui.cdp-request.v1' -or $envelope.id -ne $id -or [string]::IsNullOrWhiteSpace($id)) {
          throw 'Persistent CDP request identity is invalid.'
        }
        $bridge = Invoke-CdpRequest ([string]$envelope.requestBase64)
        if ($bridge.status -ne 'passed') { throw [string]$bridge.message }
        $response = [ordered]@{ schema='chemsema.gui.cdp-response.v1'; id=$id; status='passed'; bridge=$bridge }
      } catch {
        $response = [ordered]@{ schema='chemsema.gui.cdp-response.v1'; id=$id; status='failed'; message=$_.Exception.Message }
      }
      $output = Join-Path $outbox ($id + '.json')
      $temporary = [IO.Path]::ChangeExtension($output, '.tmp')
      [IO.File]::WriteAllText($temporary, ($response | ConvertTo-Json -Depth 16 -Compress), [Text.UTF8Encoding]::new($false))
      Move-Item -LiteralPath $temporary -Destination $output -Force
      Remove-Item -LiteralPath $claimPath -Force -ErrorAction SilentlyContinue
    }
    Start-Sleep -Milliseconds 20
  }
}

if (-not [string]::IsNullOrWhiteSpace($ChannelRoot)) {
  Start-CdpServer
} else {
  Write-CdpResult (Invoke-CdpRequest $RequestBase64)
}
