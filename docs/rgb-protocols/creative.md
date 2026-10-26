# Creative (Sound Blaster / SXFI / Katana) : analyse du logiciel officiel

Document d'interopérabilité (PureRGB, MIT). Aucun code ni asset Creative copié ; uniquement des faits de protocole et d'inventaire.
Marqueurs : **[V]** = lu dans le logiciel officiel décompilé ; **[S]** = source tierce, non vérifié ; **INCONNU** = non établi, rien n'est deviné.

## 1. Sources analysées (extraction SANS exécution, rien installé sur ce PC)

| Logiciel | Version | URL officielle | SHA-256 (= manifeste winget) |
|---|---|---|---|
| Sound Blaster Command | 3.5.10.00 | `https://files.creative.com/creative/bin/apps/swureleases/win/sbcommand/release/CreativeSBCommandSetup_3.5.10.00.exe` | `2b6d9bcd9f0a436c90a34e3af5ee3706c3e749cbe703ebddccfad9c21747a722` |
| Creative App | 1.24.05.01 | `https://files.creative.com/creative/bin/apps/swureleases/win/creativeapp/release/CreativeAppSetup_1.24.05.01.exe` | `0754362c9d6d55310fcebfbf161bb33b7b897957c2d29d1f2216d7365eb97918` |

Méthode : `innoextract` (installeurs Inno Setup) puis ILSpy (`ilspycmd`) sur les assemblages .NET (`Creative.Platform.Devices/HIDMgr/Lighting`)
et Ghidra headless 12.1.4 sur les DLL natives 32 bits `CTHIDRpA.dll` (pile HID), `CTHID.dll` (transport HID). Aucun pilote ajouté, aucun service,
aucun processus : il n'y a donc rien à désinstaller.

Limites constatées :
- Les modules d'interface par produit (Katana, Kratos S5, H6...) ne sont PAS dans les installeurs : Creative App les charge depuis son cloud
  (`commonappdata/Creative/CreativeApp/Packages.json` est chiffré ; le déchiffrer reviendrait à contourner une protection : non fait). Sound Blaster
  Command ne publie de modules que pour AE-5, AE-9, G6 et « SBX2Offline ».
- Le téléchargement depuis la page de support est protégé par un captcha ; seule l'URL CDN publiée par winget a été utilisée.
- Le pilote audio noyau Creative n'est pas dans les installeurs.

## 2. VID/PID Creative (VID 0x041E = 1054 ; PCI HDA 0x1102)

Sources : `VendorConstants.cs`, `SupportedModels.cs` (assemblage `Creative.Platform.Devices`), table de noms de `CTHIDRpA.dll` (fonction de libellé produit).
Colonne « Nom » : [V] quand il vient des chaînes de `CTHIDRpA.dll`, sinon nom de code interne du logiciel ([V]) avec nom commercial [S].

| PID | Nom | Nom de code [V] | Transport observé [V] | Éclairage / réglages exposés par le logiciel |
|---|---|---|---|---|
| 3247 | Sound BlasterX Katana | Avenger / `Katana` | USB audio + HID (route « CTSPP ») | 49 LED (7 groupes de 7 LED sur 7 colonnes, mode « Solo » = 1 groupe de 49) ; motifs ; réglages audio par pilote |
| 30CA | Sound BlasterX Kratos S5 | `Kratos2` | USB audio + HID | 8 LED (8 groupes de 1) ; motifs ; audio par pilote |
| 3255 | Sound BlasterX H6 | MegatronUSB / `H6` | HID (page LED séparée [S]) | éclairage « LedV2 » (préréglages, vitesse, luminosité) |
| 3256 | Sound BlasterX G6 | `G6` | USB audio + HID | pas d'éclairage RGB ; audio par pilote (module G6 publié) |
| 3126 | Sound BlasterX Vanguard K08 (US) | Appo / `Vanguard` | HID clavier | RGB par touche, motifs, macros |
| 3128 | Sound BlasterX Vanguard K08 (Allemand) | `VanguardGer` | HID clavier | idem |
| 3129 | Sound BlasterX Vanguard K08 (Nordique) | `VanguardNor` | HID clavier | idem |
| 3127 | Sound BlasterX Siege M04 | Cody / `Siege` | HID souris | RGB (molette, logo X, boutons), DPI, macros |
| 3258 | SXFI AIR | MegatronBT / `SX1` | **CDC série + authentification** | égaliseur, éclairage |
| 3253 | SXFI AIR Theater | MegatronW / `SX2` | CDC série + authentification | égaliseur, éclairage |
| 3262 | SXFI AIR C | MegatronAS / `SX3` | CDC série + authentification | égaliseur, éclairage |
| 3269 | SXFI GAMER | MegatronAS2 / `SXGame` | CDC série + authentification | égaliseur, éclairage |
| 3E10 | SXFI AIR GAMER [S] | OptimusBT / `SXAirGamer` | CDC série + authentification | égaliseur, éclairage |
| 6007 | SXFI AMP [S] | Nighthawk / `SXFiAmp` | CDC série + authentification | égaliseur, éclairage |
| 3264 | Sound Blaster X3 [S] | `X3` | CDC série + authentification | égaliseur, éclairage |
| 3267 | Sound Blaster G3 [S] | `G3` | CDC série + authentification | idem |
| 3268 | Sound Blaster G4 [S] | `G4` | CDC série + authentification | idem |
| 3278 | Sound Blaster X4 [S] | `X4` | CDC série + authentification | idem |
| 3227 | Sound Blaster EVO (USB) | EVO_USB | HID (voyant on/off) | voyant |
| 3228 | Sound Blaster EVO Wireless Lite | EVO_WIRELESS_LITE | HID | voyant |
| 3241 | Sound Blaster JAM [S] | SBJam | non analysé | - |
| 30C8 | Sound Blaster FRee [S] | SBFree | non analysé | - |
| 3245 / 3249 | Sound BlasterX G1 (PC / PS4) [S] | SBXG1 / SBXG1_PS4 | non analysé | - |
| 30E3 | Sound Blaster iRoar Go [S] | iRoarGo | pile « SoundCore » (Bluetooth/USB) | - |
| 1102:0051 / 1102:0191 | Sound BlasterX AE-5 / AE-5 Plus (PCIe, sous-systèmes HDA) | AE5 / AE5Plus | pilote noyau + `CTDrvLedMgr` (bande « interne » / « externe ») | RGB (pas du HID USB) |
| 1102:0071..0074, 0081 | AE-9 / AE-9 PE / AE-7 (PCIe) | AE9 / AE9PE / AE7 | pilote | RGB (pas du HID USB) |

