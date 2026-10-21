//! Mini-écrans LCD (boîtiers, AIO, ventirads) : détection, rendu de designs,
//! envoi des images. Voir CLAUDE.md et docs/superpowers/specs/2026-10-21-screens-design.md.

pub mod driver;
pub mod layout;
pub mod bindings;
pub mod sensor_map;
pub mod media;
pub mod render;
pub mod drivers;
pub mod commands;
pub mod theme_pack;
pub mod designs;
pub mod manager;
pub mod pacing;
pub mod pipeline;
pub mod runner;
pub mod sim;
pub mod video;
#[cfg(windows)]
mod video_mf;
pub mod state;
#[cfg(test)]
mod perf_bench;
