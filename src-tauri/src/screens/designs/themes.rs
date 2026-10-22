//! Thèmes vidéo intégrés (générés par `tools/themes/forge_themes.py`) : fond = vidéo embarquée
//! (`builtin_media`), socle CPU / GPU / RAM / SSD 1-2 + heure et date.

use super::builtin::Builtin;

pub const LIST: &[Builtin] = &[
    Builtin {
        id: "theme-forge-droite-u1920",
        category: "Thèmes vidéo",
        tags: &["thème", "vidéo", "forge", "cpu", "gpu", "ram", "ssd", "heure", "date", "u1920", "ultra-large", "fond libre"],
        json: include_str!("themes/theme-forge-droite-u1920.json"),
    },
    Builtin {
        id: "theme-forge-bandeau-u1920",
        category: "Thèmes vidéo",
        tags: &["thème", "vidéo", "forge", "cpu", "gpu", "ram", "ssd", "heure", "date", "u1920", "ultra-large", "bandeau"],
        json: include_str!("themes/theme-forge-bandeau-u1920.json"),
    },
    Builtin {
        id: "theme-forge-coins-u1920",
        category: "Thèmes vidéo",
        tags: &["thème", "vidéo", "forge", "cpu", "gpu", "ram", "ssd", "heure", "date", "u1920", "ultra-large", "coins"],
        json: include_str!("themes/theme-forge-coins-u1920.json"),
    },
    Builtin {
        id: "theme-forge-droite-w1280",
        category: "Thèmes vidéo",
        tags: &["thème", "vidéo", "forge", "cpu", "gpu", "ram", "ssd", "heure", "date", "w1280", "large"],
        json: include_str!("themes/theme-forge-droite-w1280.json"),
    },
    Builtin {
        id: "theme-forge-bandeau-w1280",
        category: "Thèmes vidéo",
        tags: &["thème", "vidéo", "forge", "cpu", "gpu", "ram", "ssd", "heure", "date", "w1280", "large", "bandeau"],
        json: include_str!("themes/theme-forge-bandeau-w1280.json"),
    },
    Builtin {
        id: "theme-forge-carre-s480",
        category: "Thèmes vidéo",
        tags: &["thème", "vidéo", "forge", "cpu", "gpu", "ram", "ssd", "heure", "date", "s480", "carré"],
        json: include_str!("themes/theme-forge-carre-s480.json"),
    },
    Builtin {
        id: "theme-forge-gauche-u1920",
        category: "Thèmes vidéo",
        tags: &["thème", "vidéo", "forge", "cpu", "gpu", "ram", "ssd", "heure", "date", "u1920", "ultra-large", "fond libre"],
        json: include_str!("themes/theme-forge-gauche-u1920.json"),
    },
    Builtin {
        id: "theme-forge-haut-u1920",
        category: "Thèmes vidéo",
        tags: &["thème", "vidéo", "forge", "cpu", "gpu", "ram", "ssd", "heure", "date", "u1920", "ultra-large", "bandeau"],
        json: include_str!("themes/theme-forge-haut-u1920.json"),
    },
    Builtin {
        id: "theme-forge-colonnes-u1920",
        category: "Thèmes vidéo",
        tags: &["thème", "vidéo", "forge", "cpu", "gpu", "ram", "ssd", "heure", "date", "u1920", "ultra-large", "colonnes"],
        json: include_str!("themes/theme-forge-colonnes-u1920.json"),
    },
    Builtin {
        id: "theme-forge-ambre-droite-u1920",
        category: "Thèmes vidéo",
        tags: &["thème", "vidéo", "forge", "cpu", "gpu", "ram", "ssd", "heure", "date", "u1920", "ultra-large", "ambre"],
        json: include_str!("themes/theme-forge-ambre-droite-u1920.json"),
    },
    Builtin {
        id: "theme-forge-cyan-bandeau-u1920",
        category: "Thèmes vidéo",
        tags: &["thème", "vidéo", "forge", "cpu", "gpu", "ram", "ssd", "heure", "date", "u1920", "ultra-large", "cyan"],
        json: include_str!("themes/theme-forge-cyan-bandeau-u1920.json"),
    },
    Builtin {
        id: "theme-forge-rose-coins-u1920",
        category: "Thèmes vidéo",
        tags: &["thème", "vidéo", "forge", "cpu", "gpu", "ram", "ssd", "heure", "date", "u1920", "ultra-large", "rose"],
        json: include_str!("themes/theme-forge-rose-coins-u1920.json"),
    },
    Builtin {
        id: "theme-forge-vert-colonnes-u1920",
        category: "Thèmes vidéo",
        tags: &["thème", "vidéo", "forge", "cpu", "gpu", "ram", "ssd", "heure", "date", "u1920", "ultra-large", "vert"],
        json: include_str!("themes/theme-forge-vert-colonnes-u1920.json"),
    },
    Builtin {
        id: "theme-forge-vert-haut-u1920",
        category: "Thèmes vidéo",
        tags: &["thème", "vidéo", "forge", "cpu", "gpu", "ram", "ssd", "heure", "date", "u1920", "ultra-large", "vert"],
        json: include_str!("themes/theme-forge-vert-haut-u1920.json"),
    },
    Builtin {
        id: "theme-forge-ambre-haut-u1920",
        category: "Thèmes vidéo",
        tags: &["thème", "vidéo", "forge", "cpu", "gpu", "ram", "ssd", "heure", "date", "u1920", "ultra-large", "ambre"],
        json: include_str!("themes/theme-forge-ambre-haut-u1920.json"),
    },
    Builtin {
        id: "theme-forge-cyan-gauche-w1280",
        category: "Thèmes vidéo",
        tags: &["thème", "vidéo", "forge", "cpu", "gpu", "ram", "ssd", "heure", "date", "w1280", "large", "cyan"],
        json: include_str!("themes/theme-forge-cyan-gauche-w1280.json"),
    },
    Builtin {
        id: "theme-forge-rose-haut-w1280",
        category: "Thèmes vidéo",
        tags: &["thème", "vidéo", "forge", "cpu", "gpu", "ram", "ssd", "heure", "date", "w1280", "large", "rose"],
        json: include_str!("themes/theme-forge-rose-haut-w1280.json"),
    },
    Builtin {
        id: "theme-forge-colonnes-w1280",
        category: "Thèmes vidéo",
        tags: &["thème", "vidéo", "forge", "cpu", "gpu", "ram", "ssd", "heure", "date", "w1280", "large", "colonnes"],
        json: include_str!("themes/theme-forge-colonnes-w1280.json"),
    },
];
