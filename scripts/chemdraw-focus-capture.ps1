param(
  [Parameter(Mandatory = $true)]
  [string]$InputPath,

  [Parameter(Mandatory = $true)]
  [string]$OutputPath,

  [ValidateSet("Graphics", "Bonds", "Atoms", "Tables", "TLCPlates", "TLCSpots")]
  [string]$Collection = "Graphics",

  [int]$Index = 0,

  [int]$LaneIndex = 0,

  [int]$SpotIndex = 0,

  [ValidateSet("Selected", "Highlighted", "Click", "RightClick", "Drag")]
  [string]$Mode = "Selected",

  [int]$DelayMs = 500,

  [int]$DragDeltaY = -40
)

$ErrorActionPreference = "Stop"

Add-Type -TypeDefinition @"
using System;
using System.Drawing;
using System.Runtime.InteropServices;
using System.Windows.Forms;

public static class ChemDrawCaptureNative {
  [StructLayout(LayoutKind.Sequential)]
  public struct RECT {
    public int Left;
    public int Top;
    public int Right;
    public int Bottom;
  }

  [DllImport("user32.dll")]
  public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);

  [DllImport("user32.dll")]
  public static extern bool SetForegroundWindow(IntPtr hWnd);

  [DllImport("user32.dll")]
  public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);

  [DllImport("user32.dll")]
  public static extern bool SetCursorPos(int x, int y);

  [DllImport("user32.dll")]
  public static extern void mouse_event(uint dwFlags, uint dx, uint dy, uint dwData, UIntPtr dwExtraInfo);

  public const uint MOUSEEVENTF_LEFTDOWN = 0x0002;
  public const uint MOUSEEVENTF_LEFTUP = 0x0004;
  public const uint MOUSEEVENTF_RIGHTDOWN = 0x0008;
  public const uint MOUSEEVENTF_RIGHTUP = 0x0010;
  public const int SW_RESTORE = 9;

  public static void LeftClick(int x, int y) {
    SetCursorPos(x, y);
    mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, UIntPtr.Zero);
    mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, UIntPtr.Zero);
  }

  public static void RightClick(int x, int y) {
    SetCursorPos(x, y);
    mouse_event(MOUSEEVENTF_RIGHTDOWN, 0, 0, 0, UIntPtr.Zero);
    mouse_event(MOUSEEVENTF_RIGHTUP, 0, 0, 0, UIntPtr.Zero);
  }

  public static void LeftDown(int x, int y) {
    SetCursorPos(x, y);
    mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, UIntPtr.Zero);
  }

  public static void LeftUp(int x, int y) {
    SetCursorPos(x, y);
    mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, UIntPtr.Zero);
  }

  public static void CaptureWindow(IntPtr hWnd, string outputPath) {
    RECT rect;
    if (!GetWindowRect(hWnd, out rect)) {
      throw new InvalidOperationException("GetWindowRect failed.");
    }
    int width = rect.Right - rect.Left;
    int height = rect.Bottom - rect.Top;
    if (width <= 0 || height <= 0) {
      throw new InvalidOperationException("Window has invalid capture bounds.");
    }
    using (var bitmap = new Bitmap(width, height)) {
      using (var graphics = Graphics.FromImage(bitmap)) {
        graphics.CopyFromScreen(rect.Left, rect.Top, 0, 0, new Size(width, height));
      }
      bitmap.Save(outputPath);
    }
  }
}
"@ -ReferencedAssemblies System.Drawing, System.Windows.Forms

function Resolve-ExistingIndex {
  param(
    [string]$Name,
    [object[]]$Items,
    [int]$TargetIndex
  )
  if ($TargetIndex -lt 0 -or $TargetIndex -ge $Items.Count) {
    throw "$Name index $TargetIndex is out of range for count $($Items.Count)."
  }
  return $Items[$TargetIndex]
}

function Get-WindowProcess {
  param([datetime]$LaunchedAfter)
  Start-Sleep -Milliseconds 200
  $process = Get-Process ChemDraw -ErrorAction Stop |
    Where-Object { $_.MainWindowHandle -ne 0 -and $_.StartTime -ge $LaunchedAfter } |
    Sort-Object StartTime -Descending |
    Select-Object -First 1
  if (-not $process) {
    throw "ChemDraw main window was not found."
  }
  return $process
}

