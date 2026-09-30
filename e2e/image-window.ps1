# For the Find image tests: draws a fixed pattern, saves it as a PNG, and
# (unless -SaveOnly) shows it in a small borderless window at X, Y (physical
# pixels) for Seconds. It only shows something: it sends no input.
param([string]$Png, [int]$X = 120, [int]$Y = 120, [int]$Seconds = 30, [switch]$SaveOnly)

Add-Type -AssemblyName System.Windows.Forms, System.Drawing
# Physical pixels, so the picture on screen is the saved one, 1:1.
Add-Type -Namespace E2E -Name Dpi -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetProcessDPIAware();'
[void][E2E.Dpi]::SetProcessDPIAware()

$bmp = New-Object System.Drawing.Bitmap 120, 60
$g = [System.Drawing.Graphics]::FromImage($bmp)
$rng = New-Object System.Random 7
for ($i = 0; $i -lt 24; $i++) {
    $color = [System.Drawing.Color]::FromArgb($rng.Next(256), $rng.Next(256), $rng.Next(256))
    $g.FillRectangle((New-Object System.Drawing.SolidBrush $color), ($i % 8) * 15, [math]::Floor($i / 8) * 20, 15, 20)
}
$g.Dispose()
$bmp.Save($Png, [System.Drawing.Imaging.ImageFormat]::Png)
if ($SaveOnly) { exit }

$form = New-Object System.Windows.Forms.Form
$form.FormBorderStyle = "None"
$form.StartPosition = "Manual"
$form.TopMost = $true
$form.ShowInTaskbar = $false
$form.Location = New-Object System.Drawing.Point $X, $Y
$form.ClientSize = New-Object System.Drawing.Size 120, 60
$form.BackgroundImage = $bmp
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = $Seconds * 1000
$timer.add_Tick({ $form.Close() })
$timer.Start()
[System.Windows.Forms.Application]::Run($form)
