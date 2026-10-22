//! Famille « asrock_polychrome » : à implémenter d'après docs/rgb-protocols/ (cadre : native/mod.rs).

use super::{Family, Hid, Model, NativeRgb};
use anyhow::{bail, Result};

fn probe(_vid: u16, _pid: u16) -> Option<Model> {
    None
}

fn open(_hid: Box<dyn Hid>, _model: &Model) -> Result<Box<dyn NativeRgb>> {
    bail!("famille asrock_polychrome pas encore implémentée")
}

pub const FAMILY: Family = Family { id: "asrock_polychrome", doc: "docs/rgb-protocols/", probe, open };