function Get-ObjectBounds {
  param([object]$Item)
  if ($null -ne $Item.Left -and $null -ne $Item.Top -and $null -ne $Item.Width -and $null -ne $Item.Height) {
    return [pscustomobject]@{
      Left = [double]$Item.Left
      Top = [double]$Item.Top
      Width = [double]$Item.Width
      Height = [double]$Item.Height
    }
  }
  if ($Item.Bounds) {
    $parts = "$($Item.Bounds)" -split "[ ,]+" | Where-Object { $_ -ne "" }
    if ($parts.Count -ge 4) {
      $left = [double]$parts[0]
      $top = [double]$parts[1]
      $right = [double]$parts[2]
      $bottom = [double]$parts[3]
      return [pscustomobject]@{
        Left = $left
        Top = $top
        Width = $right - $left
        Height = $bottom - $top
      }
    }
  }
  throw "Object does not expose usable bounds."
}

function Get-TlcSpotCenter {
  param(
    [object]$Plate,
    [int]$TargetLaneIndex,
    [int]$TargetSpotIndex
  )
  $lanes = @($Plate.Lanes)
  if (-not $lanes.Count) {
    throw "TLC plate has no lanes."
  }
  $lane = Resolve-ExistingIndex -Name "Lane" -Items $lanes -TargetIndex $TargetLaneIndex
  $spots = @($lane.Spots)
  if (-not $spots.Count) {
    throw "TLC lane has no spots."
  }
  $spot = Resolve-ExistingIndex -Name "Spot" -Items $spots -TargetIndex $TargetSpotIndex
  $originFraction = [double]$Plate.OriginFraction
  $frontFraction = [double]$Plate.SolventFrontFraction
  $rf = [double]$spot.Rf
  $plateBounds = Get-ObjectBounds -Item $Plate
  $laneCount = [double]$lanes.Count
  $usableHeight = $plateBounds.Height * (1.0 - $originFraction - $frontFraction)
  $originY = $plateBounds.Top + $plateBounds.Height * (1.0 - $originFraction)
  $centerX = $plateBounds.Left + $plateBounds.Width * (($TargetLaneIndex + 1.0) / ($laneCount + 1.0))
  $centerY = $originY - ($rf * $usableHeight)
  return [pscustomobject]@{
    CenterX = $centerX
    CenterY = $centerY
  }
}

function Get-BlueSelectionBounds {
  param(
    [string]$ImagePath,
    [int]$MinSearchX = 240,
    [int]$MinSearchY = 120
  )
  Add-Type -AssemblyName System.Drawing
  $bitmap = [System.Drawing.Bitmap]::FromFile($ImagePath)
  try {
    $minX = [int]::MaxValue
    $minY = [int]::MaxValue
    $maxX = -1
    $maxY = -1
    for ($y = $MinSearchY; $y -lt $bitmap.Height; $y++) {
      for ($x = $MinSearchX; $x -lt $bitmap.Width; $x++) {
        $color = $bitmap.GetPixel($x, $y)
        if ($color.B -gt 180 -and $color.G -gt 140 -and $color.R -lt 170) {
          if ($x -lt $minX) { $minX = $x }
          if ($y -lt $minY) { $minY = $y }
          if ($x -gt $maxX) { $maxX = $x }
          if ($y -gt $maxY) { $maxY = $y }
        }
      }
    }
    if ($maxX -lt $minX -or $maxY -lt $minY) {
      throw "Failed to detect blue selection bounds in $ImagePath."
    }
    return [pscustomobject]@{
      Left = $minX
      Top = $minY
      Right = $maxX
      Bottom = $maxY
      Width = $maxX - $minX
      Height = $maxY - $minY
    }
  }
  finally {
    $bitmap.Dispose()
  }
}

