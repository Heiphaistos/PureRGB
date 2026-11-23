# Analyse de MSI Center 2.0.76.0 (REA statique, 2026-11-24)

## Méthode
- `https://download-2.msi.com/uti_exe/desktop/MSI-Center.zip` (595 Mo) : l'empreinte SHA-256 du manifeste winget (version 2.0.72) ne correspond plus
  (fichier publié depuis : 2.0.76.0), donc aucune vérification d'intégrité possible ; analyse STATIQUE uniquement, rien exécuté ni installé.
- Zip -> `MSI Center_2.0.76.0.exe` (Inno Setup, `innoextract`) -> paquet MSIX `.appxbundle` -> `.appx` (zip) -> dossier `DCv2` (application .NET/WPF).
- Bibliothèques natives du module Mystic Light : `CMSILedDll.dll` (MFC, x64), `MSILedDll_MyDevice.dll`, `Mystic Light/MsiHid_x64.dll`,
  `Mystic Light/MsiGKbApi_x64.dll`. `CMSILedDll.dll` décompilée avec Ghidra headless (2,1 Mo de C, bruit MFC dominant, sortie tronquée par le délai).

## Constats
- Le paquet de base ne contient PAS les données par carte mère : les tables VID/PID et les profils de cartes (MS-7Dxx…) sont téléchargés à l'exécution
  par MSI Center (module Mystic Light). Aucune liste de PID de cartes mères exploitable n'est donc extractible du paquet seul.
- `MsiHid_x64.dll` construit les chemins HID par `VID_%04X` / `PID_%04X` ; `CMSILedDll.dll` traite explicitement `1462:3FA4` (périphérique, non identifié).
- Dossier `Device` : profils de périphériques MSI (GH70, GK7080, GM08, GM20 Elite, GM41W, GM6070 + `KeyConfig*.txt`).
- Les 185 octets du Mystic Light carte mère déjà pris en charge (`msi_mystic.rs`) viennent de la note `msi-mystic-light.md` ; MSI Center n'apporte pas de PID supplémentaire sans le module téléchargé.

## Pour aller plus loin
- Installer MSI Center sur une machine avec une carte MSI, laisser le module Mystic Light se télécharger, puis copier le dossier du module
  (sous `%ProgramFiles%\WindowsApps\…MSICenter…` ou `%ProgramData%\MSI\`) : c'est là que se trouvent les tables de cartes.
