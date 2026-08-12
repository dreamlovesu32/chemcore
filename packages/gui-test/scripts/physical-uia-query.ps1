param(
  [Parameter(Mandatory = $true)][int]$TargetProcessId,
  [string]$ExactName,
  [string]$ExactAutomationId,
  [string]$ExpectedControlType,
  [string]$ScopeName
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = [Text.UTF8Encoding]::new($false)
Add-Type -AssemblyName UIAutomationClient
Add-Type -AssemblyName UIAutomationTypes

if ([string]::IsNullOrWhiteSpace($ExactName) -and [string]::IsNullOrWhiteSpace($ExactAutomationId)) {
  throw 'UI Automation query requires an exact accessible name or automation id.'
}

$process = Get-Process -Id $TargetProcessId -ErrorAction Stop
if ($process.SessionId -eq 0) { throw 'Candidate is not in an interactive session.' }
$processCondition = [Windows.Automation.PropertyCondition]::new(
  [Windows.Automation.AutomationElement]::ProcessIdProperty,
  $TargetProcessId
)
$roots = @(
  [Windows.Automation.AutomationElement]::RootElement.FindAll(
    [Windows.Automation.TreeScope]::Children,
    $processCondition
  ) | Where-Object {
    -not $_.Current.IsOffscreen -and
    $_.Current.BoundingRectangle.Width -gt 0 -and
    $_.Current.BoundingRectangle.Height -gt 0
  }
)
if ($roots.Count -eq 0) { throw 'Candidate top-level UI Automation element is absent.' }

$topLevels = @($roots | ForEach-Object {
  $rect = $_.Current.BoundingRectangle
  [ordered]@{
    name = $_.Current.Name
    automationId = $_.Current.AutomationId
    className = $_.Current.ClassName
    offscreen = $_.Current.IsOffscreen
    rect = @(
      [int][Math]::Round($rect.Left),
      [int][Math]::Round($rect.Top),
      [int][Math]::Round($rect.Right),
      [int][Math]::Round($rect.Bottom)
    )
  }
})

$scopeCondition = if ([string]::IsNullOrWhiteSpace($ScopeName)) {
  $null
} else {
  [Windows.Automation.PropertyCondition]::new(
    [Windows.Automation.AutomationElement]::NameProperty,
    $ScopeName
  )
}
$conditions = @()
if (-not [string]::IsNullOrWhiteSpace($ExactName) -and $ExactName -ne '*') {
  $conditions += [Windows.Automation.PropertyCondition]::new(
    [Windows.Automation.AutomationElement]::NameProperty,
    $ExactName
  )
}
if (-not [string]::IsNullOrWhiteSpace($ExactAutomationId)) {
  $conditions += [Windows.Automation.PropertyCondition]::new(
    [Windows.Automation.AutomationElement]::AutomationIdProperty,
    $ExactAutomationId
  )
}
$queryCondition = if ($conditions.Count -eq 0) {
  [Windows.Automation.Condition]::TrueCondition
} elseif ($conditions.Count -eq 1) {
  $conditions[0]
} else {
  [Windows.Automation.AndCondition]::new([Windows.Automation.Condition[]]$conditions)
}

$matches = @()
foreach ($root in $roots) {
  $searchRoots = if ($null -eq $scopeCondition) {
    @($root)
  } else {
    @($root.FindAll([Windows.Automation.TreeScope]::Descendants, $scopeCondition))
  }
  foreach ($searchRoot in $searchRoots) {
    foreach ($element in $searchRoot.FindAll([Windows.Automation.TreeScope]::Descendants, $queryCondition)) {
      if ($matches.Count -ge 200) { break }
      $rect = $element.Current.BoundingRectangle
      $coordinates = @($rect.Left, $rect.Top, $rect.Right, $rect.Bottom)
      if ($coordinates | Where-Object { [double]::IsNaN($_) -or [double]::IsInfinity($_) }) { continue }
      if ($rect.Width -le 0 -or $rect.Height -le 0) { continue }
      if (-not [string]::IsNullOrWhiteSpace($ExpectedControlType) -and
          $element.Current.ControlType.ProgrammaticName -ne $ExpectedControlType) { continue }
      $matches += [ordered]@{
        name = $element.Current.Name
        automationId = $element.Current.AutomationId
        className = $element.Current.ClassName
        controlType = $element.Current.ControlType.ProgrammaticName
        enabled = $element.Current.IsEnabled
        offscreen = $element.Current.IsOffscreen
        hasKeyboardFocus = $element.Current.HasKeyboardFocus
        topLevelName = $root.Current.Name
        topLevelClassName = $root.Current.ClassName
        rect = @(
          [int][Math]::Round($rect.Left),
          [int][Math]::Round($rect.Top),
          [int][Math]::Round($rect.Right),
          [int][Math]::Round($rect.Bottom)
        )
      }
    }
  }
}

[ordered]@{
  schema = 'chemsema.gui.uia-query.v1'
  processId = $TargetProcessId
  name = $ExactName
  automationId = $ExactAutomationId
  controlType = $ExpectedControlType
  topLevels = $topLevels
  matches = $matches
} | ConvertTo-Json -Depth 6 -Compress
