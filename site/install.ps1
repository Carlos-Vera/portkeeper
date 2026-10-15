# Installs or updates Portkeeper on Windows 10 / 11.
#   irm https://portkeeper.pages.dev/install.ps1 | iex
# Downloads the latest installer from GitHub over HTTPS, removes the "downloaded
# from the internet" mark that triggers SmartScreen, and runs it silently for the
# current user. Later updates come from inside the app.
# Source: https://github.com/MarcAparisiTFG/portkeeper
$ErrorActionPreference = "Stop"
$url = "https://github.com/MarcAparisiTFG/portkeeper/releases/latest/download/Portkeeper-Windows-x64-setup.exe"
$exe = Join-Path $env:TEMP "Portkeeper-setup.exe"

Write-Host "Downloading Portkeeper for Windows..."
Invoke-WebRequest -Uri $url -OutFile $exe -UseBasicParsing
Unblock-File $exe

Get-Process portkeeper -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
Write-Host "Installing..."
Start-Process -FilePath $exe -ArgumentList "/S" -Wait
Remove-Item $exe -ErrorAction SilentlyContinue

$candidates = @(
  (Join-Path $env:LOCALAPPDATA "Portkeeper\portkeeper.exe"),
  (Join-Path $env:LOCALAPPDATA "Programs\Portkeeper\portkeeper.exe"),
  (Join-Path $env:ProgramFiles "Portkeeper\portkeeper.exe")
)
$app = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if ($app) {
  Start-Process -FilePath $app
  Write-Host "Portkeeper is installed and running in your system tray."
} else {
  Write-Host "Portkeeper is installed. Open it from the Start menu."
}
Write-Host "Updates arrive inside the app. Settings, locks and the Closed list are kept."
