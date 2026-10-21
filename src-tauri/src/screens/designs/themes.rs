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
];
