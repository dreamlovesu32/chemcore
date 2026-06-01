param(
  [Parameter(Mandatory = $true)]
  [string]$InputCdxml,

  [string]$OutputDir = "tmp/word-ole-roundtrip",

  [string]$CargoCommand = "cargo",

  [string]$OfficePackage = "chemcore-office",

  [string]$OfficeCommand = "",

  [int]$ShapeIndex = 1
)

$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.IO.Compression.FileSystem

$repoRoot = Split-Path -Parent $PSScriptRoot
$inputPath = (Resolve-Path $InputCdxml).Path
$outRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $OutputDir))
$stem = [System.IO.Path]::GetFileNameWithoutExtension($inputPath)
$sampleDir = Join-Path $outRoot $stem

New-Item -ItemType Directory -Force -Path $sampleDir | Out-Null

$payloadPath = Join-Path $sampleDir "$stem.payload.json"
$docxPath = Join-Path $sampleDir "$stem.generated.docx"
$roundtripDocxPath = Join-Path $sampleDir "$stem.roundtrip.docx"
$generatedPngPath = Join-Path $sampleDir "$stem.generated.wordcopy.png"
$roundtripPngPath = Join-Path $sampleDir "$stem.roundtrip.wordcopy.png"
$defaultOfficeExe = Join-Path $repoRoot "target\debug\chemcore-office.exe"

function Invoke-Checked {
  param(
    [string]$Command,
    [string[]]$Arguments
  )
  Write-Host "[RUN] $Command $($Arguments -join ' ')"
  & $Command @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "Command failed with exit code ${LASTEXITCODE}: $Command $($Arguments -join ' ')"
  }
}

function Get-DocxOleSummary {
  param([string]$Path)
  $zip = [System.IO.Compression.ZipFile]::OpenRead($Path)
  try {
    $entries = @($zip.Entries | ForEach-Object { $_.FullName })
    $embeddingEntries = @($entries | Where-Object { $_ -like "word/embeddings/*" })
    $emfEntries = @($entries | Where-Object { $_ -like "word/media/*.emf" })
    [pscustomobject]@{
      path = $Path
      entryCount = $entries.Count
      embeddingCount = $embeddingEntries.Count
      emfCount = $emfEntries.Count
      hasDocumentXml = $entries -contains "word/document.xml"
      hasRelationships = $entries -contains "word/_rels/document.xml.rels"
      embeddings = $embeddingEntries
      emfEntries = $emfEntries
    }
  }
  finally {
    $zip.Dispose()
  }
}

Invoke-Checked $CargoCommand @(
  "run", "-q",
  "-p", "chemcore-engine",
  "--example", "cdxml_to_clipboard_payload",
  "--",
  $inputPath,
  $payloadPath
)

if ($OfficeCommand) {
  Invoke-Checked $OfficeCommand @(
    "--write-word-docx-payload",
    $payloadPath,
    $docxPath
  )
} elseif (Test-Path $defaultOfficeExe) {
  Invoke-Checked $defaultOfficeExe @(
    "--write-word-docx-payload",
    $payloadPath,
    $docxPath
  )
} else {
  Invoke-Checked $CargoCommand @(
    "run", "-q",
    "-p", $OfficePackage,
    "--",
    "--write-word-docx-payload",
    $payloadPath,
    $docxPath
  )
}

$word = $null
$doc = $null

try {
  $word = New-Object -ComObject Word.Application
  $word.Visible = $false
  $word.DisplayAlerts = 0

  Write-Host "[WORD] open generated"
  $doc = $word.Documents.Open($docxPath)
  if ($doc.InlineShapes.Count -lt $ShapeIndex) {
    throw "Generated document only has $($doc.InlineShapes.Count) inline shape(s); requested index $ShapeIndex."
  }
  Write-Host "[WORD] save generated"
  $doc.Save() | Out-Null
  Write-Host "[WORD] close generated"
  $doc.Close([ref]0) | Out-Null
  $doc = $null

  Copy-Item -LiteralPath $docxPath -Destination $roundtripDocxPath -Force

  Write-Host "[WORD] open roundtrip"
  $doc = $word.Documents.Open($roundtripDocxPath)
  if ($doc.InlineShapes.Count -lt $ShapeIndex) {
    throw "Roundtrip document only has $($doc.InlineShapes.Count) inline shape(s); requested index $ShapeIndex."
  }
  Write-Host "[WORD] roundtrip inline shape ok"
}
finally {
  if ($doc -ne $null) {
    $doc.Close([ref]0) | Out-Null
  }
  if ($word -ne $null) {
    $word.Quit() | Out-Null
    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($word) | Out-Null
  }
}

& (Join-Path $PSScriptRoot "word-copy-inline-shape.ps1") -InputDocx $docxPath -OutputPng $generatedPngPath -ShapeIndex $ShapeIndex
if ($LASTEXITCODE -ne 0) {
  throw "Failed to export generated docx inline shape preview."
}
& (Join-Path $PSScriptRoot "word-copy-inline-shape.ps1") -InputDocx $roundtripDocxPath -OutputPng $roundtripPngPath -ShapeIndex $ShapeIndex
if ($LASTEXITCODE -ne 0) {
  throw "Failed to export roundtrip docx inline shape preview."
}

$summary = [pscustomobject]@{
  inputCdxml = $inputPath
  payload = $payloadPath
  generatedDocx = Get-DocxOleSummary -Path $docxPath
  roundtripDocx = Get-DocxOleSummary -Path $roundtripDocxPath
  generatedPng = $generatedPngPath
  roundtripPng = $roundtripPngPath
}

$summaryPath = Join-Path $sampleDir "$stem.summary.json"
$summary | ConvertTo-Json -Depth 6 | Set-Content -Encoding UTF8 $summaryPath
Write-Output $summaryPath
