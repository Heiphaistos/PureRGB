//! Registre des designs embarqués. Un module par catégorie (`LIST`), un JSON par design.

use super::{clock, gaming, generated, minimal, monitor, showcase};

pub struct Builtin {
    pub id: &'static str,
    pub category: &'static str,
    pub tags: &'static [&'static str],
    pub json: &'static str,
}

pub fn all() -> impl Iterator<Item = &'static Builtin> {
    [monitor::LIST, gaming::LIST, minimal::LIST, clock::LIST, showcase::LIST, generated::LIST].into_iter().flatten()
}
