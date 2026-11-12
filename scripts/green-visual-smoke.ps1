# Captura comprobable de SST Green ejecutándose en Windows.
# Las imágenes provienen de la ventana real de Slint, no de una maqueta.
param(
    [Parameter(Mandatory = $true)][string]$Executable,
    [Parameter(Mandatory = $true)][string]$OutputDirectory
)
$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class SSTWin32Preview {
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int cmd);
  [DllImport("user32.dll")] public static extern bool MoveWindow(IntPtr hWnd, int x, int y, int width, int height, bool repaint);
}
'@

function Save-SSTWindow([System.Diagnostics.Process]$SSTProcess, [string]$Destination) {
    # Alternate-screen transitions may recreate the native HWND. The old
    # handle is not guaranteed to survive sys/network monitor entry or exit.
    $valid = $false
    for ($retry = 0; $retry -lt 35; $retry++) {
        $SSTProcess.Refresh()
        if ($SSTProcess.HasExited) {
            throw "SST closed while capturing $Destination (exit code $($SSTProcess.ExitCode))"
        }
        $handle = $SSTProcess.MainWindowHandle
        $rect = New-Object SSTWin32Preview+RECT
        if ($handle -ne [IntPtr]::Zero -and
            [SSTWin32Preview]::GetWindowRect($handle, [ref]$rect)) {
            $width = $rect.Right - $rect.Left
            $height = $rect.Bottom - $rect.Top
            if ($width -ge 450 -and $height -ge 250) {
                $valid = $true
                break
            }
        }
        Start-Sleep -Milliseconds 250
    }
    if (-not $valid) {
        throw "SST has no drawable native window for $Destination (PID $($SSTProcess.Id))"
    }

    [void][SSTWin32Preview]::SetForegroundWindow($handle)
    $bitmap = New-Object System.Drawing.Bitmap($width, $height)
    try {
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        try {
            $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size)
        } finally { $graphics.Dispose() }
        $bitmap.Save($Destination, [System.Drawing.Imaging.ImageFormat]::Png)
    } finally { $bitmap.Dispose() }
}

$process = $null
try {
    $exe = (Resolve-Path $Executable).Path
    $process = Start-Process -FilePath $exe -ArgumentList '--gui' -WorkingDirectory (Split-Path $exe) -PassThru
    $handle = [IntPtr]::Zero
    for ($i = 0; $i -lt 40; $i++) {
        Start-Sleep -Milliseconds 500
        $process.Refresh()
        if ($process.HasExited) { throw "SST exited early with code $($process.ExitCode)" }
        $handle = $process.MainWindowHandle
        if ($handle -ne [IntPtr]::Zero) { break }
    }
    if ($handle -eq [IntPtr]::Zero) { throw "SST did not create a native window" }
    # Winit/Slint may initially create a 16x16 placeholder HWND. Do not
    # treat that transient rectangle as the actual rendered console.
    $validBounds = $false
    for ($attempt = 0; $attempt -lt 35; $attempt++) {
        $process.Refresh()
        if ($process.HasExited) { throw "SST exited before becoming drawable" }
        [void][SSTWin32Preview]::ShowWindow($handle, 9)
        if ($attempt -ge 5) {
            [void][SSTWin32Preview]::MoveWindow($handle, 25, 25, 1260, 780, $true)
        }
        [void][SSTWin32Preview]::SetForegroundWindow($handle)
        Start-Sleep -Milliseconds 400
        $rect = New-Object SSTWin32Preview+RECT
        if ([SSTWin32Preview]::GetWindowRect($handle, [ref]$rect)) {
            if (($rect.Right - $rect.Left) -ge 640 -and ($rect.Bottom - $rect.Top) -ge 400) {
                $validBounds = $true
                break
            }
        }
        $handle = $process.MainWindowHandle
    }
    if (-not $validBounds) {
        throw "SST did not expose a drawable application window (last handle $handle)."
    }
    Save-SSTWindow $process (Join-Path $OutputDirectory "01-consola-real.png")
    [System.Windows.Forms.SendKeys]::SendWait('sys{ENTER}')
    Start-Sleep -Seconds 6
    Save-SSTWindow $process (Join-Path $OutputDirectory "02-sys-pantalla-completa.png")
    [System.Windows.Forms.SendKeys]::SendWait('{ESC}')
    Start-Sleep -Seconds 2
    Save-SSTWindow $process (Join-Path $OutputDirectory "03-regreso-consola.png")
    "Capturas reales obtenidas del proceso SST (PID $($process.Id))." |
        Out-File -FilePath (Join-Path $OutputDirectory "resultado.txt") -Encoding utf8
} catch {
    "Error de captura real en runner: $_" |
        Out-File -FilePath (Join-Path $OutputDirectory "resultado.txt") -Encoding utf8
    throw
} finally {
    if ($null -ne $process) {
        try { if (-not $process.HasExited) { Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue } }
        catch { }
    }
}
