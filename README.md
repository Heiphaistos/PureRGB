# PureRGB

Contrôle unifié de l'éclairage RGB/ARGB et de la ventilation sous Windows 10/11 : cartes mères, RAM, GPU, claviers, souris, bandes LED, hubs, AIO / watercooling.

![Version](https://img.shields.io/badge/version-0.1.0-orange) ![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-blue)

## Fonctionnalités

- **900+ appareils** via le pont [OpenRGB SDK](https://openrgb.org) (serveur local, port 6742)
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
| `PureRGB_x.y.z_x64-setup.exe` | Installeur NSIS (par utilisateur, sans admin) |
| `PureRGB.exe` | Portable — un seul fichier, aucune installation |

## Démarrage rapide

1. Installer [OpenRGB](https://openrgb.org/releases.html), le lancer, activer **SDK Server**
2. Lancer PureRGB → **Scanner**
3. Choisir un appareil, un effet, **Appliquer**

Sans OpenRGB : cocher **Drivers natifs** dans Réglages (matériel Corsair Node / NZXT HUE2 uniquement, fermer iCUE/CAM d'abord).

## Anti-conflit — comment ça marche

- Le scan HID n'ouvre **aucun** handle : énumération seule, les logiciels constructeur gardent la main.
- Un handle USB n'est ouvert que si les drivers natifs sont activés **et** qu'une couleur est appliquée.
- Les logiciels RGB actifs sont listés en bandeau d'avertissement au lancement et à chaque scan.
- OpenRGB gère lui-même les mutex SMBus (`Global\Access_SMBUS.HTP.Method`) pour la RAM et les cartes mères.

## Build

```bash
npm install
npx tauri build   # produit setup NSIS + exe dans src-tauri/target/release
```

Prérequis : Rust stable, Node 20+, Windows 10/11.

## Stack

Tauri v2 (Rust) + Vue 3 + TypeScript. Backends : client TCP OpenRGB SDK (protocole v1), hidapi.

## Licence

MIT — les protocoles matériels natifs sont réimplémentés d'après la documentation publique des projets OpenRGB et liquidctl (aucun code GPL inclus).
