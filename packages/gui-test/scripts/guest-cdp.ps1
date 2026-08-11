param(
  [string]$RequestBase64,
  [int]$Port = 9223,
  [string]$AllowedRoot,
  [string]$ChannelRoot
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$script:CdpRequestId = 0
$script:CdpEvents = [Collections.Generic.List[object]]::new()
$script:PersistentCdpSocket = $null
$script:PersistentCdpTargetUrl = $null
$script:TraceActive = $false

Add-Type -AssemblyName System.Web.Extensions
$script:CdpJsonSerializer = New-Object System.Web.Script.Serialization.JavaScriptSerializer
$script:CdpJsonSerializer.MaxJsonLength = [int]::MaxValue
$script:CdpJsonSerializer.RecursionLimit = 256

$MaximumArtifactBytes = 64 * 1024 * 1024
$TraceCategories = 'devtools.timeline,disabled-by-default-devtools.timeline,blink.user_timing,v8.execute,loading,latencyInfo,renderer.scheduler'

function Get-Sha256Hex([string]$Path) {
  $stream = [IO.File]::Open($Path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::ReadWrite)
  $algorithm = [Security.Cryptography.SHA256]::Create()
  try {
    $hash = $algorithm.ComputeHash($stream)
    return ($hash | ForEach-Object { $_.ToString('x2') }) -join ''
  } finally {
    $algorithm.Dispose()
    $stream.Dispose()
  }
}

function Write-CdpResult([object]$Value) {
  $Value | ConvertTo-Json -Depth 12 -Compress
}

function Receive-CdpMessage([Net.WebSockets.ClientWebSocket]$Socket, [Threading.CancellationToken]$CancellationToken) {
  $stream = [IO.MemoryStream]::new()
  try {
    do {
      $buffer = New-Object byte[] 16384
      $received = $Socket.ReceiveAsync(
        [ArraySegment[byte]]::new($buffer),
        $CancellationToken
      ).GetAwaiter().GetResult()
      if ($received.MessageType -eq [Net.WebSockets.WebSocketMessageType]::Close) {
        throw 'WebView2 CDP socket closed before returning a response.'
      }
      $stream.Write($buffer, 0, $received.Count)
    } while (-not $received.EndOfMessage)
    # CDP event payloads can legally contain object keys that differ only by
    # casing. Windows PowerShell's ConvertFrom-Json treats those keys as
    # duplicates, while JavaScriptSerializer preserves them in a
    # case-sensitive Dictionary<string, object>.
    return $script:CdpJsonSerializer.DeserializeObject([Text.Encoding]::UTF8.GetString($stream.ToArray()))
  } finally {
    $stream.Dispose()
  }
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
    $message = Receive-CdpMessage $Socket $cancellation.Token
    if ($null -ne $message.method) { $script:CdpEvents.Add($message) }
  } while ($message.id -ne $requestId)
  if ($null -ne $message.error) {
    throw "CDP $Method failed: $($message.error.message)"
  }
  $message.result
  } finally {
    $cancellation.Dispose()
  }
}

function Wait-CdpEvent([Net.WebSockets.ClientWebSocket]$Socket, [string]$Method) {
  for ($index = 0; $index -lt $script:CdpEvents.Count; $index += 1) {
    if ($script:CdpEvents[$index].method -eq $Method) {
      $event = $script:CdpEvents[$index]
      $script:CdpEvents.RemoveAt($index)
      return $event.params
    }
  }
  $cancellation = [Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(30))
  try {
    while ($true) {
      $message = Receive-CdpMessage $Socket $cancellation.Token
      if ($message.method -eq $Method) { return $message.params }
      if ($null -ne $message.method) { $script:CdpEvents.Add($message) }
    }
  } finally {
    $cancellation.Dispose()
  }
}

