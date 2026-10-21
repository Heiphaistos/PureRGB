//! Designs « Gaming » : un fichier JSON par design dans `gaming/`.

use super::builtin::Builtin;

pub const LIST: &[Builtin] = &[
    Builtin {
        id: "gaming-tachymetre",
        category: "Gaming",
        tags: &["gpu", "cpu", "jauge", "heure", "carré"],
        json: include_str!("gaming/tachometre.json"),
    },
    Builtin {
        id: "gaming-fps-hud",
        category: "Gaming",
        tags: &["gpu", "cpu", "ram", "barres", "carré"],
        json: include_str!("gaming/fps_hud.json"),
    },
    Builtin {
        id: "gaming-barres-de-vie",
        category: "Gaming",
        tags: &["cpu", "gpu", "ram", "barres", "carré"],
        json: include_str!("gaming/barres_de_vie.json"),
    },
    Builtin {
        id: "gaming-synthwave",
        category: "Gaming",
        tags: &["cpu", "gpu", "heure", "rétro", "carré"],
        json: include_str!("gaming/synthwave.json"),
    },
    Builtin {
        id: "gaming-cockpit",
        category: "Gaming",
        tags: &["cpu", "gpu", "ram", "heure", "jauge", "carré"],
        json: include_str!("gaming/cockpit.json"),
    },
    Builtin {
        id: "gaming-neon-large",
        category: "Gaming",
        tags: &["cpu", "gpu", "ram", "courbe", "large"],
        json: include_str!("gaming/neon_large.json"),
    },
    Builtin {
        id: "gaming-hud-bandeau",
        category: "Gaming",
        tags: &["cpu", "gpu", "ram", "courbe", "heure", "large"],
        json: include_str!("gaming/hud_bandeau.json"),
    },
    Builtin {
        id: "gaming-mini-320",
        category: "Gaming",
        tags: &["gpu", "cpu", "ram", "petit"],
        json: include_str!("gaming/mini_320.json"),
    },
    Builtin {
        id: "gaming-bandeau-640",
        category: "Gaming",
        tags: &["cpu", "gpu", "ram", "heure", "large"],
        json: include_str!("gaming/bandeau_640.json"),
    },
];
