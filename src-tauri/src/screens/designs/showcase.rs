//! Designs « Vitrine » : un fichier JSON par design dans `showcase/`.

use super::builtin::Builtin;

pub const LIST: &[Builtin] = &[
    Builtin {
        id: "showcase-anneaux-connectes",
        category: "Vitrine",
        tags: &["anneaux", "montre", "cpu", "gpu", "ram", "ssd", "heure"],
        json: include_str!("showcase/anneaux_connectes.json"),
    },
    Builtin {
        id: "showcase-jauge-geante",
        category: "Vitrine",
        tags: &["jauge", "plein écran", "cpu", "température"],
        json: include_str!("showcase/jauge_geante.json"),
    },
    Builtin {
        id: "showcase-egaliseur-foret",
        category: "Vitrine",
        tags: &["égaliseur", "barres", "forêt", "charge"],
        json: include_str!("showcase/egaliseur_foret.json"),
    },
    Builtin {
        id: "showcase-crt-vert",
        category: "Vitrine",
        tags: &["rétro", "crt", "vert", "terminal"],
        json: include_str!("showcase/terminal_crt_vert.json"),
    },
    Builtin {
        id: "showcase-glace",
        category: "Vitrine",
        tags: &["glace", "large", "cpu", "gpu", "liquide", "heure"],
        json: include_str!("showcase/glace.json"),
    },
    Builtin {
        id: "showcase-lave",
        category: "Vitrine",
        tags: &["lave", "large", "cpu", "gpu", "barres"],
        json: include_str!("showcase/lave.json"),
    },
    Builtin {
        id: "showcase-aurore",
        category: "Vitrine",
        tags: &["aurore", "petit", "cpu", "gpu", "heure"],
        json: include_str!("showcase/aurore.json"),
    },
    Builtin {
        id: "showcase-aio-liquide",
        category: "Vitrine",
        tags: &["aio", "rond", "liquide", "pompe"],
        json: include_str!("showcase/aio_liquide.json"),
    },
    Builtin {
        id: "showcase-aio-ocean",
        category: "Vitrine",
        tags: &["aio", "rond", "océan", "liquide", "pompe"],
        json: include_str!("showcase/aio_ocean.json"),
    },
];
