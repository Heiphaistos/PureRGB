//! Designs « Moniteur » : un fichier JSON par design dans `monitor/`.

use super::builtin::Builtin;

pub const LIST: &[Builtin] = &[
    Builtin {
        id: "moniteur-duo",
        category: "Moniteur",
        tags: &["cpu", "gpu", "ram", "heure"],
        json: include_str!("monitor/moniteur_duo.json"),
    },
    Builtin {
        id: "monitor-reference",
        category: "Moniteur",
        tags: &["référence", "cpu", "gpu", "ram", "ssd", "heure", "date", "carré"],
        json: include_str!("monitor/reference.json"),
    },
    Builtin {
        id: "monitor-anneaux",
        category: "Moniteur",
        tags: &["anneaux", "cpu", "gpu", "charge", "heure", "carré"],
        json: include_str!("monitor/anneaux.json"),
    },
    Builtin {
        id: "monitor-barres",
        category: "Moniteur",
        tags: &["barres", "cpu", "gpu", "ram", "ssd", "carré"],
        json: include_str!("monitor/barres.json"),
    },
    Builtin {
        id: "monitor-courbes",
        category: "Moniteur",
        tags: &["courbes", "historique", "charge", "cpu", "gpu", "carré"],
        json: include_str!("monitor/courbes.json"),
    },
    Builtin {
        id: "monitor-aio",
        category: "Moniteur",
        tags: &["aio", "liquide", "pompe", "ventilateur", "température", "carré"],
        json: include_str!("monitor/aio.json"),
    },
    Builtin {
        id: "monitor-reseau",
        category: "Moniteur",
        tags: &["réseau", "débit", "courbes", "carré"],
        json: include_str!("monitor/reseau.json"),
    },
    Builtin {
        id: "monitor-large-1280",
        category: "Moniteur",
        tags: &["large", "jauges", "cpu", "gpu", "ram", "ssd", "heure"],
        json: include_str!("monitor/large-1280.json"),
    },
    Builtin {
        id: "monitor-ultra-1920",
        category: "Moniteur",
        tags: &["ultra-large", "bandeau", "cpu", "gpu", "ram", "ssd", "heure"],
        json: include_str!("monitor/ultra-1920.json"),
    },
    Builtin {
        id: "monitor-compact-320",
        category: "Moniteur",
        tags: &["compact", "petit écran", "cpu", "gpu", "heure"],
        json: include_str!("monitor/compact-320.json"),
    },
    Builtin {
        id: "monitor-reference-1920",
        category: "Moniteur",
        tags: &["référence", "trofeo vision", "large", "cpu", "gpu", "ram", "ssd", "heure", "date", "fond libre"],
        json: include_str!("monitor/monitor_reference_1920.json"),
    },
];
