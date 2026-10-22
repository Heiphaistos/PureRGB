//! Famille « corsair_commander » : à implémenter d'après docs/rgb-protocols/ (cadre : native/mod.rs).

use super::{Family, Hid, Model, NativeRgb};
use anyhow::{bail, Result};

fn probe(_vid: u16, _pid: u16) -> Option<Model> {
    None
}

fn open(_hid: Box<dyn Hid>, _model: &Model) -> Result<Box<dyn NativeRgb>> {
    bail!("famille corsair_commander pas encore implémentée")
}

pub const FAMILY: Family = Family { id: "corsair_commander", doc: "docs/rgb-protocols/", probe, open };