function Read-CdpStream([Net.WebSockets.ClientWebSocket]$Socket, [string]$Handle) {
  $output = [IO.MemoryStream]::new()
  try {
    do {
      $chunk = Invoke-Cdp $Socket 'IO.read' @{ handle = $Handle; size = 1024 * 1024 }
      $chunkData = [string]$chunk.data
      if (-not [string]::IsNullOrEmpty($chunkData)) {
        $bytes = if ($chunk.base64Encoded) {
          [Convert]::FromBase64String($chunkData)
        } else {
          [Text.Encoding]::UTF8.GetBytes($chunkData)
        }
        if (($output.Length + $bytes.Length) -gt $MaximumArtifactBytes) {
          throw 'Performance trace exceeds 64 MiB.'
        }
        $output.Write($bytes, 0, $bytes.Length)
      }
    } while (-not $chunk.eof)
    return ,$output.ToArray()
  } finally {
    try { [void](Invoke-Cdp $Socket 'IO.close' @{ handle = $Handle }) } catch {}
    $output.Dispose()
  }
}

function Invoke-CdpRequest([string]$EncodedRequest) {
try {
  if ([string]::IsNullOrWhiteSpace($EncodedRequest)) { throw 'The CDP request is absent.' }
  $requestJson = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($EncodedRequest))
  $request = $requestJson | ConvertFrom-Json
  if ($request.mode -notin @('locate', 'state', 'count', 'count-state', 'distinct-count', 'distinct-count-state', 'text', 'text-state', 'entity-rects-state', 'ui-state', 'trace-start', 'trace-mark', 'artifact-export')) {
    throw "Unsupported CDP bridge mode '$($request.mode)'."
  }
  if ($request.mode -in @('count', 'count-state', 'distinct-count', 'distinct-count-state', 'text', 'text-state') -and
    ([string]::IsNullOrWhiteSpace([string]$request.selector) -or ([string]$request.selector).Length -gt 2048)) {
    throw 'DOM observation requires a selector of 1 to 2048 characters.'
  }
  if ($request.mode -eq 'ui-state') {
    $allowedStyleProperties = @('backgroundColor', 'borderColor', 'boxShadow', 'cursor', 'display', 'fill', 'opacity', 'outlineColor', 'outlineStyle', 'outlineWidth', 'pointerEvents', 'stroke', 'strokeWidth', 'visibility')
    if ([string]::IsNullOrWhiteSpace([string]$request.selector) -or ([string]$request.selector).Length -gt 2048) {
      throw 'UI state observation requires a selector of 1 to 2048 characters.'
    }
    if ($null -ne $request.referenceSelector -and ([string]::IsNullOrWhiteSpace([string]$request.referenceSelector) -or ([string]$request.referenceSelector).Length -gt 2048)) {
      throw 'UI state reference requires a selector of 1 to 2048 characters.'
    }
    $styleProperties = if ($null -eq $request.styleProperties) {
      ,([object[]]::new(0))
    } else {
      ,@($request.styleProperties)
    }
    if ($styleProperties.Count -gt $allowedStyleProperties.Count -or @($styleProperties | Select-Object -Unique).Count -ne $styleProperties.Count -or @($styleProperties | Where-Object { $_ -notin $allowedStyleProperties }).Count -gt 0) {
      throw 'UI state styles must be unique allowlisted properties.'
    }
  }
  if ($request.mode -eq 'artifact-export' -and [string]$request.artifactId -notmatch '^[a-f0-9]{32}$') {
    throw 'Artifact export requires a 32-character lowercase hexadecimal identity.'
  }
  if ($request.mode -eq 'trace-mark' -and [string]$request.name -notmatch '^chemsema-action:[A-Za-z0-9._:-]{1,220}$') {
    throw 'Trace marks require a bounded ChemSema action marker.'
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
  $persistent = -not [string]::IsNullOrWhiteSpace($ChannelRoot)
  $socket = if ($persistent -and $null -ne $script:PersistentCdpSocket -and
    $script:PersistentCdpSocket.State -eq [Net.WebSockets.WebSocketState]::Open -and
    $script:PersistentCdpTargetUrl -eq [string]$target.webSocketDebuggerUrl) {
    $script:PersistentCdpSocket
  } else {
    if ($null -ne $script:PersistentCdpSocket) { $script:PersistentCdpSocket.Dispose() }
    [Net.WebSockets.ClientWebSocket]::new()
  }
  try {
    if ($socket.State -ne [Net.WebSockets.WebSocketState]::Open) {
      $connectCancellation = [Threading.CancellationTokenSource]::new([TimeSpan]::FromSeconds(15))
      try {
        [void]$socket.ConnectAsync(
          [Uri]$target.webSocketDebuggerUrl,
          $connectCancellation.Token
        ).GetAwaiter().GetResult()
      } finally {
        $connectCancellation.Dispose()
      }
      if ($persistent) {
        $script:PersistentCdpSocket = $socket
        $script:PersistentCdpTargetUrl = [string]$target.webSocketDebuggerUrl
      }
    }
    $screenshotBase64 = $null
    $traceBytes = $null
    if ($request.mode -eq 'trace-start') {
      if (-not $persistent) { throw 'Performance tracing requires the persistent CDP agent.' }
      if ($script:TraceActive) { throw 'Performance tracing is already active.' }
      [void](Invoke-Cdp $socket 'Tracing.start' @{
        categories = $TraceCategories
        transferMode = 'ReturnAsStream'
        streamFormat = 'json'
        streamCompression = 'gzip'
        bufferUsageReportingInterval = 1000
      })
      $script:TraceActive = $true
      return [ordered]@{
        schema = 'chemsema.gui.cdp-bridge.v1'
        status = 'passed'
        mode = 'trace-start'
        value = [ordered]@{ started = $true; categories = $TraceCategories }
      }
    }
    if ($request.mode -eq 'trace-mark') {
      if (-not $script:TraceActive) { throw 'Performance trace was not started before the action marker.' }
      $markerJson = [string]$request.name | ConvertTo-Json -Compress
      [void](Invoke-Cdp $socket 'Runtime.evaluate' @{ expression="performance.mark($markerJson); true"; returnByValue=$true })
      return [ordered]@{
        schema = 'chemsema.gui.cdp-bridge.v1'
        status = 'passed'
        mode = 'trace-mark'
        value = [ordered]@{ marked=$true; name=[string]$request.name }
      }
    }
    if ($request.mode -eq 'artifact-export') {
      if (-not $script:TraceActive) { throw 'Performance trace was not started before the scenario.' }
      [void](Invoke-Cdp $socket 'Tracing.end' @{})
      $traceComplete = Wait-CdpEvent $socket 'Tracing.tracingComplete'
      $script:TraceActive = $false
      if ($traceComplete.dataLossOccurred) { throw 'WebView2 reported data loss in the performance trace.' }
      if ([string]::IsNullOrWhiteSpace([string]$traceComplete.stream)) { throw 'WebView2 did not return a performance trace stream.' }
      if ($traceComplete.traceFormat -and $traceComplete.traceFormat -ne 'json') { throw "Unexpected performance trace format '$($traceComplete.traceFormat)'." }
      $traceBytes = Read-CdpStream $socket ([string]$traceComplete.stream)
      if ($traceBytes.Length -eq 0) { throw 'The performance trace is empty.' }
      $screenshot = Invoke-Cdp $socket 'Page.captureScreenshot' @{
        format = 'png'
        fromSurface = $true
        captureBeyondViewport = $false
      }
      $screenshotBase64 = [string]$screenshot.data
      if ([string]::IsNullOrWhiteSpace($screenshotBase64) -or $screenshotBase64.Length -gt (16 * 1024 * 1024)) {
        throw 'CDP screenshot is absent or exceeds the bounded artifact channel.'
      }
      $expression = @'
(() => {
  const boundUtf8 = (value, maximumBytes) => {
    const encoded = new TextEncoder().encode(value || '');
    return {
      value: new TextDecoder().decode(encoded.slice(0, maximumBytes)),
      truncated: encoded.length > maximumBytes,
      originalBytes: encoded.length
    };
  };
  const dom = boundUtf8(document.documentElement?.outerHTML || '', 64 * 1024 * 1024);
  return {
    state: {
      runtimeState: document.body.dataset.runtimeState || null,
      revision: null,
      appScript: document.querySelector('script[type="module"]')?.src || null,
      engine: null,
      window: { href: location.href, title: document.title, visibilityState: document.visibilityState, focused: document.hasFocus() },
      viewport: { width: innerWidth, height: innerHeight, devicePixelRatio },
      rendered: {
        bonds: document.querySelectorAll('[data-bond-id]').length,
        nodes: document.querySelectorAll('[data-node-id]').length,
        objects: document.querySelectorAll('[data-object-id]').length,
        overlays: document.querySelectorAll('#editor-overlay-layer > *').length
      }
    },
    domHtml: dom.value,
    truncation: {
      domHtml: dom.truncated,
      domHtmlOriginalBytes: dom.originalBytes,
      performanceTrace: false,
      performanceTraceOriginalBytes: null
    }
  };
})()
'@
    } elseif ($request.mode -eq 'state') {
      $expression = @'
(() => ({
  runtimeState: document.body.dataset.runtimeState || null,
  revision: null,
  appScript: document.querySelector('script[type="module"]')?.src || null,
  engine: null,
  window: { href: location.href, title: document.title, visibilityState: document.visibilityState, focused: document.hasFocus() },
  viewport: { width: innerWidth, height: innerHeight, devicePixelRatio },
  rendered: { bonds: document.querySelectorAll('[data-bond-id]').length, nodes: document.querySelectorAll('[data-node-id]').length }
}))()
'@
    } elseif ($request.mode -eq 'entity-rects-state') {
      $entityIds = @($request.entityIds)
      if ($entityIds.Count -lt 1 -or $entityIds.Count -gt 16 -or @($entityIds | Select-Object -Unique).Count -ne $entityIds.Count) {
        throw 'Entity rectangle observation requires 1 to 16 unique ids.'
      }
      if (@($entityIds | Where-Object { [string]::IsNullOrWhiteSpace([string]$_) -or ([string]$_).Length -gt 128 }).Count -gt 0) {
        throw 'Entity rectangle observation contains an invalid id.'
      }
      $entityIdsBase64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes(($entityIds | ConvertTo-Json -Compress)))
      $expression = @"
(() => {
  const entityIds = JSON.parse(new TextDecoder().decode(Uint8Array.from(atob('$entityIdsBase64'), c => c.charCodeAt(0))));
  const entities = entityIds.map(entityId => {
    const allMatches = [...document.querySelectorAll('[data-object-id="' + CSS.escape(entityId) + '"]')];
    const renderRoots = allMatches.filter(element => element.hasAttribute('data-renderer'));
    const isVisibleEntityElement = element => {
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      return (rect.width > 0 || rect.height > 0) && style.visibility !== 'hidden' && style.display !== 'none';
    };
    const visibleRenderRoots = renderRoots.filter(isVisibleEntityElement);
    const elements = visibleRenderRoots.length ? visibleRenderRoots : allMatches;
    const visibleElements = elements.filter(isVisibleEntityElement);
    const screenRects = visibleElements.map(element => element.getBoundingClientRect());
    const screenXs = screenRects.flatMap(rect => [rect.left, rect.right]);
    const screenYs = screenRects.flatMap(rect => [rect.top, rect.bottom]);
    const rect = screenRects.length
      ? [Math.min(...screenXs), Math.min(...screenYs), Math.max(...screenXs), Math.max(...screenYs)]
      : null;
    const worldPoints = [];
    for (const element of visibleElements) {
      const localBounds = typeof element.getBBox === 'function' ? element.getBBox() : null;
      const documentRoot = element.closest?.('[data-layer="document-content"]') || null;
      const elementMatrix = element.getCTM?.() || null;
      const rootMatrix = documentRoot?.getCTM?.() || null;
      if (!localBounds || !elementMatrix || !rootMatrix) continue;
      const relativeMatrix = rootMatrix.inverse().multiply(elementMatrix);
      worldPoints.push(...[
        new DOMPoint(localBounds.x, localBounds.y),
        new DOMPoint(localBounds.x + localBounds.width, localBounds.y),
        new DOMPoint(localBounds.x, localBounds.y + localBounds.height),
        new DOMPoint(localBounds.x + localBounds.width, localBounds.y + localBounds.height)
      ].map(point => point.matrixTransform(relativeMatrix)));
    }
    const worldXs = worldPoints.map(point => point.x);
    const worldYs = worldPoints.map(point => point.y);
    const worldRect = worldPoints.length
      ? [Math.min(...worldXs), Math.min(...worldYs), Math.max(...worldXs), Math.max(...worldYs)]
      : null;
    return {
      entityId,
      matchCount: visibleRenderRoots.length || (visibleElements.length ? 1 : 0),
      visible: visibleElements.length > 0,
      rect,
      worldRect
    };
  });
  return {
    entities,
    state: {
      runtimeState: document.body.dataset.runtimeState || null,
      revision: null,
      appScript: document.querySelector('script[type="module"]')?.src || null,
      engine: null,
      window: { href: location.href, title: document.title, visibilityState: document.visibilityState, focused: document.hasFocus() },
      rendered: { bonds: document.querySelectorAll('[data-bond-id]').length, nodes: document.querySelectorAll('[data-node-id]').length }
    }
  };
})()
"@
    } elseif ($request.mode -eq 'ui-state') {
      $uiRequest = [ordered]@{
        selector = [string]$request.selector
        referenceSelector = if ($null -ne $request.referenceSelector) { [string]$request.referenceSelector } else { $null }
        styleProperties = $styleProperties
      }
      $uiRequestBase64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes(($uiRequest | ConvertTo-Json -Depth 4 -Compress)))
      $expression = @"
(() => {
  const request = JSON.parse(new TextDecoder().decode(Uint8Array.from(atob('$uiRequestBase64'), c => c.charCodeAt(0))));
  const visible = element => {
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return (rect.width > 0 || rect.height > 0) && style.display !== 'none' && style.visibility !== 'hidden';
  };
  const rectArray = rect => [rect.left, rect.top, rect.right, rect.bottom];
  const union = rects => rects.length ? [
    Math.min(...rects.map(rect => rect[0])),
    Math.min(...rects.map(rect => rect[1])),
    Math.max(...rects.map(rect => rect[2])),
    Math.max(...rects.map(rect => rect[3]))
  ] : null;
  const observe = selector => {
    if (!selector) return null;
    const elements = [...document.querySelectorAll(selector)].slice(0, 128);
    const visibleElements = elements.filter(visible);
    const rects = visibleElements.slice(0, 32).map(element => rectArray(element.getBoundingClientRect()));
    const styleValues = {};
    for (const property of request.styleProperties) {
      styleValues[property] = [...new Set(visibleElements.slice(0, 32).map(element => getComputedStyle(element)[property]))];
    }
    return {
      count: elements.length,
      truncated: document.querySelectorAll(selector).length > 128,
      visibleCount: visibleElements.length,
      focusedCount: elements.filter(element => element === document.activeElement).length,
      focusWithinCount: elements.filter(element => element.contains(document.activeElement)).length,
      hoverCount: elements.filter(element => element.matches(':hover')).length,
      disabledCount: elements.filter(element => !!element.disabled || element.getAttribute('aria-disabled') === 'true').length,
      rects,
      unionRect: union(rects),
      styleValues
    };
  };
  const result = observe(request.selector);
  result.reference = observe(request.referenceSelector);
  result.viewport = { width: innerWidth, height: innerHeight, devicePixelRatio };
  result.windowFocused = document.hasFocus();
  result.activeElement = {
    tag: document.activeElement?.tagName?.toLowerCase() || null,
    id: document.activeElement?.id || null,
    role: document.activeElement?.getAttribute?.('role') || null,
    ariaLabel: document.activeElement?.getAttribute?.('aria-label') || null
  };
  return result;
})()
"@
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
    revision: null,
    appScript: document.querySelector('script[type="module"]')?.src || null,
    engine: null,
    window: { href: location.href, title: document.title, visibilityState: document.visibilityState, focused: document.hasFocus() },
    rendered: { bonds: document.querySelectorAll('[data-bond-id]').length, nodes: document.querySelectorAll('[data-node-id]').length }
  }
}))()
"@
      }
    } elseif ($request.mode -in @('text', 'text-state')) {
      $selectorBase64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes([string]$request.selector))
      $textExpression = @"