function Get-TlcSpotScreenPoint {
  param(
    [IntPtr]$WindowHandle,
    [object]$Plate,
    [int]$TargetLaneIndex,
    [int]$TargetSpotIndex
  )
  $plate.Selected = $true
  Start-Sleep -Milliseconds 200
  $tempPath = Join-Path $env:TEMP ("chemcore-chemdraw-plate-select-" + [guid]::NewGuid().ToString("N") + ".png")
  try {
    [ChemDrawCaptureNative]::CaptureWindow($WindowHandle, $tempPath)
    $selectionBounds = Get-BlueSelectionBounds -ImagePath $tempPath
  }
  finally {
    Remove-Item -LiteralPath $tempPath -ErrorAction SilentlyContinue
  }
  $plate.Selected = $false
  Start-Sleep -Milliseconds 100

  $docPoint = Get-TlcSpotCenter -Plate $Plate -TargetLaneIndex $TargetLaneIndex -TargetSpotIndex $TargetSpotIndex
  $plateBounds = Get-ObjectBounds -Item $Plate
  $relativeX = ($docPoint.CenterX - $plateBounds.Left) / $plateBounds.Width
  $relativeY = ($docPoint.CenterY - $plateBounds.Top) / $plateBounds.Height
  return [pscustomobject]@{
    ScreenX = [int][Math]::Round($selectionBounds.Left + ($selectionBounds.Width * $relativeX))
    ScreenY = [int][Math]::Round($selectionBounds.Top + ($selectionBounds.Height * $relativeY))
  }
}

function Invoke-Interaction {
  param(
    [IntPtr]$WindowHandle,
    [string]$TargetMode,
    [string]$TargetCollection,
    [object]$TargetItem,
    [int]$TargetLaneIndex,
    [int]$TargetSpotIndex,
    [int]$TargetDragDeltaY
  )

  [ChemDrawCaptureNative]::ShowWindow($WindowHandle, [ChemDrawCaptureNative]::SW_RESTORE) | Out-Null
  [ChemDrawCaptureNative]::SetForegroundWindow($WindowHandle) | Out-Null
  Start-Sleep -Milliseconds 200

  switch ($TargetMode) {
    "Selected" {
      if ($TargetItem.PSObject.Properties.Name -contains "Selected") {
        $TargetItem.Selected = $true
        return $null
      }
      throw "$TargetCollection does not expose a writable Selected property."
    }
    "Highlighted" {
      if ($TargetItem.PSObject.Properties.Name -contains "Highlighted") {
        $TargetItem.Highlighted = $true
        return $null
      }
      throw "$TargetCollection does not expose a writable Highlighted property."
    }
    "Click" {
      if ($TargetCollection -eq "TLCSpots") {
        $point = Get-TlcSpotScreenPoint -WindowHandle $WindowHandle -Plate $TargetItem -TargetLaneIndex $TargetLaneIndex -TargetSpotIndex $TargetSpotIndex
        [ChemDrawCaptureNative]::LeftClick($point.ScreenX, $point.ScreenY)
        return $null
      }
      $bounds = Get-ObjectBounds -Item $TargetItem
      $x = [int][Math]::Round($bounds.Left + ($bounds.Width / 2.0))
      $y = [int][Math]::Round($bounds.Top + ($bounds.Height / 2.0))
      [ChemDrawCaptureNative]::LeftClick($x, $y)
      return $null
    }
    "RightClick" {
      if ($TargetCollection -eq "TLCSpots") {
        $point = Get-TlcSpotScreenPoint -WindowHandle $WindowHandle -Plate $TargetItem -TargetLaneIndex $TargetLaneIndex -TargetSpotIndex $TargetSpotIndex
        [ChemDrawCaptureNative]::RightClick($point.ScreenX, $point.ScreenY)
        return $null
      }
      $bounds = Get-ObjectBounds -Item $TargetItem
      $x = [int][Math]::Round($bounds.Left + ($bounds.Width / 2.0))
      $y = [int][Math]::Round($bounds.Top + ($bounds.Height / 2.0))
      [ChemDrawCaptureNative]::RightClick($x, $y)
      return $null
    }
    "Drag" {
      if ($TargetCollection -eq "TLCSpots") {
        $point = Get-TlcSpotScreenPoint -WindowHandle $WindowHandle -Plate $TargetItem -TargetLaneIndex $TargetLaneIndex -TargetSpotIndex $TargetSpotIndex
        [ChemDrawCaptureNative]::LeftDown($point.ScreenX, $point.ScreenY)
        Start-Sleep -Milliseconds 120
        $releaseY = $point.ScreenY + $TargetDragDeltaY
        [ChemDrawCaptureNative]::SetCursorPos($point.ScreenX, $releaseY) | Out-Null
        Start-Sleep -Milliseconds 180
        return [pscustomobject]@{ ReleaseX = $point.ScreenX; ReleaseY = $releaseY }
      }
      $bounds = Get-ObjectBounds -Item $TargetItem
      $x = [int][Math]::Round($bounds.Left + ($bounds.Width / 2.0))
      $y = [int][Math]::Round($bounds.Top + ($bounds.Height / 2.0))
      [ChemDrawCaptureNative]::LeftDown($x, $y)
      Start-Sleep -Milliseconds 120
      $releaseY = $y + $TargetDragDeltaY
      [ChemDrawCaptureNative]::SetCursorPos($x, $releaseY) | Out-Null
      Start-Sleep -Milliseconds 180
      return [pscustomobject]@{ ReleaseX = $x; ReleaseY = $releaseY }
    }
  }
}

