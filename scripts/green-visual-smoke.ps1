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
}
'@

function Save-SSTWindow([IntPtr]$Handle, [string]$Destination) {
    $rect = New-Object SSTWin32Preview+RECT
    if (-not [SSTWin32Preview]::GetWindowRect($Handle, [ref]$rect)) {
        throw "GetWindowRect failed"
    }
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    if ($width -lt 450 -or $height -lt 250) {
        throw "Invalid SST window bounds: $width x $height"
    }
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
    [void][SSTWin32Preview]::ShowWindow($handle, 9)
    [void][SSTWin32Preview]::SetForegroundWindow($handle)
    Start-Sleep -Seconds 3
    Save-SSTWindow $handle (Join-Path $OutputDirectory "01-consola-real.png")
    [System.Windows.Forms.SendKeys]::SendWait('sys{ENTER}')
    Start-Sleep -Seconds 6
    Save-SSTWindow $handle (Join-Path $OutputDirectory "02-sys-pantalla-completa.png")
    [System.Windows.Forms.SendKeys]::SendWait('{ESC}')
    Start-Sleep -Seconds 2
    Save-SSTWindow $handle (Join-Path $OutputDirectory "03-regreso-consola.png")
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
