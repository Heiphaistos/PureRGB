//! Famille « gigabyte_fusion » : à implémenter d'après docs/rgb-protocols/ (cadre : native/mod.rs).

use super::{Family, Hid, Model, NativeRgb};
use anyhow::{bail, Result};

fn probe(_vid: u16, _pid: u16) -> Option<Model> {
    None
}

fn open(_hid: Box<dyn Hid>, _model: &Model) -> Result<Box<dyn NativeRgb>> {
    bail!("famille gigabyte_fusion pas encore implémentée")
}

pub const FAMILY: Family = Family { id: "gigabyte_fusion", doc: "docs/rgb-protocols/", probe, open };
