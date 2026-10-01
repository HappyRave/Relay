# Regenerates the documentation screenshots in docs\images from the real app.
#
# Needs a release build (npx tauri build) and Node. Runs Relay with a throwaway
# data folder, which is seeded with the design's sample macros, and drives it
# through scripts\cdp.mjs. Don't touch the mouse or keyboard while it runs.
#
# Usage: powershell -ExecutionPolicy Bypass -File scripts\docs-screenshots.ps1
$root = Split-Path $PSScriptRoot -Parent
$s = Join-Path $root "target\docs-shots"
$outDir = Join-Path $root "docs\images"
$data = Join-Path $s "data"
if (Test-Path $s) { Remove-Item -Recurse -Force $s -Confirm:$false }
New-Item -ItemType Directory -Force $s, $data, $outDir | Out-Null
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public class D1 { [StructLayout(LayoutKind.Sequential)] public struct R { public int L, T, Ri, B; }
  public struct P { public int X, Y; }
  [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int a, out R r, int s);
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string c, string t);
  [DllImport("user32.dll")] public static extern bool ClientToScreen(IntPtr h, ref P p);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h); }
"@
function Js($code) { $code | Out-File -Encoding utf8 "$s\snippet.js"; node (Join-Path $PSScriptRoot "cdp.mjs") 9333 "$s\snippet.js" }
# The tray icon also owns a window titled "Relay"; the widget's class is "Tauri Window".
function Hwnd() { [D1]::FindWindow("Tauri Window", "Relay") }
function Shot($name, $h = (Hwnd)) {
  # An error from an earlier step would stay on screen.
  Js 'if (window.__relay.toast) window.__relay.dismissToast(); return true' | Out-Null
  Start-Sleep -Milliseconds 350
  # The visible frame (DWMWA_EXTENDED_FRAME_BOUNDS), without the invisible borders.
  $r = New-Object D1+R; [D1]::DwmGetWindowAttribute($h, 9, [ref]$r, 16) | Out-Null
  $bmp = New-Object System.Drawing.Bitmap ($r.Ri - $r.L), ($r.B - $r.T)
  $g = [System.Drawing.Graphics]::FromImage($bmp); $g.CopyFromScreen($r.L, $r.T, 0, 0, $bmp.Size)
  $bmp.Save((Join-Path $outDir $name), [System.Drawing.Imaging.ImageFormat]::Png); $g.Dispose(); $bmp.Dispose()
  "saved $name"
}
function Hover($selector) {
  $pos = (Js "const r = document.querySelector('$selector').getBoundingClientRect(); return [Math.round(r.x + r.width / 2), Math.round(r.y + r.height / 2)]") | ConvertFrom-Json
  $o = New-Object D1+P; [D1]::ClientToScreen((Hwnd), [ref]$o) | Out-Null
  [D1]::SetCursorPos($o.X + $pos[0], $o.Y + $pos[1]) | Out-Null
}
function Tab($name) { Js "[...document.querySelectorAll('.tabs button')].find(b => b.textContent.includes('$name')).click(); return true" | Out-Null }
function Park() { [D1]::SetCursorPos(20, 20) | Out-Null }

# The run history, from the browser preview's fixture, dated back from now.
$runs = Get-Content -Raw (Join-Path $root "src\lib\dev\sample-runs.json") | ConvertFrom-Json
$now = [DateTime]::UtcNow
$entries = @($runs | Sort-Object { -$_.ago_ms } | ForEach-Object { $_.entry.at = $now.AddMilliseconds(-$_.ago_ms).ToString("yyyy-MM-ddTHH:mm:ssZ"); $_.entry })
@{ version = 1; entries = $entries } | ConvertTo-Json -Depth 10 | Out-File -Encoding ascii (Join-Path $data "runs.json")

