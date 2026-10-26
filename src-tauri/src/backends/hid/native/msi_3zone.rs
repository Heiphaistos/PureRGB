//! Famille « msi_3zone » : clavier 3 zones des portables MSI (ancien contrôleur SteelSeries, VID 1770 PID FF00).
//! Pilote expérimental, désactivé par défaut. Source : docs/rgb-protocols/oem-portables.md (§ MSI) ; faits de format relevés dans les sources
//! publiques OpenRGB (GPL-2, 2026-10-26, [S]) qui citent le projet public msi-keyboard de bparker06 ; aucun code copié ; MSI Center non analysé
//! (téléchargement officiel refusé par winget : somme de contrôle différente, non contourné).
//!
//! Trame : feature report de 8 octets, identifiant 1 : `01 02 40 <zone 1..3> R G B EC` (0xEC = 236 fixe). Une trame par zone (gauche, milieu, droite).
//! INCONNU : la source envoie aussi les zones 4 à 7 (zone « aux »), rôle non établi, donc NON envoyées ici ; persistance non établie.

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if (vid, pid) != (0x1770, 0xFF00) {
        return None;
    }
    let zones = ["Gauche", "Milieu", "Droite"].map(|n| ZoneInfo::fixed(n, 1)).to_vec();
    Some(Model::new("MSI portable (clavier 3 zones)", DeviceType::Keyboard, zones))
}

struct Msi3 {
    hid: Box<dyn Hid>,
    last: [Option<Color>; 3],
}

fn open(hid: Box<dyn Hid>, _model: &Model) -> Result<Box<dyn NativeRgb>> {
    Ok(Box::new(Msi3 { hid, last: [None; 3] }))
}

fn frame(zone: u8, c: Color) -> [u8; 8] {
    [1, 2, 0x40, zone, c.r, c.g, c.b, 0xEC]
}

impl NativeRgb for Msi3 {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        if colors.len() > 3 {
            bail!("MSI 3 zones : {} couleurs pour 3 zones", colors.len());
        }
        for z in 0..3 {
            let c = colors.get(z).copied().unwrap_or(Color::BLACK);
            if self.last[z] != Some(c) {
                self.hid.send_feature_report(&frame(z as u8 + 1, c))?;
                self.last[z] = Some(c);
            }
        }
        Ok(())
    }
}

pub const FAMILY: Family = Family { id: "msi_3zone", doc: "docs/rgb-protocols/oem-portables.md", probe, open };

#[cfg(test)]
mod tests {
    use super::super::MockHid;
    use super::*;

    #[test]
    fn probe_and_frames() {
        assert!(probe(0x1770, 0xFF01).is_none());
        let model = probe(0x1770, 0xFF00).expect("pid");
        assert_eq!(model.led_count(), 3);
        let (mock, _, feat) = MockHid::new(vec![]);
        let mut d = open(Box::new(mock), &model).expect("open");
        d.set_colors(&[Color::new(9, 8, 7), Color::new(1, 2, 3)]).expect("set");
        d.set_colors(&[Color::new(9, 8, 7), Color::new(1, 2, 3)]).expect("identique : rien");
        assert!(d.set_colors(&[Color::BLACK; 4]).is_err());
        let f = feat.lock();
        assert_eq!(f.as_slice(), [vec![1, 2, 0x40, 1, 9, 8, 7, 0xEC], vec![1, 2, 0x40, 2, 1, 2, 3, 0xEC], vec![1, 2, 0x40, 3, 0, 0, 0, 0xEC]]);
    }
}
