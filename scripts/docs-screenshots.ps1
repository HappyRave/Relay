# Regenerates the documentation screenshots in docs\images from the real app.
#
# Needs a build with the UI in it (npx tauri build, or pass -Exe with the debug
# one) and Node. Runs Relay with a throwaway data folder, seeded with the
# design's sample macros, light and solid, and drives it through scripts\cdp.mjs.
# Then starts it once more on an empty folder, for the first run. Each window is
# captured on its own (PrintWindow), so windows covering it don't matter; don't
# use the mouse or keyboard while it runs, though.
#
# Usage: powershell -ExecutionPolicy Bypass -File scripts\docs-screenshots.ps1 [-Exe target\debug\relay.exe]
param([string]$Exe = "target\release\relay.exe")
$root = Split-Path $PSScriptRoot -Parent
$exePath = if ([IO.Path]::IsPathRooted($Exe)) { $Exe } else { Join-Path $root $Exe }
$s = Join-Path $root "target\docs-shots"
$outDir = Join-Path $root "docs\images"
$data = Join-Path $s "data"
if (Test-Path $s) { Remove-Item -Recurse -Force $s -Confirm:$false }
New-Item -ItemType Directory -Force $s, $data, $outDir | Out-Null
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System; using System.Runtime.InteropServices;
public class D1 { [StructLayout(LayoutKind.Sequential)] public struct R { public int L, T, Ri, B; }
  [DllImport("dwmapi.dll")] public static extern int DwmGetWindowAttribute(IntPtr h, int a, out R r, int s);
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out R r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr dc, uint flags);
  [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
  [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string c, string t);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h); }
"@
[D1]::SetProcessDPIAware() | Out-Null
function Js($code, $window = "main") { $code | Out-File -Encoding utf8 "$s\snippet.js"; node (Join-Path $PSScriptRoot "cdp.mjs") 9333 "$s\snippet.js" $window }
# The tray icon also owns a window titled "Relay"; Relay's own are "Tauri Window"s.
function Main() { [D1]::FindWindow("Tauri Window", "Relay") }
function Player() { [D1]::FindWindow("Tauri Window", "Relay player") }
function Shot($name, $h = (Main)) {
  # An error from an earlier step would stay on screen.
  Js 'window.__relay.clearToasts(); return true' | Out-Null
  Start-Sleep -Milliseconds 400
  # The window as it draws itself (PW_RENDERFULLCONTENT), cut to its visible
  # frame (DWMWA_EXTENDED_FRAME_BOUNDS), without the invisible borders.
  $w = New-Object D1+R; [D1]::GetWindowRect($h, [ref]$w) | Out-Null
  $f = New-Object D1+R; [D1]::DwmGetWindowAttribute($h, 9, [ref]$f, 16) | Out-Null
  $all = New-Object System.Drawing.Bitmap ($w.Ri - $w.L), ($w.B - $w.T)
  $g = [System.Drawing.Graphics]::FromImage($all); $dc = $g.GetHdc()
  [D1]::PrintWindow($h, $dc, 2) | Out-Null
  $g.ReleaseHdc($dc); $g.Dispose()
  $frame = New-Object System.Drawing.Rectangle ($f.L - $w.L), ($f.T - $w.T), ($f.Ri - $f.L), ($f.B - $f.T)
  $bmp = $all.Clone($frame, $all.PixelFormat)
  $bmp.Save((Join-Path $outDir $name), [System.Drawing.Imaging.ImageFormat]::Png); $all.Dispose(); $bmp.Dispose()
  "saved $name"
}
function Park() { [D1]::SetCursorPos(20, 20) | Out-Null }
function Start-Relay($dir) {
  $env:RELAY_DATA_DIR = $dir
  $env:RELAY_DEVTOOLS_PORT = "9333"
  $p = Start-Process $exePath -PassThru
  Start-Sleep -Seconds 7
  [D1]::SetForegroundWindow((Main)) | Out-Null
  Park
  $p
}

# The samples (as the E2E tests seed them), the new install's keys, the first
# run done, and the light theme without glass, whatever this PC's look.
@'
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
const [samples, dir] = process.argv.slice(2);
mkdirSync(`${dir}/macros`, { recursive: true });
for (const m of JSON.parse(readFileSync(samples, "utf8"))) writeFileSync(`${dir}/macros/${m.id}.rly`, JSON.stringify(m));
const keys = { record: "Ctrl + Shift + F9", play: "Ctrl + Shift + F10", player: "Ctrl + Shift + M", kill: "Ctrl + Alt + Backspace" };
writeFileSync(`${dir}/settings.json`, JSON.stringify({ keys, intro: "done", theme: "light", glass: false }));
'@ | Out-File -Encoding utf8 "$s\seed.mjs"
node "$s\seed.mjs" (Join-Path $root "e2e\samples.json") $data