$resolvedInputPath = (Resolve-Path -LiteralPath $InputPath).Path
$resolvedOutputPath = $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($OutputPath)
$outputDir = Split-Path -Parent $resolvedOutputPath
if ($outputDir) {
  New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
}

$app = $null
$doc = $null
$process = $null
$launchBaseline = Get-Date
try {
  $app = New-Object -ComObject ChemDraw.Application
  $app.Visible = $true
  $doc = $app.Documents.Open($resolvedInputPath)
  $doc.Activate() | Out-Null
  Start-Sleep -Milliseconds 300
  $process = Get-WindowProcess -LaunchedAfter $launchBaseline

  switch ($Collection) {
    "Graphics" { $target = Resolve-ExistingIndex -Name "Graphic" -Items @($doc.Graphics) -TargetIndex $Index }
    "Bonds" { $target = Resolve-ExistingIndex -Name "Bond" -Items @($doc.Bonds) -TargetIndex $Index }
    "Atoms" { $target = Resolve-ExistingIndex -Name "Atom" -Items @($doc.Atoms) -TargetIndex $Index }
    "Tables" { $target = Resolve-ExistingIndex -Name "Table" -Items @($doc.Tables) -TargetIndex $Index }
    "TLCPlates" { $target = Resolve-ExistingIndex -Name "TLC plate" -Items @($doc.TLCPlates) -TargetIndex $Index }
    "TLCSpots" { $target = Resolve-ExistingIndex -Name "TLC plate" -Items @($doc.TLCPlates) -TargetIndex $Index }
    default { throw "Unsupported collection $Collection" }
  }

  $windowHandle = [IntPtr]$process.MainWindowHandle
  $interactionState = Invoke-Interaction -WindowHandle $windowHandle -TargetMode $Mode -TargetCollection $Collection -TargetItem $target -TargetLaneIndex $LaneIndex -TargetSpotIndex $SpotIndex -TargetDragDeltaY $DragDeltaY
  Start-Sleep -Milliseconds $DelayMs
  [ChemDrawCaptureNative]::CaptureWindow($windowHandle, $resolvedOutputPath)
  if ($Mode -eq "Drag" -and $interactionState) {
    [ChemDrawCaptureNative]::LeftUp([int]$interactionState.ReleaseX, [int]$interactionState.ReleaseY)
  }
  Write-Host "Saved $resolvedOutputPath"
}
finally {
  if ($doc) {
    try { $doc.Close() | Out-Null } catch {}
    try { [void][System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($doc) } catch {}
  }
  if ($app) {
    try { $app.Quit() | Out-Null } catch {}
    try { [void][System.Runtime.InteropServices.Marshal]::FinalReleaseComObject($app) } catch {}
  }
  if ($process) {
    Start-Sleep -Milliseconds 200
    try {
      $stillRunning = Get-Process -Id $process.Id -ErrorAction SilentlyContinue
      if ($stillRunning) {
        $stillRunning | Stop-Process -Force -ErrorAction SilentlyContinue
      }
    } catch {}
  }
  [GC]::Collect()
  [GC]::WaitForPendingFinalizers()
}
