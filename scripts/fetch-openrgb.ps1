# Télécharge OpenRGB 1.0rc3 portable dans src-tauri/resources/openrgb/
# (embarqué ensuite dans l'installeur NSIS par tauri build).
# Vérifie le SHA-256 officiel avant extraction.
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

$url = "https://codeberg.org/OpenRGB/OpenRGB/releases/download/release_candidate_1.0rc3/OpenRGB_1.0rc3_Windows_64_6fbcf62.zip"
$sha = "A6BB0FBCB7B6EB84214287E3808FADAE2777C902EFB3DD6CD1E2976F14271C8C"
$root = Split-Path $PSScriptRoot -Parent
$dest = Join-Path $root "src-tauri\resources\openrgb"

# PawnIOLib.dll n'existe que depuis 1.0rc : son absence = ancienne version 0.9 a purger.
if ((Test-Path (Join-Path $dest "OpenRGB.exe")) -and (Test-Path (Join-Path $dest "PawnIOLib.dll"))) {
    Write-Host "OpenRGB deja present: $dest"
    exit 0
}
if (Test-Path $dest) { Remove-Item $dest -Recurse -Force }

$zip = Join-Path $env:TEMP "openrgb_fetch.zip"
$tmp = Join-Path $env:TEMP "openrgb_fetch"
Write-Host "Telechargement $url"
Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing

$actual = (Get-FileHash $zip -Algorithm SHA256).Hash
if ($actual -ne $sha) {
    Remove-Item $zip -Force
    throw "SHA-256 inattendu: $actual (attendu $sha)"
}

if (Test-Path $tmp) { Remove-Item $tmp -Recurse -Force }
Expand-Archive $zip $tmp -Force
New-Item -ItemType Directory -Force (Split-Path $dest -Parent) | Out-Null
if (Test-Path $dest) { Remove-Item $dest -Recurse -Force }
Move-Item (Join-Path $tmp "OpenRGB Windows 64-bit") $dest
Remove-Item $zip -Force
Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue

# VC++ runtime app-local : OpenRGB (Qt/MSVC) ne demarre pas sans ces DLLs
# sur un Windows vierge (aucune erreur visible, process vivant mais mort-ne).
foreach ($dll in 'vcruntime140.dll','vcruntime140_1.dll','msvcp140.dll') {
    $src = Join-Path $env:WINDIR "System32\$dll"
    if (Test-Path $src) { Copy-Item $src $dest -Force }
    else { Write-Warning "$dll absent de System32 - OpenRGB pourrait ne pas demarrer sur systeme vierge" }
}
Write-Host "OpenRGB installe dans $dest"