(() => {
  const selector = new TextDecoder().decode(Uint8Array.from(atob('$selectorBase64'), c => c.charCodeAt(0)));
  const elements = [...document.querySelectorAll(selector)];
  const textOf = element => ['INPUT', 'TEXTAREA', 'SELECT'].includes(element.tagName)
    ? element.value
    : element.textContent;
  return { count: elements.length, text: elements.length === 1 ? textOf(elements[0]) : null };
})()
"@
      if ($request.mode -eq 'text') {
        $expression = $textExpression
      } else {
        $expression = @"
(() => {
  const selector = new TextDecoder().decode(Uint8Array.from(atob('$selectorBase64'), c => c.charCodeAt(0)));
  const elements = [...document.querySelectorAll(selector)];
  return {
    count: elements.length,
      text: elements.length === 1
        ? (['INPUT', 'TEXTAREA', 'SELECT'].includes(elements[0].tagName) ? elements[0].value : elements[0].textContent)
        : null,
    state: {
      revision: null,
      appScript: document.querySelector('script[type="module"]')?.src || null,
      engine: null,
      window: { href: location.href, title: document.title, visibilityState: document.visibilityState, focused: document.hasFocus() },
      rendered: { bonds: document.querySelectorAll('[data-bond-id]').length, nodes: document.querySelectorAll('[data-node-id]').length }
    }
  };
})()
"@
      }
    } else {
      $targetBase64 = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes(($request.target | ConvertTo-Json -Depth 8 -Compress)))
      $expression = @"
