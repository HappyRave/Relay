# Regenerates the documentation screenshots in docs\images from the real app.
#
# Needs a release build (npx tauri build) and Node. Runs Relay with a throwaway
# data folder, which is seeded with the design's sample macros, and drives it
# through scripts\cdp.mjs. Don't touch the mouse or keyboard while it runs.
#
# Usage: powershell -ExecutionPolicy Bypass -File scripts\docs-screenshots.ps1
$root = Split-Path $PSScriptRoot -Parent
$s = Join-Path $env:TEMP "relay-docs-shots"
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
function Shot($name) {
  Start-Sleep -Milliseconds 350
  # The visible frame (DWMWA_EXTENDED_FRAME_BOUNDS), without the invisible borders.
  $h = Hwnd; $r = New-Object D1+R; [D1]::DwmGetWindowAttribute($h, 9, [ref]$r, 16) | Out-Null
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

$env:RELAY_DATA_DIR = $data
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = "--remote-debugging-port=9333"
$relay = Start-Process (Join-Path $root "target\release\relay.exe") -PassThru
Start-Sleep -Seconds 7
[D1]::SetForegroundWindow((Hwnd)) | Out-Null
Park

# 1. The editor mid-way through the invoice macro.
Js 'window.__relay.seek(4200); return true' | Out-Null
Shot "editor.png"

# 2. Playing, with the loop badge and the red trail. The samples were recorded
# on a 1920x1080 desktop, so this really clicks there: keep the screen clear.
Js 'await window.__relay.setPlayback({ repeat: { count: 3 } }); window.__relay.seek(0); await window.__relay.togglePlay(); return true' | Out-Null
Start-Sleep -Milliseconds 5600
Shot "playing.png"
Js 'await window.__relay.stop(); return true' | Out-Null

# 3. The step editor on the pixel check.
Js 'const i = window.__relay.steps.findIndex(s => s.kind === "pixel_wait"); document.querySelectorAll(".list .row")[i].click(); await new Promise(r => setTimeout(r, 400)); return i' | Out-Null
Park
Shot "step-editor.png"
Js 'const i = window.__relay.steps.findIndex(s => s.kind === "pixel_wait"); document.querySelectorAll(".list .row")[i].click(); window.__relay.seek(0); return true' | Out-Null

# 4. The Library, a row hovered to show its actions.
Tab "Library"
Hover ".item:nth-child(2)"
Shot "library.png"
Park

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

Stop-Process -Id $relay.Id -Confirm:$false