Non retrouvés dans le logiciel (donc INCONNU) : Pebble, Pebble Plus/V3, Outlier, Aurvana, Sound Blaster Play!, Sound Blaster GC (GC7/GC8), Katana V2/V2X (PID 0x3283 selon [S] : <https://blog.nns.ee/2026/02/20/katana-v2x-re>).
Outlier/Aurvana sont des appareils Bluetooth/Airoha (bibliothèques `AirohaHidCoreLib`/`AirohaPeqLibrary` dans Creative App), pas des cibles RGB USB.

### À ajouter au catalogue (VID 041e, à intégrer par le propriétaire)
`["041e","3247","Sound BlasterX Katana"]`, `["041e","30ca","Sound BlasterX Kratos S5"]`, `["041e","3255","Sound BlasterX H6"]`, `["041e","3256","Sound BlasterX G6"]`,
`["041e","3126","Sound BlasterX Vanguard K08 (US)"]`, `["041e","3128","Sound BlasterX Vanguard K08 (DE)"]`, `["041e","3129","Sound BlasterX Vanguard K08 (Nordique)"]`,
`["041e","3127","Sound BlasterX Siege M04"]`, `["041e","3258","SXFI AIR"]`, `["041e","3253","SXFI AIR Theater"]`, `["041e","3262","SXFI AIR C"]`, `["041e","3269","SXFI GAMER"]`,
`["041e","3e10","SXFI AIR GAMER"]`, `["041e","6007","SXFI AMP"]`, `["041e","3264","Sound Blaster X3"]`, `["041e","3267","Sound Blaster G3"]`, `["041e","3268","Sound Blaster G4"]`,
`["041e","3278","Sound Blaster X4"]`, `["041e","3227","Sound Blaster EVO"]`, `["041e","3228","Sound Blaster EVO Wireless Lite"]`.
Le signalrgb-devices.csv donne aussi Katana = `0x3247`, interface HID n° 4 [S].

## 3. Ce que le logiciel expose

- **Katana / Kratos S5 / G6 / iRoar Go / X3 / G3 / G4 / X4 / SXFI AMP** : les réglages sonores (SBX Surround, Crystalizer, Smart Volume, Dialog+, basses,
  égaliseur graphique, profils) passent par la pile « SoundCore » du pilote audio Creative (`Interop.CtSndCr`, propriétés d'endpoint Windows), pas par des
  rapports HID (`Creative.Platform.Devices.Features.SoundCore*`) [V]. Ils exigent le pilote Creative : **rien à implémenter côté HID**.
- **Éclairage Katana / Kratos S5** : motifs `None, ColourCycle, SpectrumAnalyzer, Static, Wave, Pulsate, Blink, Morph, Aurora, Cosmic(1000), WaveCycleBounce(1001),
  Impact(1002), PeakMeterSpectrum(2000)`, directions (gauche-droite, droite-gauche, haut-bas, bas-haut, horaire, antihoraire), boucle/rebond, durée, 7 couleurs par groupe,
  fente de profil (« slot ») active, nom de fente [V].
- **SXFI / X3 / G3 / G4 / X4** : égaliseur + éclairage par port série CDC, protégé (voir §5).
- Ordres génériques côté HID (`ICTHIDRpA`) : micro (boost, AEC), voyant on/off (EVO seulement), batterie, version de firmware, profils.

## 4. Protocole « CTSPP » sur HID (Katana, Kratos S5, G6, iRoar Go) [V]

`CTHIDRpA.dll` (`SetDeviceControlParam`) route les quatre PID 3247, 30CA, 3256 et 30E3 vers la classe qui construit des paquets **CTSPP** (même format que
les appareils Bluetooth, `CCTHIDRpA::SetDeviceCTSPPControlParam`). Transport : rapport de sortie HID de **0x41 octets** (identifiant de rapport + 64), réponse lue sur un rapport de 0x41 octets.
Format [V] : `[ID rapport][0x5A][commande][longueur][charge utile…]` (identifiant de rapport lu dans l'objet appareil, valeur exacte **INCONNUE** sans appareil ; 0x00 probable).
Exemples observés dans le code :

| Message logiciel | Trame après `5A` |
|---|---|
| SetControlPermission | `06 01 01` (prise de contrôle par l'hôte) |
| SetCreativeLedState (LED maître on/off) | `3A 02 06 <état>` |
| SetCreativeLedActiveSlot | `3A 02 0F <fente>` |
| SetCreativeLedPattern | `3A 03 08 <fente> <motif>` |
| SetCreativeLedPatternTiming | `3A 06 0A <cible> <fente/motif> 03 <période ms LE16 = 60000/cpm>` |
| SetCreativeLedColour | `3A <len> 04 <cible 0/1> <fente(1) ou motif(0)> <nb groupes> {…}` puis blocs `3A <len> 0A …` (couleurs 4 octets) |
| Copy/Swap/Restore fente | `3A 03 19|1A|1B <a> <b>` |
| Nom de fente | `3A <len> 15 <fente> <nb> <offset> <UTF-16>` |
| RestartCreativeLedPattern | `3A 01 0C` |
| SetActiveProfile / RestoreProfile / SetProfileData | `1A 03…` / `1C 03 01…` / `17…` |
| FactoryReset | `9B…` |

« cible » : 1 = fente de profil (`DEFINITION_CreativeLEDTarget_LedSlotProfile`), 0 = motif actif (`..._LedPattern`) [V, noms des rappels].

## 5. Ce qui n'est PAS implémenté, et pourquoi

1. **Éclairage Katana / Kratos** : tout le chemin officiel écrit dans des **fentes de profil de l'appareil** (couleurs, motif, durée par fente, nom de fente) ;
   le logiciel ne montre aucun mode « temps réel » en RAM pour ces appareils (la pile LED temps réel `CTDrvLedMgr`/`ClplLEDController` vise les cartes PCIe AE-5/AE-9
   via le pilote noyau, pas l'USB). Le caractère volatil des écritures n'est pas prouvable sans appareil : la règle « aucune écriture de profil » l'interdit.
   **INCONNU** : volatilité de `SetCreativeLedState` / cible 0, identifiant de rapport, interface HID exacte (SignalRGB indique l'interface 4 [S]).
2. **Réglages sonores** : portés par le pilote Creative (voir §3), pas par HID : rien de natif possible sans ce pilote.
3. **SXFI AIR / Theater / C / GAMER / AMP / X3 / G3 / G4 / X4 / Katana V2** : port série CDC avec échange défi-réponse (`CTCDC.dll`, clé dérivée d'octets statiques de la DLL,
   voir [S] <https://blog.nns.ee/2026/02/20/katana-v2x-re>). Le reproduire reviendrait à contourner un contrôle d'accès : **refusé**.
4. **Vanguard K08 / Siege M04 / H6** : protocoles différents (macros, DPI, « LedV2 ») dont les trames ne sont pas lisibles sans exécuter la pile ; ni appareil ni capture : **INCONNU**.
   [S] `Oscillope/sbx-h6-rgb` (GPL-3.0, capture `.pcapng` de la routine LED du pilote Windows) : H6 = interface HID séparée pour les LED, couleur RVB + luminosité. Licence GPL : non réutilisée.
5. **AE-5 / AE-9** : PCIe (pilote noyau), pas du HID USB. [S] <https://gitlab.com/CalcProgrammer1/OpenRGB/-/merge_requests/3658> (bande LED externe AE-5).

Résultat côté code : aucune famille native ajoutée (aucun protocole USB à la fois [V], RAM et réversible). Aucun fichier du dépôt autre que ce document n'est modifié.

## 6. Pour aller plus loin (avec un appareil)
Brancher une Katana, capturer (USBPcap) le logiciel officiel allumant/éteignant le LED maître et changeant la couleur de la fente active, comparer aux trames du §4,
et vérifier la volatilité (débrancher/rebrancher) avant d'écrire une famille `creative_spp` (probe 3247/30CA, `set_setting` « LED maître » en premier).
