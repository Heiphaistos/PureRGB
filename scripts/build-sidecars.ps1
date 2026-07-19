# Construit les sidecars embarques dans l'installeur :
# - liquidctl.exe (PyInstaller onefile depuis PyPI, version epinglee)
# - sensord.exe (dotnet publish self-contained, LibreHardwareMonitorLib)
# Prerequis : Python 3.10+, .NET SDK 8.
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$LiquidctlVersion = "1.16.0"

$root = Split-Path $PSScriptRoot -Parent
$resources = Join-Path $root "src-tauri\resources"

# --- liquidctl ---
$lcDest = Join-Path $resources "liquidctl"
if (-not (Test-Path (Join-Path $lcDest "liquidctl.exe"))) {
    $work = Join-Path $env:TEMP "purergb-lcbuild"
    if (Test-Path $work) { Remove-Item $work -Recurse -Force }
    New-Item -ItemType Directory -Force $work | Out-Null
    python -m venv (Join-Path $work "venv")
    $pip = Join-Path $work "venv\Scripts\pip.exe"
    & $pip install --quiet "liquidctl==$LiquidctlVersion" pillow pyinstaller
    Set-Content (Join-Path $work "lc_entry.py") "import sys`nfrom liquidctl.cli import main`nsys.exit(main())" -Encoding ascii
    Push-Location $work
    & (Join-Path $work "venv\Scripts\pyinstaller.exe") --onefile --console --name liquidctl `
        --collect-submodules liquidctl --copy-metadata liquidctl lc_entry.py
    Pop-Location
    New-Item -ItemType Directory -Force $lcDest | Out-Null
    Copy-Item (Join-Path $work "dist\liquidctl.exe") $lcDest -Force
    Remove-Item $work -Recurse -Force -ErrorAction SilentlyContinue
}
Write-Host "liquidctl.exe pret: $lcDest"

# --- sensord ---
$sdDest = Join-Path $resources "sensord"
if (-not (Test-Path (Join-Path $sdDest "sensord.exe"))) {
    Push-Location (Join-Path $root "sidecars\sensord")
    dotnet publish -c Release -o publish
    Pop-Location
    New-Item -ItemType Directory -Force $sdDest | Out-Null
    Copy-Item (Join-Path $root "sidecars\sensord\publish\sensord.exe") $sdDest -Force
}
Write-Host "sensord.exe pret: $sdDest"
