# Analyse de Gigabyte Control Center 26.11.05.01 (REA, 2026-11-24)

## Méthode
- Paquet officiel `GCC_26.11.05.01.zip` (`download.gigabyte.com/FileList/Utility/`), 861 Mo : programme NSIS contenant un installateur NSIS par module
  (`GBT_VGA`, `GBT_rgbMotherboard_UC`, `GBT_RGB_Sync_Control`, `GBT_SingleLed`, `GBT_Keyboard`, `GBT_Mouse`, `GBT_Cooler`…).
- `GBT_rgbMotherboard_UC` et `GBT_RGB_Sync_Control` s'ouvrent avec 7-Zip. `GBT_VGA` et `GBT_SingleLed` ont un flux chiffré (7-Zip : « Is not archive ») :
  `GBT_VGA` a été exécuté en silencieux (`/S`, droits administrateur) pour déposer ses fichiers, copiés dans `D:\Projet\analyse\gigabyte\vga_files`,
  puis désinstallé (`Lib\GBT_VGA\uninst.exe /S`) ; le service `AorusLcdService` qu'il crée a disparu et `C:\Program Files\GIGABYTE` a été supprimé.
- `GvLedLib.dll` (x64, natif, PDB `GvLedLib_New_241202_Add_N50_Lcd`) décompilée avec Ghidra 12.1.4 headless
  (`D:\Projet\analyse\outils\ghidra\DecompNamed.java`), 1893 fonctions.

## Constats
- `GvLedLib.dll` exporte une fonction par famille de carte (`dllexp_GvCommLedSet` aiguille par identifiant de sous-système vers
  `GvP1200WLedSet`, `Gv208SAORUSXLedSet`, `GvAIOLedSet`, `GvKKBLedSet`, `GvAMP900LedSet`…). Les écritures I2C passent par une structure
  (adresse 8 bits, drapeau « registre utilisé » à +4, octets à +8, taille à +0x108) puis `CNvApiTemp::WriteI2C` / `GvWriteI2C` (3 essais, pause 10 ms).
- **Gen3 (Xtreme, adresse 0x60)** : `FUN_180067340` envoie en I2C brute un paquet de **0x40 octets** `[E6, zone, mode, luminosité, …, nombre de LEDs, RGB…]`
  (jusqu'à 8 LEDs par paquet) ; `FUN_180067580` envoie `[AA]` (enregistrement permanent) sur 0x40 octets : **jamais utilisé par PureRGB**. Concorde avec signalrgb.md.
- Le `[D6]` de GCC (`HidD_SetFeature`, rapport de 9 octets) est un échange HID USB, pas de l'I2C de carte graphique : l'init `[D6]` bloc de 32 octets de la note SignalRGB
  n'est PAS confirmée par GCC. Elle reste dans le pilote (note SignalRGB) ; à retirer si une carte réelle la rejette.
- **Gen4 (0x75)** : l'en-tête `12 01 01 06 0A` de signalrgb.md n'a pas été retrouvé dans `GvLedLib.dll` (ni sous forme d'octets ni de constante 32 bits) :
  probablement dans `GvIllumLib.dll` / `GvADLX.dll` (non analysées). Pilote non écrit : INCONNU.
- Autres bibliothèques utiles du module : `GLedApi.dll` (x86), `GvIllumLib.dll` (NVIDIA Illumination), `GvIntelI2C.dll`, `GvADLX.dll` (AMD), `Model.ini`
  (identifiant de sous-système -> comportement), `GbtSioLib.dll` (Super I/O). Rien de cela n'est repris dans PureRGB à ce jour.
- Module carte mère : `RGBFI.dll` (.NET, 13 classes, pas de trace de protocole GPU), `SMBCtrl.dll` (x64, SMBus), `GHidApi.dll`, `rgbcfg.xml` (modes par défaut,
  tables `mono_line` par identifiant de carte mère).
