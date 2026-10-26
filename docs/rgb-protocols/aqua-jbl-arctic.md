# Aqua Computer, JBL Quantum, ARCTIC : analyse (2026-10-26)

[V] lu dans une source/ un code officiel, [S] supposé, INCONNU = non établi. Dossier de travail : `D:\Tools\analyse\aqua-jbl-arctic\`.
Aucune trame n'a été envoyée à un appareil (aucun n'est branché) ; rien n'a été installé sur le PC (JBL : extraction 7-Zip de l'installeur, sans exécution).

## 1. JBL Quantum (VID 0x0ECB, Harman) : logiciel QuantumENGINE 2.8.6

- Source : `https://support.jbl.com/quantum-engine-win-latest` -> `appstorage.onecloud.harman.com/.../JBL_QuantumENGINE_Installer_2.8.6_x64.exe`
  (196 417 832 o, SHA-256 `f3af68849b38234caf0d82130c735bb92e9f6c4bcc354f1e79bfae6421878723`). NSIS, extrait sans installation ; `QuantumServer.dll`
  (.NET, non obfusqué) décompilé avec ILSpy (`jbl/src_server/`). `HIDDevice.dll` n'est qu'une enveloppe hidapi : le protocole est dans le .NET.
- PID (de `driver/QuantumUSBAudio.inf`, noms « Quantum NNN » à préfixer JBL) :

| PID | Nom (INF) |
|---|---|
| 1F46 | Quantum 600 Chat / Quantum 600 Game |
| 1F47 | Quantum 800 Chat / Quantum 800 Game |
| 1F48 | Quantum ONE Chat / Quantum ONE Game |
| 1F4C | Quantum 400 Chat / Quantum 400 Game |
| 1F52 | Quantum 300 |
| 2039 | Quantum 400 Chat / Quantum 400 Game |
| 203A | Quantum ONE Chat / Quantum ONE Game |
| 203C | Quantum 600 Chat / Quantum 600 Game |
| 203E | Quantum 800 Chat / Quantum 800 Game |
| 2047 | Quantum 350 |
| 205C | Quantum 610 Chat / Quantum 610 Game |
| 205F | Quantum Stream |
| 2069 | Quantum 810 Chat / Quantum 810 Game |
| 206B | Quantum 350 |
| 2088 | Quantum 910 Chat / Quantum 910 Game |
| 208A | Quantum TWS |
| 209B | Quantum TWS CN |
| 209C | Quantum 910 wireless for calibration |
| 209D | Quantum 910 Xbox Chat / Quantum 910 Xbox Game |
| 209F | Quantum 910X wireless for calibration |
| 20A2 | Quantum 910 PS Chat / Quantum 910 PS Game |
| 20A4 | Quantum 910P wireless for calibration |
| 20A5 | Quantum 360 Xbox / Quantum 360 Xbox Chat / Quantum 360 Xbox Game |
| 20AB | Quantum 360 PS / Quantum 360 PS Chat / Quantum 360 PS Game |
| 20AF | Quantum Stream Talk |
| 20B0 | Quantum Stream Studio |
| 20B2 | Quantum 350 |
| 20BE | Quantum TWS AIR |
| 20C2 | Quantum ONE Chat / Quantum ONE Game |
| 20ED | Quantum Stream Wireless |
| 20EE | Quantum 400 Chat / Quantum 400 Game |
| 2112 | Quantum 50C |
| 2115 | Quantum 360 / Quantum 360 Chat / Quantum 360 Game |
| 2125 | Quantum 650 Dongle Chat / Quantum 650 Dongle Game |
| 2126 | Quantum 650 Headset Chat / Quantum 650 Headset Game |
| 2127 | Quantum 950 Dongle Chat / Quantum 950 Dongle Game |
| 2128 | Quantum 950 Headset Chat / Quantum 950 Headset Game |
| 2129 | Quantum 950 wireless for calibration |
| 213B | Quantum 650X Dongle Chat / Quantum 650X Dongle Game |
| 213D | Quantum 650X Headset Chat / Quantum 650X Headset Game |
| 213E | Quantum 950X Dongle Chat / Quantum 950X Dongle Game |
| 2140 | Quantum 950X Headset Chat / Quantum 950X Headset Game |
| 2141 | Quantum 950X wireless for calibration |
| 2142 | Quantum 360 Xbox Black / Quantum 360 Xbox Black Chat / Quantum 360 Xbox Black Game |
| 2147 | Quantum 910 Xbox Black Chat / Quantum 910 Xbox Black Game / Quantum 910 Xbox Game |
| 214A | Quantum 910X Black wireless for calibration |
| 218D | Quantum 520 Dongle Chat / Quantum 520 Dongle Game |
| 218E | Quantum 520 Headset Chat / Quantum 520 Headset Game |
| 219A | Quantum 850 Dongle Chat / Quantum 850 Dongle Game |
| 219B | Quantum 850 Headset Chat / Quantum 850 Headset Game |
| 219C | Quantum 850 wireless for calibration |
| 21A7 | Quantum 520X Dongle Chat / Quantum 520X Dongle Game |
| 21A9 | Quantum 520X Headset Chat / Quantum 520X Headset Game |
  Le dongle, le casque en USB direct et l'interface d'étalonnage ont des PID distincts (ex. Q810 dongle 2069, casque 206A ; Q910 dongle 2088, casque 2089,
  étalonnage 209C) ; le serveur lie le contrôle HID au PID « dongle ». Ids décimaux du serveur : Q610 8284, Q810 8297, Q910 8328, Q910P 8354, Q910X 8349, Q910XB 8519.

