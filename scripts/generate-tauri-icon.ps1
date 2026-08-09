param(
  [string]$OutputPath = "src-tauri/icons/icon.ico"
)

Add-Type -AssemblyName System.Drawing

$outputFullPath = [System.IO.Path]::GetFullPath((Join-Path (Get-Location) $OutputPath))
$outputDirectory = Split-Path -Parent $outputFullPath
[System.IO.Directory]::CreateDirectory($outputDirectory) | Out-Null

$size = 256
$bitmap = New-Object System.Drawing.Bitmap($size, $size)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$graphics.Clear([System.Drawing.Color]::FromArgb(16, 18, 24))

$bounds = New-Object System.Drawing.Rectangle(12, 12, 232, 232)
$gradient = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
  $bounds,
  [System.Drawing.Color]::FromArgb(168, 124, 255),
  [System.Drawing.Color]::FromArgb(103, 202, 255),
  35
)
$graphics.FillEllipse($gradient, $bounds)

$innerBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(26, 29, 38))
$graphics.FillEllipse($innerBrush, (New-Object System.Drawing.Rectangle(38, 38, 180, 180)))

$starBrush = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
$points = @(
  (New-Object System.Drawing.PointF(128, 58)),
  (New-Object System.Drawing.PointF(145, 108)),
  (New-Object System.Drawing.PointF(198, 108)),
  (New-Object System.Drawing.PointF(155, 139)),
  (New-Object System.Drawing.PointF(172, 190)),
  (New-Object System.Drawing.PointF(128, 159)),
  (New-Object System.Drawing.PointF(84, 190)),
  (New-Object System.Drawing.PointF(101, 139)),
  (New-Object System.Drawing.PointF(58, 108)),
  (New-Object System.Drawing.PointF(111, 108))
)
$graphics.FillPolygon($starBrush, $points)

$pngStream = New-Object System.IO.MemoryStream
$bitmap.Save($pngStream, [System.Drawing.Imaging.ImageFormat]::Png)
$pngBytes = $pngStream.ToArray()

$fileStream = [System.IO.File]::Open($outputFullPath, [System.IO.FileMode]::Create)
$writer = New-Object System.IO.BinaryWriter($fileStream)
try {
  $writer.Write([uint16]0)
  $writer.Write([uint16]1)
  $writer.Write([uint16]1)
  $writer.Write([byte]0)
  $writer.Write([byte]0)
  $writer.Write([byte]0)
  $writer.Write([byte]0)
  $writer.Write([uint16]1)
  $writer.Write([uint16]32)
  $writer.Write([uint32]$pngBytes.Length)
  $writer.Write([uint32]22)
  $writer.Write($pngBytes)
} finally {
  $writer.Dispose()
  $pngStream.Dispose()
  $starBrush.Dispose()
  $innerBrush.Dispose()
  $gradient.Dispose()
  $graphics.Dispose()
  $bitmap.Dispose()
}

Write-Output "Generated $outputFullPath"