$env:RELAY_DATA_DIR = $data
$env:RELAY_DEVTOOLS_PORT = "9333"
$relay = Start-Process (Join-Path $root "target\release\relay.exe") -PassThru
Start-Sleep -Seconds 7
[D1]::SetForegroundWindow((Hwnd)) | Out-Null
Park

# 1. The editor mid-way through the invoice macro.
Js 'window.__relay.seek(4200); return true' | Out-Null
Shot "editor.png"

# 2. Playing, with the loop badge and the red trail. The samples were recorded
# on a 1920x1080 desktop and would really click there, so this plays only
# inside the longest pause (stretched to 5 s, then undone), where nothing happens.
Js 'const r = window.__relay; const i = r.steps.reduce((b, s, j) => (s.pause > r.steps[b].pause ? j : b), 0); await r.setPause(i, 5000); await r.setPlayback({ repeat: { count: 3 } }); const s = r.steps[i]; r.seek(s.t - s.pause + 200); await r.togglePlay(); return i' | Out-Null
Start-Sleep -Milliseconds 1200
Shot "playing.png"
Js 'const r = window.__relay; await r.stop(); while (r.mode !== "idle") await new Promise(res => setTimeout(res, 50)); await r.undo(); r.seek(0); return r.duration' | Out-Null

# 3. The step editor on the pixel check.
Js 'const i = window.__relay.steps.findIndex(s => s.kind === "pixel_wait"); document.querySelectorAll(".list .row")[i].click(); await new Promise(r => setTimeout(r, 400)); return i' | Out-Null
Park
Shot "step-editor.png"
Js 'const i = window.__relay.steps.findIndex(s => s.kind === "pixel_wait"); document.querySelectorAll(".list .row")[i].click(); window.__relay.seek(0); return true' | Out-Null

# 3b. The step editor on a Find image step, whose image is the header's Export
# button cut out of editor.png (inserted after the first click, then undone).
$b = (Js "const b = [...document.querySelectorAll('.header button')].find(b => b.textContent.trim() === 'Export').getBoundingClientRect(); const k = devicePixelRatio; return [b.x, b.y, b.width, b.height].map(v => Math.round(v * k))") | ConvertFrom-Json
$src = [System.Drawing.Bitmap]::FromFile((Join-Path $outDir "editor.png"))
$pad = 6
$x0 = [math]::Max(0, $b[0] - $pad); $y0 = [math]::Max(0, $b[1] - $pad)
$cw = [math]::Min($src.Width, $b[0] + $b[2] + $pad) - $x0; $ch = [math]::Min($src.Height, $b[1] + $b[3] + $pad) - $y0
$crop = $src.Clone((New-Object System.Drawing.Rectangle $x0, $y0, $cw, $ch), $src.PixelFormat)
$crop.Save("$s\button.png", [System.Drawing.Imaging.ImageFormat]::Png); $crop.Dispose(); $src.Dispose()
$png = ("$s\button.png" -replace '\\', '/')
Js "const r = window.__relay; const image = await window.__TAURI_INTERNALS__.invoke('load_image', { path: '$png' }); const at = r.steps.find(s => s.kind === 'click').t; await r.edit({ op: 'insert_find_image', at, dur: 800, image, click_x: $([int]($cw / 2)), click_y: $([int]($ch / 2)), btn: 'Left', threshold: 85, timeout_ms: 5000, area: null, label: 'Export button' }); const i = r.steps.findIndex(s => s.kind === 'find_image'); r.seek(r.steps[i].t); document.querySelectorAll('.list .row')[i].click(); await new Promise(res => setTimeout(res, 400)); document.querySelector('.editor').scrollIntoView({ block: 'start' }); return i" | Out-Null
Park
Shot "find-image.png"
Js 'const r = window.__relay; const i = r.steps.findIndex(s => s.kind === "find_image"); document.querySelectorAll(".list .row")[i].click(); await r.undo(); r.seek(0); return true' | Out-Null

