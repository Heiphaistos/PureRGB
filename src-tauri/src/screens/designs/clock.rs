//! Designs « Horloge » : un fichier JSON par design dans `clock/`.

use super::builtin::Builtin;

pub const LIST: &[Builtin] = &[
    Builtin {
        id: "clock-classique-480",
        category: "Horloge",
        tags: &["horloge", "analogique", "classique", "cpu", "gpu"],
        json: include_str!("clock/classique_480.json"),
    },
    Builtin {
        id: "clock-neon-480",
        category: "Horloge",
        tags: &["horloge", "numérique", "néon", "cyber", "cpu", "gpu"],
        json: include_str!("clock/neon_480.json"),
    },
    Builtin {
        id: "clock-segments-480",
        category: "Horloge",
        tags: &["horloge", "segments", "lcd", "pixel", "cpu", "gpu"],
        json: include_str!("clock/segments_480.json"),
    },
    Builtin {
        id: "clock-minimal-480",
        category: "Horloge",
        tags: &["horloge", "minimal", "clair", "numérique", "cpu", "gpu"],
        json: include_str!("clock/minimal_480.json"),
    },
    Builtin {
        id: "clock-anneau-480",
        category: "Horloge",
        tags: &["horloge", "analogique", "anneaux", "jauges", "cpu", "gpu"],
        json: include_str!("clock/anneau_480.json"),
    },
    Builtin {
        id: "clock-panorama-1280",
        category: "Horloge",
        tags: &["horloge", "analogique", "numérique", "large", "cpu", "gpu"],
        json: include_str!("clock/panorama_1280.json"),
    },
    Builtin {
        id: "clock-bandeau-1920",
        category: "Horloge",
        tags: &["horloge", "numérique", "large", "sobre", "cpu", "gpu"],
        json: include_str!("clock/bandeau_1920.json"),
    },
    Builtin {
        id: "clock-ruban-640",
        category: "Horloge",
        tags: &["horloge", "numérique", "ruban", "néon", "cpu", "gpu"],
        json: include_str!("clock/ruban_640.json"),
    },
    Builtin {
        id: "clock-retro-320",
        category: "Horloge",
        tags: &["horloge", "analogique", "rétro", "ambre", "pixel"],
        json: include_str!("clock/retro_320.json"),
    },
];