(() => {
  const target = JSON.parse(new TextDecoder().decode(Uint8Array.from(atob('$targetBase64'), c => c.charCodeAt(0))));
  const inputRoleOf = (element) => ({
    button:'button', submit:'button', reset:'button', checkbox:'checkbox', radio:'radio',
    number:'spinbutton', range:'slider', search:'searchbox',
    email:'textbox', password:'textbox', tel:'textbox', text:'textbox', url:'textbox'
  }[element.type] || 'textbox');
  const roleOf = (element) => element.getAttribute('role') || ({BUTTON:'button',ASIDE:'complementary',MAIN:'main',TEXTAREA:'textbox',SELECT:'combobox'}[element.tagName] || (element.tagName === 'INPUT' ? inputRoleOf(element) : null));
  const labelName = (label) => {
    const heading = label.querySelector(':scope > span');
    if (heading?.textContent?.trim()) return heading.textContent.trim();
    const clone = label.cloneNode(true);
    clone.querySelectorAll('input, select, textarea, button, option, em').forEach(control => control.remove());
    return clone.textContent?.trim() || '';
  };
  const nameOf = (element) => {
    const labelledBy = (element.getAttribute('aria-labelledby') || '').split(/\s+/).filter(Boolean)
      .map(id => document.getElementById(id)?.textContent?.trim() || '').filter(Boolean).join(' ');
    const associatedLabels = [...(element.labels || [])].map(labelName).filter(Boolean).join(' ');
    return element.getAttribute('aria-label') || labelledBy || associatedLabels
      || element.getAttribute('title') || element.getAttribute('placeholder') || element.textContent?.trim() || '';
  };
  const visibleElement = element => {
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    return (rect.width > 0 || rect.height > 0) && style.visibility !== 'hidden' && style.display !== 'none';
  };
  const geometryPointerRect = element => {
    const selector = '[data-role="document-graphic"], path, line, polyline, polygon, circle, ellipse, rect';
    const geometryElements = [element, ...element.querySelectorAll(selector)]
      .filter((candidate, index, candidates) => candidates.indexOf(candidate) === index)
      .filter(candidate => typeof candidate.getTotalLength === 'function' && typeof candidate.getPointAtLength === 'function')
      .filter(visibleElement);
    const documentGraphics = geometryElements.filter(candidate => candidate.getAttribute('data-role') === 'document-graphic');
    const candidates = documentGraphics.length ? documentGraphics : geometryElements;
    let best = null;
    for (const candidate of candidates) {
      try {
        const length = candidate.getTotalLength();
        const matrix = candidate.getScreenCTM();
        if (!Number.isFinite(length) || length <= 0 || !matrix) continue;
        const midpoint = candidate.getPointAtLength(length * 0.5);
        const clientPoint = new DOMPoint(midpoint.x, midpoint.y).matrixTransform(matrix);
        if (!Number.isFinite(clientPoint.x) || !Number.isFinite(clientPoint.y)) continue;
        if (!best || length > best.length) best = { length, x: clientPoint.x, y: clientPoint.y };
      } catch {
        // Unsupported or temporarily detached SVG geometry falls back to its rendered rectangle.
      }
    }
    return best
      ? { left: best.x - 0.5, top: best.y - 0.5, right: best.x + 0.5, bottom: best.y + 0.5, width: 1, height: 1 }
      : null;
  };
  const find = (query, root) => {
    if (query.strategy === 'automation-id' || query.strategy === 'test-id') {
      const element = root.querySelector('#' + CSS.escape(query.value));
      return element ? [element] : [];
    }
    if (query.strategy === 'role') {
      return [...root.querySelectorAll('*')].filter(element => roleOf(element) === query.value && (!query.name || nameOf(element) === query.name));
    }
    if (query.strategy === 'selector') {
      return [...root.querySelectorAll(query.value)];
    }
    if (query.strategy === 'entity-id') {
      const matches = [...root.querySelectorAll('[data-object-id="' + CSS.escape(query.value) + '"]')];
      const visibleCandidate = candidate => {
        const rect = candidate.getBoundingClientRect();
        const style = getComputedStyle(candidate);
        return (rect.width > 0 || rect.height > 0) && style.visibility !== 'hidden' && style.display !== 'none';
      };
      const element = matches.find(candidate => candidate.hasAttribute('data-renderer') && visibleCandidate(candidate)) || matches.find(visibleCandidate);
      return element ? [element] : [];
    }
    if (query.strategy === 'world-geometry') {
      if (query.value !== 'page-background') throw new Error('Unsupported world geometry target ' + query.value);
      const element = root.querySelector('[data-layer="page-background"]');
      return element ? [element] : [];
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
    const semanticPointerElement = target.strategy === 'entity-id'
      && element.getAttribute('data-object-type') === 'group'
      ? [...element.querySelectorAll('[data-role^="document-"], [data-bond-id], [data-node-id]')]
        .find(visibleElement) || element
      : element;
    const renderedRect = semanticPointerElement.getBoundingClientRect();
    const selectorGeometryRect = target.strategy === 'selector'
      && (renderedRect.width === 0 || renderedRect.height === 0)
      ? geometryPointerRect(element)
      : null;
    const rect = target.strategy === 'entity-id'
      ? geometryPointerRect(element) || renderedRect
      : selectorGeometryRect || renderedRect;
    const style = getComputedStyle(element);
    return {
      tag: element.tagName.toLowerCase(),
      role: roleOf(element),
      name: nameOf(element),
      automationId: element.id || null,
      disabled: !!element.disabled || element.getAttribute('aria-disabled') === 'true',
      visible: (target.strategy === 'entity-id' ? (rect.width > 0 || rect.height > 0) : (rect.width > 0 && rect.height > 0)) && style.visibility !== 'hidden' && style.display !== 'none',
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
    $value = $evaluation.result.value
    if ($request.mode -eq 'artifact-export') {
      $snapshot = $value
      $logBytes = [byte[]]::new(0)
      $logTruncated = $false
      $logOriginalBytes = 0
      $candidateRoot = Join-Path $AllowedRoot 'candidate'
      $logPath = Get-ChildItem -LiteralPath $candidateRoot -Filter 'webview.log' -File -Recurse -ErrorAction SilentlyContinue |
        Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1
      if ($null -ne $logPath) {
        $maximumLogBytes = 16 * 1024 * 1024
        $stream = [IO.File]::Open($logPath.FullName, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::ReadWrite)
        try {
          $logOriginalBytes = [int64]$stream.Length
          $readBytes = [int][Math]::Min($logOriginalBytes, $maximumLogBytes)
          if ($logOriginalBytes -gt $readBytes) {
            [void]$stream.Seek(-$readBytes, [IO.SeekOrigin]::End)
            $logTruncated = $true
          }
          $buffer = [byte[]]::new($readBytes)
          $offset = 0
          while ($offset -lt $readBytes) {
            $read = $stream.Read($buffer, $offset, $readBytes - $offset)
            if ($read -le 0) { break }
            $offset += $read
          }
          if ($offset -eq $buffer.Length) {
            $logBytes = $buffer
          } else {
            $logBytes = [byte[]]::new($offset)
            [Array]::Copy($buffer, $logBytes, $offset)
          }
        } finally {
          $stream.Dispose()
        }
      }
      $snapshot.truncation | Add-Member -NotePropertyName webviewLog -NotePropertyValue $logTruncated
      $snapshot.truncation | Add-Member -NotePropertyName webviewLogOriginalBytes -NotePropertyValue $logOriginalBytes
      $snapshot.truncation.performanceTraceOriginalBytes = [int64]$traceBytes.Length
      $artifactRoot = Join-Path (Join-Path $AllowedRoot 'artifacts') ([string]$request.artifactId)
      $allowedArtifactRoot = [IO.Path]::GetFullPath((Join-Path $AllowedRoot 'artifacts')).TrimEnd('\') + '\'
      $resolvedArtifactRoot = [IO.Path]::GetFullPath($artifactRoot).TrimEnd('\') + '\'
      if (-not $resolvedArtifactRoot.StartsWith($allowedArtifactRoot, [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Artifact export path escaped the authorized test root.'
      }
      if (Test-Path -LiteralPath $artifactRoot) { Remove-Item -LiteralPath $artifactRoot -Recurse -Force }
      New-Item -ItemType Directory -Path $artifactRoot -Force | Out-Null

      $stateValue = [ordered]@{
        schema = 'chemsema.gui.production-snapshot.v1'
        state = $snapshot.state
        truncation = $snapshot.truncation
      }
      $payloads = @(
        [ordered]@{ name='final-screenshot.png'; mediaType='image/png'; bytes=[Convert]::FromBase64String($screenshotBase64); truncated=$false },
        [ordered]@{ name='final-state.json'; mediaType='application/json'; bytes=[Text.Encoding]::UTF8.GetBytes(($stateValue | ConvertTo-Json -Depth 16)); truncated=$false },
        [ordered]@{ name='final-dom.html'; mediaType='text/html'; bytes=[Text.Encoding]::UTF8.GetBytes([string]$snapshot.domHtml); truncated=[bool]$snapshot.truncation.domHtml },
        [ordered]@{ name='performance-trace.json.gz'; mediaType='application/gzip'; bytes=$traceBytes; truncated=$false },
        [ordered]@{ name='webview.log'; mediaType='text/plain'; bytes=$logBytes; truncated=$logTruncated }
      )
      $exported = @()
      foreach ($payload in $payloads) {
        if ($payload.bytes.Length -gt (64 * 1024 * 1024)) { throw "Artifact $($payload.name) exceeds 64 MiB." }
        $path = Join-Path $artifactRoot $payload.name
        [IO.File]::WriteAllBytes($path, $payload.bytes)
        $hash = Get-Sha256Hex $path
        $exported += [ordered]@{
          name = $payload.name
          mediaType = $payload.mediaType
          guestPath = $path
          size = [int64]$payload.bytes.Length
          sha256 = $hash
          truncated = [bool]$payload.truncated
        }
      }
      $value = [ordered]@{
        schema = 'chemsema.gui.guest-artifact-export.v1'
        artifactId = [string]$request.artifactId
        artifacts = $exported
      }
    }
    return [ordered]@{
      schema = 'chemsema.gui.cdp-bridge.v1'
      status = 'passed'
      mode = [string]$request.mode
      value = $value
    }
  } finally {
    if (-not $persistent -and $null -ne $socket) { $socket.Dispose() }
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
        $envelope = Get-Content -Raw -Encoding UTF8 -LiteralPath $claimPath | ConvertFrom-Json
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
  if ($null -ne $script:PersistentCdpSocket) {
    $script:PersistentCdpSocket.Dispose()
    $script:PersistentCdpSocket = $null
  }
}

if (-not [string]::IsNullOrWhiteSpace($ChannelRoot)) {
  Start-CdpServer
} else {
  Write-CdpResult (Invoke-CdpRequest $RequestBase64)
}