# The run history, from the browser preview's fixture, dated back from now.
$runs = Get-Content -Raw (Join-Path $root "src\lib\dev\sample-runs.json") | ConvertFrom-Json
$now = [DateTime]::UtcNow
$entries = @($runs | Sort-Object { -$_.ago_ms } | ForEach-Object { $_.entry.at = $now.AddMilliseconds(-$_.ago_ms).ToString("yyyy-MM-ddTHH:mm:ssZ"); $_.entry })
@{ version = 1; entries = $entries } | ConvertTo-Json -Depth 10 | Out-File -Encoding ascii (Join-Path $data "runs.json")

$relay = Start-Relay $data
$open = 'const r = window.__relay; await r.openMacro(r.library.find((m) => m.name === "Export invoice to PDF").id); await new Promise((res) => setTimeout(res, 500)); '

# 1. The editor on the invoice macro, its playhead mid-way.
Js ($open + 'r.select([]); r.seek(4200); return true') | Out-Null
Shot "editor.png"

# 2. Playing, on loop 1 of 3. The samples were recorded on a 1920x1080 desktop
# and would really click there, so this plays only inside the longest pause
# (stretched to 5 s, then undone), where nothing happens.
$playInPause = 'const r = window.__relay; const i = r.steps.reduce((b, s, j) => (s.pause > r.steps[b].pause ? j : b), 0); await r.setPause(i, 5000); await r.setPlayback({ repeat: { count: 3 } }); const s = r.steps[i]; r.seek(s.t - s.pause + 200); await r.togglePlay(); return i'
$stopAndUndo = 'const r = window.__relay; await r.stop(); while (r.mode !== "idle") await new Promise((res) => setTimeout(res, 50)); await r.undo(); r.seek(0); return true'
Js $playInPause | Out-Null
Start-Sleep -Milliseconds 1200
Shot "playing.png"
Js $stopAndUndo | Out-Null

# 3. The Step tab on the pixel check.
Js 'const r = window.__relay; const i = r.steps.findIndex((s) => s.kind === "pixel_wait"); r.select([i]); await new Promise((res) => setTimeout(res, 400)); return i' | Out-Null
Shot "step-editor.png"

# 3b. The Step tab on a Find image step, whose image is the top bar's Play key
# cut out of editor.png (inserted after the first click, then undone).
$b = (Js "const b = [...document.querySelectorAll('header.bar button')].find((b) => /^Play/.test(b.textContent.trim())).getBoundingClientRect(); const k = devicePixelRatio; return [b.x, b.y, b.width, b.height].map((v) => Math.round(v * k))") | ConvertFrom-Json
$src = [System.Drawing.Bitmap]::FromFile((Join-Path $outDir "editor.png"))
$pad = 6
$x0 = [math]::Max(0, $b[0] - $pad); $y0 = [math]::Max(0, $b[1] - $pad)
$cw = [math]::Min($src.Width, $b[0] + $b[2] + $pad) - $x0; $ch = [math]::Min($src.Height, $b[1] + $b[3] + $pad) - $y0
$crop = $src.Clone((New-Object System.Drawing.Rectangle $x0, $y0, $cw, $ch), $src.PixelFormat)
$crop.Save("$s\button.png", [System.Drawing.Imaging.ImageFormat]::Png); $crop.Dispose(); $src.Dispose()
$png = ("$s\button.png" -replace '\\', '/')
Js "const r = window.__relay; const image = await window.__TAURI_INTERNALS__.invoke('load_image', { path: '$png' }); const at = r.steps.find((s) => s.kind === 'click').t; await r.edit({ op: 'insert_find_image', at, dur: 800, image, click_x: $([int]($cw / 2)), click_y: $([int]($ch / 2)), btn: 'Left', threshold: 85, timeout_ms: 5000, area: null, label: 'Play key' }); r.select([r.steps.findIndex((s) => s.kind === 'find_image')]); await new Promise((res) => setTimeout(res, 400)); return true" | Out-Null
Shot "find-image.png"
Js 'const r = window.__relay; r.select([]); await r.undo(); r.seek(0); return true' | Out-Null

