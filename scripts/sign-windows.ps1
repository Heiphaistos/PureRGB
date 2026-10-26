# Signature Authenticode appelée par Tauri (bundle.windows.signCommand) pour chaque exe/installeur.
# Sans secret PURERGB_PFX_B64 (build local, fork, PR) : ne fait rien et ne casse pas le build.
param([Parameter(Mandatory = $true)][string]$Fichier)
$ErrorActionPreference = "Stop"
if (-not $env:PURERGB_PFX_B64) { Write-Host "signature ignorée (pas de certificat) : $Fichier"; exit 0 }

$pfx = Join-Path $env:RUNNER_TEMP "purergb-signing.pfx"
if (-not (Test-Path $pfx)) { [IO.File]::WriteAllBytes($pfx, [Convert]::FromBase64String($env:PURERGB_PFX_B64)) }

$signtool = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin" -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
    Where-Object { $_.Directory.Name -eq "x64" } | Sort-Object FullName -Descending | Select-Object -First 1
if (-not $signtool) { throw "signtool.exe introuvable" }

# Horodatage obligatoire : sans lui la signature meurt à l'expiration du certificat (2031-09-22).
& $signtool.FullName sign /f $pfx /p $env:PURERGB_PFX_PASSWORD /fd SHA256 /td SHA256 /tr "http://timestamp.digicert.com" /d "PureRGB" $Fichier
if ($LASTEXITCODE -ne 0) { throw "signtool a échoué ($LASTEXITCODE) pour $Fichier" }
