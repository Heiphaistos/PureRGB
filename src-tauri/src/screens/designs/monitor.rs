//! Designs « Moniteur » : un fichier JSON par design dans `monitor/`.

use super::builtin::Builtin;

pub const LIST: &[Builtin] = &[
    Builtin {
        id: "moniteur-duo",
        category: "Moniteur",
        tags: &["cpu", "gpu", "ram", "heure"],
        json: include_str!("monitor/moniteur_duo.json"),
    },
];