### Trois dialectes HID (le logiciel les appelle HIDV1/V2/V3)
- **V1** : rapports de fonctionnalité dont l'ID de rapport = ID de message, charge utile = struct empaquetée (1 octet pour un niveau). Collection choisie
  par page d'usage **0xFF13** (Q610, Q810, Q910*) ou par sonde (rapport `50` lu avec succès ; Q300/350/360/400/600/800/ONE, Stream).
- **V2** (page 0xFF12, Stream Wireless) et **V3** (page 0xFF12, Q520/650/850/950) : non décodés ici (INCONNU), aucun usage.
- Réglages V1 relevés [V] (get/set/entrée) : sidetone `5C/5D/5C` niveau 0..3 (Off/Faible/Moyen/Fort) ; éclairage actif `4A/4B/07` (1 octet) ;
  animation `4C` + images clés `4D` par groupe de LED (jusqu'à 10 images) ; version matérielle `50`.
- **Implémenté** : sidetone V1 des récepteurs Q610 (205C), Q810 (2069), Q910 (2088), Q910X (209D), Q910P (20A2), Q910XB (2147) : lecture `5C`,
  écriture `5D <niveau>`. Module `jbl_quantum.rs`, 4 tests MockHid.
- **[S] persistance** : rien ne dit si le casque garde le niveau. Garde-fous : lecture obligatoire avant la 1re écriture, niveau d'origine remis à la fermeture
  (Drop/release). A valider sur matériel avant de promouvoir.
- **Refusé** : éclairage (une animation + images clés sont téléversées dans le casque, même lecture possible : persistance probable, non prouvée),
  EQ, profils, anti-bruit, mise à jour (FOTA), réinitialisation d'usine. Sidetone des Q300/Q50C : logiciel (hors appareil). Q350/360/400/600/800/ONE : même
  `5C/5D` dans le code mais collection inconnue sans sonde `50` ; non revendiqués.

## 2. Aqua Computer (VID 0x0C70, « MCT Elektronikladen » dans usb.ids)

- PID [V] (pilote noyau Linux `aquacomputer_d5next.c` + liquidctl) : F001 aquaero 5/6, F003 High Flow, F00A Farbwerk, F00B Aquastream Ultimate,
  F00D Quadro, F00E D5 Next, F010 Farbwerk 360, F011 Octo, F012 High Flow Next, F014 Leakshield, F0B6 Aquastream XT, F0BD Poweradjust 3.
- Vitesse ventilateurs [V] : rapport de fonctionnalité `03` (D5 Next 0x329 o, Quadro 0x3C1, Octo 0x65F, Farbwerk 360 0x682) = TOUTE la configuration
  de l'appareil ; l'écriture relit le rapport, change type/pourcentage du canal (centi-pourcent, grand-boutiste), recalcule le CRC-16/USB, renvoie le tout, puis
  le logiciel officiel envoie toujours `02 00 00 00 02 00 00 00 00 34 C6` (aquaero : `06 00 02 00 00 00 00`).
- **REFUSÉ** : cette réécriture est celle de la configuration complète (courbes, noms, alarmes) suivie du rapport « d'application » du logiciel :
  persistance dans l'appareil NON écartée, aucune preuve de RAM seule, et pas de matériel pour tester. Pas d'implémentation ventilateur (règle : jamais sans
  preuve).
- Éclairage Farbwerk/Farbwerk 360/Aquastream : INCONNU (liquidctl « not yet reverse engineered », le pilote noyau n'a que les capteurs). Les offsets
  de couleur dans le rapport `03` ne sont publiés nulle part.
- aquasuite : `aquasuite_setup.exe` (SHA-256 `b6cffe88…0b60`, 1 277 280 o) n'est qu'un TÉLÉCHARGEUR .NET fortement obfusqué (anti-altération, détection de
  débogueur) qui récupère le vrai paquet à l'exécution ; son adresse n'est pas dans les pages du site. Je n'ai pas contourné la protection ni piloté son
  interface : le paquet aquasuite n'a pas été obtenu. Il a été lancé quelques secondes puis tué (rien installé, aucun pilote ajouté).

## 3. ARCTIC

- Liquid Freezer III A-RGB / P-series : éclairage adressable par en-tête carte mère ; pas de logiciel propre identifié (SignalRGB pour la série Pro, d'après la presse).
  « ARCTIC RGB Control Master » (Windows) existe pour P12 PWM PST RGB / Liquid Freezer II avec pilote CH341 : contrôleur USB-SÉRIE, donc hors du cadre HID
  `native` (déjà documenté dans `autres-refroidissement.md` : CH340 1A86:7523, 250 000 bauds). Rien d'implémenté, aucune analyse du logiciel (non téléchargé).
