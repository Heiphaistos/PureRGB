//! Designs « Minimal » : un fichier JSON par design dans `minimal/`.

use super::builtin::Builtin;

pub const LIST: &[Builtin] = &[
    Builtin {
        id: "minimal-cpu-chiffre",
        category: "Minimal",
        tags: &["cpu","carré","sombre","chiffre"],
        json: include_str!("minimal/cpu_chiffre.json"),
    },
    Builtin {
        id: "minimal-gpu-clair",
        category: "Minimal",
        tags: &["gpu","carré","clair","vram"],
        json: include_str!("minimal/gpu_clair.json"),
    },
    Builtin {
        id: "minimal-heure",
        category: "Minimal",
        tags: &["heure","date","carré","sombre"],
        json: include_str!("minimal/heure.json"),
    },
    Builtin {
        id: "minimal-anneau",
        category: "Minimal",
        tags: &["cpu","jauge","carré","noir"],
        json: include_str!("minimal/anneau.json"),
    },
    Builtin {
        id: "minimal-duo-clair",
        category: "Minimal",
        tags: &["cpu","gpu","carré","clair"],
        json: include_str!("minimal/duo_clair.json"),
    },
    Builtin {
        id: "minimal-trio-large",
        category: "Minimal",
        tags: &["cpu","gpu","ram","large","1280x480"],
        json: include_str!("minimal/trio_large.json"),
    },
    Builtin {
        id: "minimal-ligne-claire",
        category: "Minimal",
        tags: &["heure","cpu","gpu","large","clair","1920x462"],
        json: include_str!("minimal/ligne_claire.json"),
    },
    Builtin {
        id: "minimal-ruban",
        category: "Minimal",
        tags: &["heure","cpu","gpu","ruban","640x172"],
        json: include_str!("minimal/ruban.json"),
    },
    Builtin {
        id: "minimal-qvga",
        category: "Minimal",
        tags: &["cpu","gpu","ram","320x240"],
        json: include_str!("minimal/qvga.json"),
    },
];
