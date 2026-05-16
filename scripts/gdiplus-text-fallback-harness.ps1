param(
    [Parameter(Mandatory = $true)]
    [string]$Scenario,
    [string]$OutEmf = "tmp/gdiplus-harness.emf",
    [ValidateSet("rect", "point", "driver")]
    [string]$Mode = "rect"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing

if (-not ("Harness.GdiPlusNative" -as [type])) {
    Add-Type -ReferencedAssemblies System.Drawing -TypeDefinition @"
using System;
using System.Drawing;
using System.Reflection;
using System.Runtime.InteropServices;

namespace Harness {
    public static class GdiPlusNative {
        [DllImport("gdiplus.dll", ExactSpelling = true)]
        public static extern int GdipDrawDriverString(
            IntPtr graphics,
            ushort[] text,
            int length,
            IntPtr font,
            IntPtr brush,
            PointF[] positions,
            int flags,
            IntPtr matrix
        );

        public static IntPtr NativeGraphics(Graphics graphics) {
            return (IntPtr)typeof(Graphics).GetField("nativeGraphics", BindingFlags.Instance | BindingFlags.NonPublic).GetValue(graphics);
        }

        public static IntPtr NativeFont(Font font) {
            return (IntPtr)typeof(Font).GetField("nativeFont", BindingFlags.Instance | BindingFlags.NonPublic).GetValue(font);
        }

        public static IntPtr NativeBrush(Brush brush) {
            return (IntPtr)typeof(Brush).GetField("nativeBrush", BindingFlags.Instance | BindingFlags.NonPublic).GetValue(brush);
        }
    }
}
"@
}

function New-Run {
    param(
        [string]$Text,
        [float]$X,
        [float]$Y,
        [float]$Width,
        [float]$Height,
        [float]$FontSize,
        [bool]$Bold = $true,
        [string]$Color = "Black"
    )
    [pscustomobject]@{
        Text = $Text
        X = $X
        Y = $Y
        Width = $Width
        Height = $Height
        FontSize = $FontSize
        Bold = $Bold
        Color = $Color
    }
}

function Get-ScenarioRuns {
    param([string]$Name)

    $titleSecondLine = @(
        (New-Run "Cu(MeCN)" 2030.5767 1210.4937 869.1265 144.8788 27.0 $true),
        (New-Run "4"         2513.4248 1253.9574  75.0178 108.6591 20.0 $true),
        (New-Run "PF"        2555.1013 1210.4937 229.8175 144.8788 27.0 $true),
        (New-Run "6"         2682.7778 1253.9574  75.0178 108.6591 20.0 $true),
        (New-Run " "         2724.4543 1210.4937  49.9680 144.8788 27.0 $true),
        (New-Run "(5 "       2752.2144 1210.4937 209.8830 144.8788 27.0 $true),
        (New-Run "mol%), "   2868.8162 1210.4937 639.4846 144.8788 27.0 $true),
        (New-Run "L "        3224.0854 1210.4937 156.5780 144.8788 27.0 $true),
        (New-Run "(7 "       3311.0732 1210.4937 209.8830 144.8788 27.0 $true),
        (New-Run "mol%)"     3427.6741 1210.4937 540.6427 144.8788 27.0 $true)
    )

    switch ($Name) {
        "rect-fresh" {
            return $titleSecondLine
        }
        "rect-with-title-line1" {
            return @(
                (New-Run "4DPAIPN " 2444.1101 1093.1191 816.1727 144.8788 27.0 $true),
                (New-Run "(2 "      2897.5396 1093.1191 209.8830 144.8788 27.0 $true),
                (New-Run "mol%)"    3014.1414 1093.1191 539.5486 144.8788 27.0 $true)
            ) + $titleSecondLine
        }
        "rect-with-l-label" {
            return @(
                (New-Run "L:" 614.2654 2144.9202 169.7506 144.8788 27.0 $true)
            ) + $titleSecondLine
        }
        "rect-with-ph-and-l" {
            return @(
                (New-Run "Ph" 4778.9487 1418.9058 229.8175 144.8788 27.0 $true $false),
                (New-Run "L:"  614.2654 2144.9202 169.7506 144.8788 27.0 $true)
            ) + $titleSecondLine
        }
        default {
            throw "Unknown scenario: $Name"
        }
    }
}

function Get-RunBaselineY {
    param(
        [System.Drawing.Font]$Font,
        [float]$TopY
    )
    $family = $Font.FontFamily
    $style = $Font.Style
    $ascent = $family.GetCellAscent($style)
    $em = $family.GetEmHeight($style)
    if ($em -le 0) {
        return $TopY
    }
    return $TopY + ($Font.Size * ($ascent / [float]$em))
}

function Measure-PrefixWidth {
    param(
        [System.Drawing.Graphics]$Graphics,
        [System.Drawing.Font]$Font,
        [System.Drawing.StringFormat]$Format,
        [string]$Text
    )
    if ([string]::IsNullOrEmpty($Text)) {
        return 0.0
    }
    $size = $Graphics.MeasureString($Text, $Font, [System.Int32]::MaxValue, $Format)
    return [float]$size.Width
}

function Draw-DriverStringRun {
    param(
        [System.Drawing.Graphics]$Graphics,
        [pscustomobject]$Run,
        [System.Drawing.Font]$Font,
        [System.Drawing.Brush]$Brush,
        [System.Drawing.StringFormat]$Format
    )
    $chars = $Run.Text.ToCharArray()
    $positions = New-Object System.Drawing.PointF[] ($chars.Length)
    $baselineY = Get-RunBaselineY -Font $Font -TopY ([float]$Run.Y)
    for ($i = 0; $i -lt $chars.Length; $i++) {
        $prefix = if ($i -eq 0) { "" } else { $Run.Text.Substring(0, $i) }
        $x = [float]$Run.X + (Measure-PrefixWidth -Graphics $Graphics -Font $Font -Format $Format -Text $prefix)
        $positions[$i] = [System.Drawing.PointF]::new($x, $baselineY)
    }
    $utf16 = [System.Text.Encoding]::Unicode.GetBytes($Run.Text)
    $codeUnits = New-Object 'System.UInt16[]' ($utf16.Length / 2)
    [System.Buffer]::BlockCopy($utf16, 0, $codeUnits, 0, $utf16.Length)
    $status = [Harness.GdiPlusNative]::GdipDrawDriverString(
        [Harness.GdiPlusNative]::NativeGraphics($Graphics),
        $codeUnits,
        $codeUnits.Length,
        [Harness.GdiPlusNative]::NativeFont($Font),
        [Harness.GdiPlusNative]::NativeBrush($Brush),
        $positions,
        0,
        [System.IntPtr]::Zero
    )
    if ($status -ne 0) {
        throw "GdipDrawDriverString failed: $status"
    }
}

$runs = Get-ScenarioRuns $Scenario

$directory = Split-Path -Parent $OutEmf
if ($directory) {
    New-Item -ItemType Directory -Force -Path $directory | Out-Null
}

$bitmap = New-Object System.Drawing.Bitmap 10, 10
$refGraphics = [System.Drawing.Graphics]::FromImage($bitmap)
$hdc = $refGraphics.GetHdc()
$frame = New-Object System.Drawing.RectangleF(0, 0, 2000, 1000)
$metafile = New-Object System.Drawing.Imaging.Metafile(
    $OutEmf,
    $hdc,
    $frame,
    [System.Drawing.Imaging.MetafileFrameUnit]::Pixel,
    [System.Drawing.Imaging.EmfType]::EmfPlusDual
)
$refGraphics.ReleaseHdc($hdc)
$refGraphics.Dispose()
$bitmap.Dispose()

$graphics = [System.Drawing.Graphics]::FromImage($metafile)
$graphics.PageUnit = [System.Drawing.GraphicsUnit]::Pixel
$graphics.PageScale = 1.0
$graphics.PageScale = 0.26666668
$graphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAlias
$graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias

$format = [System.Drawing.StringFormat]::GenericTypographic.Clone()
$format.FormatFlags = 0x6804

$fonts = @{}

try {
    foreach ($run in $runs) {
        $key = "{0}:{1}" -f $run.FontSize, $run.Bold
        if (-not $fonts.ContainsKey($key)) {
            $style = if ($run.Bold) {
                [System.Drawing.FontStyle]::Bold
            } else {
                [System.Drawing.FontStyle]::Regular
            }
            $fonts[$key] = New-Object System.Drawing.Font(
                "Arial",
                $run.FontSize,
                $style,
                [System.Drawing.GraphicsUnit]::Pixel
            )
        }
        $font = $fonts[$key]
        $brush = switch ($run.Color) {
            "Orange" { [System.Drawing.Brushes]::Orange }
            default { [System.Drawing.Brushes]::Black }
        }
        if ($Mode -eq "driver") {
            Draw-DriverStringRun -Graphics $graphics -Run $run -Font $font -Brush $brush -Format $format
        }
        elseif ($Mode -eq "point") {
            $point = [System.Drawing.PointF]::new($run.X, $run.Y)
            $graphics.DrawString($run.Text, $font, $brush, $point, $format)
        }
        else {
            $rect = [System.Drawing.RectangleF]::new($run.X, $run.Y, $run.Width, $run.Height)
            $graphics.DrawString($run.Text, $font, $brush, $rect, $format)
        }
    }
}
finally {
    foreach ($font in $fonts.Values) {
        $font.Dispose()
    }
    $format.Dispose()
    $graphics.Dispose()
    $metafile.Dispose()
}

Write-Output $OutEmf
