param(
  [Parameter(Mandatory = $true)]
  [string]$InputDocx,

  [Parameter(Mandatory = $true)]
  [string]$OutputPng,

  [int]$ShapeIndex = 1,

  [int]$OpenDelayMs = 800,

  [int]$SelectDelayMs = 250,

  [int]$ClipboardDelayMs = 800
)

$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing

$inputPath = (Resolve-Path $InputDocx).Path
$outputPath = [System.IO.Path]::GetFullPath($OutputPng)
$outputDir = Split-Path -Parent $outputPath
if (-not (Test-Path $outputDir)) {
  New-Item -ItemType Directory -Force -Path $outputDir | Out-Null
}

$word = $null
$doc = $null

try {
  $word = New-Object -ComObject Word.Application
  $word.Visible = $false
  $word.DisplayAlerts = 0

  $doc = $word.Documents.Open($inputPath)
  Start-Sleep -Milliseconds $OpenDelayMs

  if ($doc.InlineShapes.Count -lt $ShapeIndex) {
    throw "Document only has $($doc.InlineShapes.Count) inline shape(s); requested index $ShapeIndex."
  }

  $doc.InlineShapes.Item($ShapeIndex).Select()
  Start-Sleep -Milliseconds $SelectDelayMs

  $word.Selection.CopyAsPicture()
  Start-Sleep -Milliseconds $ClipboardDelayMs

  $image = [System.Windows.Forms.Clipboard]::GetImage()
  if ($null -eq $image) {
    throw "Clipboard image is null after CopyAsPicture."
  }

  $image.Save($outputPath, [System.Drawing.Imaging.ImageFormat]::Png)
  Write-Output "saved $outputPath"
}
finally {
  if ($doc -ne $null) {
    $doc.Close([ref]0) | Out-Null
  }
  if ($word -ne $null) {
    $word.Quit() | Out-Null
    [System.Runtime.Interopservices.Marshal]::ReleaseComObject($word) | Out-Null
  }
}
