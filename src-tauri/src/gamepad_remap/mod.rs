//! Remappage de manette : XInput (manette physique) -> profil (boutons, zones mortes, courbes,
//! gâchettes, anti-dérive) -> manette Xbox 360 virtuelle ViGEmBus. Désactivé par défaut
//! (`gamepad_remap_enabled`). Partie pure : `model.rs` + `logic.rs`.

pub mod commands;
pub mod driver;
pub mod logic;
pub mod model;
mod worker;

pub use worker::{start, stop};