# 4. The Library, a row hovered to show its actions.
Tab "Library"
Hover ".item:nth-child(2)"
Shot "library.png"
Park

# 4b. The run history, its first run with checks opened.
Js 'await window.__relay.showRuns(true); await new Promise(r => setTimeout(r, 300)); const i = window.__relay.shownRuns.findIndex(r => r.checks.length > 0); document.querySelectorAll(".run .head")[i].click(); document.querySelectorAll(".run")[i].scrollIntoView({ block: "start" }); return i' | Out-Null
Shot "run-history.png"
Js 'await window.__relay.showRuns(false); return true' | Out-Null

# 5. Triggers, with a hotkey and a schedule set (turned off again afterwards).
Js 'await window.__relay.loadMacro(window.__relay.library[0].id); return true' | Out-Null
Js 'await window.__relay.setTriggers({ hotkey: { enabled: true, combo: "Ctrl + Alt + 1" }, schedule: { enabled: true, schedule: { days: [true, true, true, true, true, false, false], time: "09:00" } }, app_launch: { enabled: false, exe: "EXCEL.EXE", delay_ms: 2000 } }); return true' | Out-Null
Tab "Triggers"
Shot "triggers.png"
Js 'const t = window.__relay.triggerStatus.triggers; await window.__relay.setTriggers({ hotkey: { enabled: false, combo: t.hotkey.combo }, schedule: Object.assign({}, t.schedule, { enabled: false }) }); return true' | Out-Null

# 6. Settings.
Tab "Settings"
Shot "settings.png"

# 7. The export dialog.
Tab "Steps"
Js 'window.__relay.exportOpen = true; return true' | Out-Null
Shot "export.png"
Js 'window.__relay.exportOpen = false; return true' | Out-Null

# 8. The countdown before recording (cancelled right after).
Js 'await window.__relay.toggleRec(); await new Promise(r => setTimeout(r, 700)); return true' | Out-Null
Shot "countdown.png"
Js 'await window.__relay.stop(); return true' | Out-Null
Start-Sleep -Milliseconds 400

# 9. The compact player.
Js 'window.__relay.expanded = false; window.__relay.seek(4200); await new Promise(r => setTimeout(r, 700)); return true' | Out-Null
Shot "compact.png"
Js 'window.__relay.expanded = true; return true' | Out-Null
Start-Sleep -Milliseconds 700

# 10. An exported program playing. Its macro is the first sample's, with its
# events replaced by one 8 s wait played once, so it sends nothing to the
# desktop and closes by itself. It's written outside %TEMP%: antivirus
# behavior checks are stricter about programs started from there.
$sample = (Join-Path $s "sample.rly") -replace '\\', '/'
Js "await window.__TAURI_INTERNALS__.invoke('export_macro', { id: window.__relay.library[0].id, format: 'rly', path: '$sample' }); return true" | Out-Null
$doc = Get-Content -Raw $sample | ConvertFrom-Json
$doc.id = [guid]::NewGuid().ToString()
$doc.events = @(@{ type = "wait"; t = 0; dur = 8000; label = "" })
$doc.playback.repeat = @{ count = 1 }
$doc | ConvertTo-Json -Depth 20 | Out-File -Encoding ascii $sample
$programDir = Join-Path $s "program"
New-Item -ItemType Directory -Force $programDir | Out-Null
$program = (Join-Path $programDir "export-invoice-to-pdf.exe") -replace '\\', '/'
Js "const inv = window.__TAURI_INTERNALS__.invoke; const r = await inv('import_macros', { paths: ['$sample'] }); await inv('export_macro', { id: r.imported[0], format: 'exe', path: '$program' }); return true" | Out-Null
$player = Start-Process $program -PassThru
Start-Sleep -Milliseconds 4500
Shot "player.png" ([D1]::FindWindow("RelayPlayer", [NullString]::Value))
$player.WaitForExit(15000) | Out-Null

Stop-Process -Id $relay.Id -Confirm:$false
