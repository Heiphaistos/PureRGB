//! Registre des designs embarqués. Un module par catégorie (`LIST`), un JSON par design.

use super::{generated, themes};

pub struct Builtin {
    pub id: &'static str,
    pub category: &'static str,
    pub tags: &'static [&'static str],
    pub json: &'static str,
}

pub fn all() -> impl Iterator<Item = &'static Builtin> {
    [themes::LIST, generated::LIST].into_iter().flatten()
}
