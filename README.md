# PureRGB

Contrôle unifié de l'éclairage RGB/ARGB et de la ventilation sous Windows 10/11 : cartes mères, RAM, GPU, claviers, souris, bandes LED, hubs, AIO / watercooling.

![Version](https://img.shields.io/badge/version-0.4.0-orange) ![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-blue)

## Fonctionnalités

- **900+ appareils** via [OpenRGB](https://openrgb.org) **embarqué** : l'installeur inclut OpenRGB **1.0rc3** (support matériel 2024-2026 : Corsair iCUE Link, contrôleurs récents…), démarré automatiquement en arrière-plan (serveur SDK, port 6742). L'exe portable le télécharge au premier besoin (release officielle, SHA-256 vérifié). Un OpenRGB déjà lancé est réutilisé, jamais doublé.
- **RAM et carte mère** : PureRGB s'exécute en administrateur et installe le driver signé [PawnIO](https://pawnio.eu) (SHA-256 vérifié) — indispensable à l'accès SMBus qu'OpenRGB 1.0rc utilise pour la RAM RGB et de nombreuses cartes mères.
- **Gestion des conflits intégrée** (onglet Conflits) : stoppe et/ou désactive au démarrage les services et processus constructeur qui verrouillent le matériel (iCUE, CAM, Armoury Crate, Mystic Light, RGB Fusion, Synapse, G HUB, SignalRGB…), avec restauration en un clic (mode de démarrage d'origine sauvegardé).
- **Drivers natifs expérimentaux** (USB direct, sans OpenRGB) :
  - Corsair Lighting Node Pro / Core (LED)
  - NZXT HUE 2, Smart Device V2, RGB & Fan Controller (LED + **ventilateurs PWM**)
- **Détection automatique** du matériel RGB connu (Corsair, NZXT, ASUS, MSI, Gigabyte, Razer, Logitech, SteelSeries, HyperX, Lian Li, Thermaltake, Cooler Master, DeepCool, EVGA)
- **9 effets** : fixe, respiration, arc-en-ciel (cycle/vague), vague bicolore, comète, clignotement, dégradé, éteint — couleur(s), vitesse, luminosité, sens
- **Contrôle ventilateurs** : vitesse fixe par canal (plancher 20 % anti-arrêt pompe)
- **Anti-conflit** : détecte iCUE, CAM, Armoury Crate, Mystic Light, RGB Fusion, Synapse, G HUB, SignalRGB… et prévient avant tout accès concurrent au matériel
- **Léger** : moteur d'effets à tick adaptatif — un effet statique = **0 % CPU** (le thread dort), FPS animations réglable 5–60
- Profils persistants, tray Windows, instance unique, démarrage minimisé

## Installation

Deux formats dans les [Releases](../../releases) :

| Fichier | Usage |
|---|---|
| `PureRGB_x.y.z_x64-setup.exe` | Installeur NSIS (par utilisateur) |
| `PureRGB.exe` | Portable — un seul fichier, aucune installation |

L'application demande les droits **administrateur** au lancement (UAC) : requis pour l'accès SMBus (RAM, carte mère) et la gestion des services en conflit.

## Démarrage rapide

1. Lancer PureRGB — OpenRGB embarqué démarre tout seul (ou cliquer « Démarrer OpenRGB » dans le bandeau)
2. **Scanner** → choisir un appareil, un effet, **Appliquer**

Optionnel : **Drivers natifs** dans Réglages pour piloter Corsair Node / NZXT HUE2 en USB direct sans OpenRGB (fermer iCUE/CAM d'abord).

## Anti-conflit — comment ça marche

- Le scan HID n'ouvre **aucun** handle : énumération seule, les logiciels constructeur gardent la main.
- Un handle USB n'est ouvert que si les drivers natifs sont activés **et** qu'une couleur est appliquée.
- Les logiciels RGB actifs sont listés en bandeau d'avertissement au lancement et à chaque scan.
- OpenRGB gère lui-même les mutex SMBus (`Global\Access_SMBUS.HTP.Method`) pour la RAM et les cartes mères.

## Build

```bash
npm install
powershell -File scripts/fetch-openrgb.ps1   # récupère OpenRGB à embarquer
npx tauri build   # produit setup NSIS + exe dans src-tauri/target/release
```

Prérequis : Rust stable, Node 20+, Windows 10/11.

## Stack

Tauri v2 (Rust) + Vue 3 + TypeScript. Backends : client TCP OpenRGB SDK (protocole v1), hidapi.

## Licence

PureRGB : MIT — les protocoles matériels natifs sont réimplémentés d'après la documentation publique des projets OpenRGB et liquidctl (aucun code GPL inclus dans PureRGB).

OpenRGB est distribué en binaire séparé, non modifié, sous licence **GPLv2** — © CalcProgrammer1 et contributeurs, sources : https://gitlab.com/CalcProgrammer1/OpenRGB. PureRGB communique avec lui uniquement par son API réseau SDK (processus distinct).