# 4. The Macro tab's triggers, with a hotkey and a schedule set (turned off again afterwards).
Js 'const r = window.__relay; await r.setTriggers({ hotkey: { enabled: true, combo: "Ctrl + Alt + 1" }, schedule: { enabled: true, schedule: { days: [true, true, true, true, true, false, false], time: "09:00" } } }); r.inspector = "macro"; r.macroSection = "triggers"; return true' | Out-Null
Shot "triggers.png"
Js 'const r = window.__relay; const t = r.triggerStatus.triggers; await r.setTriggers({ hotkey: { enabled: false, combo: t.hotkey.combo }, schedule: Object.assign({}, t.schedule, { enabled: false }) }); r.inspector = "step"; return true' | Out-Null

# 5. The export dialog.
Js 'window.__relay.exportOpen = true; return true' | Out-Null
Shot "export.png"
Js 'window.__relay.exportOpen = false; return true' | Out-Null

# 6. The Library.
Js 'const r = window.__relay; r.go("library"); await r.showLibrary("macros"); return true' | Out-Null
Shot "library.png"

# 7. Activity, its first run with checks opened.
Js 'const r = window.__relay; await r.showLibrary("runs"); await new Promise((res) => setTimeout(res, 300)); const i = r.shownRuns.findIndex((x) => x.checks.length > 0); document.querySelectorAll(".run .head")[i].click(); return i' | Out-Null
Shot "activity.png"
Js 'await window.__relay.showLibrary("macros"); return true' | Out-Null

# 8. Settings: Relay's keys.
Js 'window.__relay.openSettings("keys"); return true' | Out-Null
Shot "settings.png"

# 9. The player counting down before a recording (cancelled right after).
Js ($open + 'await r.toggleRec(); await new Promise((res) => setTimeout(res, 700)); return true') | Out-Null
Shot "countdown.png" (Player)
Js 'const r = window.__relay; await r.stop(); while (r.mode !== "idle") await new Promise((res) => setTimeout(res, 50)); return true' | Out-Null
Start-Sleep -Milliseconds 800

# 10. The player, in player mode, playing inside the stretched pause.
Js 'await window.__relay.setPlayerMode(true); return true' | Out-Null
Start-Sleep -Milliseconds 600
Js $playInPause | Out-Null
Start-Sleep -Milliseconds 1200
Shot "player.png" (Player)
Js $stopAndUndo | Out-Null
Js 'await window.__relay.setPlayerMode(false); return true' | Out-Null
Start-Sleep -Milliseconds 600

# 11. An exported program playing. Its macro is the invoice sample's, with its
# events replaced by one 8 s wait played once, so it sends nothing to the
# desktop and closes by itself. It's written outside %TEMP%: antivirus
# behavior checks are stricter about programs started from there.
$sample = (Join-Path $s "sample.rly") -replace '\\', '/'
Js ($open + "await window.__TAURI_INTERNALS__.invoke('export_macro', { id: r.view.id, format: 'rly', path: '$sample' }); return true") | Out-Null
$doc = Get-Content -Raw $sample | ConvertFrom-Json
$doc.id = [guid]::NewGuid().ToString()
$doc.events = @(@{ type = "wait"; t = 0; dur = 8000; label = "" })
$doc.playback.repeat = @{ count = 1 }
$doc | ConvertTo-Json -Depth 20 | Out-File -Encoding ascii $sample
$programDir = Join-Path $s "program"
New-Item -ItemType Directory -Force $programDir | Out-Null
$program = (Join-Path $programDir "export-invoice-to-pdf.exe") -replace '\\', '/'
Js "const inv = window.__TAURI_INTERNALS__.invoke; const r = await inv('import_macros', { paths: ['$sample'] }); await inv('export_macro', { id: r.imported[0], format: 'exe', path: '$program' }); return true" | Out-Null
$exported = Start-Process $program -PassThru
Start-Sleep -Milliseconds 4500
Shot "exported.png" ([D1]::FindWindow("RelayPlayer", [NullString]::Value))
$exported.WaitForExit(15000) | Out-Null
Stop-Process -Id $relay.Id -Confirm:$false
Start-Sleep -Seconds 2

# 12. The first run, on an empty folder.
$fresh = Join-Path $s "fresh"
New-Item -ItemType Directory -Force $fresh | Out-Null
'{ "theme": "light", "glass": false }' | Out-File -Encoding ascii (Join-Path $fresh "settings.json")
$relay = Start-Relay $fresh
Shot "first-run.png"
Stop-Process -Id $relay.Id -Confirm:$false
