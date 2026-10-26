//! Famille « dell_g » : clavier 4 zones des portables Dell G-series (contrôleur AlienFX, VID 187C PID 0550/0551).
//! Pilote expérimental, désactivé par défaut. Source : docs/rgb-protocols/oem-portables.md (§ Dell) ; faits de format relevés dans les sources
//! publiques OpenRGB (GPL-2, 2026-10-26, [S]) ; Alienware Command Center non analysé (application du Store, non installable ici). Aucun code copié.
//!
//! Rapports : feature reports de 34 octets, identifiant 0 : `00 03 <commande> …`. Plateforme : `00 03 20 02` puis lecture du rapport ; octets 4-5 =
//! identifiant de plateforme, octet 6 = nombre de zones. Seules les plateformes dont la table des zones est connue sont pilotées (0C01 G5 SE 5505,
//! 0E03 G15 5511, 0E07 G15 5520, 0E0A G15 5530 : 4 zones) ; 0A01 G7 7500 (16 zones avec barre lumineuse) : table des zones connue mais NON implémentée) ; toute autre est refusée à l'ouverture (INCONNU).
//! Couleur : commande 0x27 `00 03 27 R G B <nb zones u16> <ids de zones…>` (une trame par couleur distincte), réponse lue ensuite (erreur si elle
//! est identique à la trame envoyée). Les identifiants matériels des zones valent 0..3 (0C01, 0E03) ou 0x10..0x13 (0E07, 0E0A).
//! JAMAIS utilisé : commandes d'animation 0x21 (sauvegarde en flash des animations), 0x22 (animations d'alimentation), 0x28 (reset), 0xFF (effacement).
//! Délai de 60 ms après chaque trame (la source note que le contrôleur n'aime pas être saturé).

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};

const VID: u16 = 0x187C;
const LEN: usize = 34;
const DELAY_MS: u64 = if cfg!(test) { 0 } else { 60 };

/// (identifiant de plateforme, identifiants matériels des 4 zones : gauche, milieu, droite, pavé numérique).
struct Platform {
    id: u16,
    hw: &'static [u8],
}

const FOUR: [&str; 4] = ["Gauche", "Milieu", "Droite", "Pavé numérique"];
const PLATFORMS: [Platform; 4] = [
    Platform { id: 0x0C01, hw: &[0, 1, 2, 3] },
    Platform { id: 0x0E03, hw: &[0, 1, 2, 3] },
    Platform { id: 0x0E07, hw: &[0x10, 0x11, 0x12, 0x13] },
    Platform { id: 0x0E0A, hw: &[0x10, 0x11, 0x12, 0x13] },
];

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != VID || !matches!(pid, 0x0550 | 0x0551) {
        return None;
    }
    // Disposition réelle connue seulement après lecture de la plateforme : 4 zones (cas le plus courant) ; `open` refuse tout écart.
    let zones = FOUR.map(|n| ZoneInfo::fixed(n, 1)).to_vec();
    Some(Model::new("Dell G-series (clavier 4 zones)", DeviceType::Keyboard, zones))
}

struct DellG {
    hid: Box<dyn Hid>,
    hw: &'static [u8],
    last: Vec<Option<Color>>,
}

fn pause() {
    if DELAY_MS > 0 {
        std::thread::sleep(std::time::Duration::from_millis(DELAY_MS));
    }
}

fn config_request() -> [u8; LEN] {
    let mut r = [0u8; LEN];
    r[..4].copy_from_slice(&[0, 3, 0x20, 0x02]);
    r
}

fn color_frame(c: Color, zones: &[u8]) -> [u8; LEN] {
    let mut r = [0u8; LEN];
    r[..8].copy_from_slice(&[0, 3, 0x27, c.r, c.g, c.b, 0, zones.len() as u8]);
    r[8..8 + zones.len()].copy_from_slice(zones);
    r
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    hid.send_feature_report(&config_request())?;
    pause();
    let mut buf = [0u8; LEN];
    let n = hid.get_feature_report(&mut buf)?;
    if n < 7 {
        bail!("Dell G : réponse de configuration trop courte ({n} octets)");
    }
    let id = u16::from(buf[4]) << 8 | u16::from(buf[5]);
    let Some(p) = PLATFORMS.iter().find(|p| p.id == id) else {
        bail!("Dell G : plateforme {id:04X} inconnue (INCONNU : disposition des zones non documentée), refusée");
    };
    debug_assert_eq!(p.hw.len(), model.zones.len());
    Ok(Box::new(DellG { hid, hw: p.hw, last: vec![None; p.hw.len()] }))
}

impl NativeRgb for DellG {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        if colors.len() > self.hw.len() {
            bail!("Dell G : {} couleurs pour {} zones", colors.len(), self.hw.len());
        }
        let want: Vec<Color> = (0..self.hw.len()).map(|i| colors.get(i).copied().unwrap_or(Color::BLACK)).collect();
        // Une trame par couleur distincte, seulement pour les zones qui ont changé.
        let mut groups: Vec<(Color, Vec<u8>)> = Vec::new();
        for (i, c) in want.iter().enumerate() {
            if self.last[i] == Some(*c) {
                continue;
            }
            match groups.iter_mut().find(|(g, _)| g == c) {
                Some((_, ids)) => ids.push(self.hw[i]),
                None => groups.push((*c, vec![self.hw[i]])),
            }
        }
        for (c, ids) in groups {
            let frame = color_frame(c, &ids);
            self.hid.send_feature_report(&frame)?;
            pause();
            let mut resp = [0u8; LEN];
            let n = self.hid.get_feature_report(&mut resp)?;
            if n < 2 || resp[1] != 3 || resp == frame {
                bail!("Dell G : le contrôleur a refusé la couleur");
            }
        }
        self.last = want.into_iter().map(Some).collect();
        Ok(())
    }
}

pub const FAMILY: Family = Family { id: "dell_g", doc: "docs/rgb-protocols/oem-portables.md", probe, open };

#[cfg(test)]
mod tests {
    use super::super::MockHid;
    use super::*;

    fn config(platform: u16, zones: u8) -> Vec<u8> {
        let mut r = vec![0u8; LEN];
        r[1] = 3;
        r[4] = (platform >> 8) as u8;
        r[5] = platform as u8;
        r[6] = zones;
        r
    }

    fn ack() -> Vec<u8> {
        let mut r = vec![0u8; LEN];
        r[1] = 3;
        r[2] = 0x28; // différent de la trame envoyée
        r
    }

    #[test]
    fn probe_claims_the_two_g_series_pids() {
        assert_eq!(probe(VID, 0x0550).expect("g").led_count(), 4);
        assert!(probe(VID, 0x0551).is_some());
        assert!(probe(VID, 0x0552).is_none());
    }

    #[test]
    fn unknown_platform_is_refused_without_any_color_frame() {
        let model = probe(VID, 0x0550).expect("g");
        let (mock, _, feat) = MockHid::new(vec![config(0x1234, 4)]);
        assert!(open(Box::new(mock), &model).is_err());
        let f = feat.lock();
        assert_eq!(f.len(), 1, "seule la lecture de configuration est envoyée");
        assert_eq!(&f[0][..4], &[0, 3, 0x20, 0x02]);
    }

    #[test]
    fn colors_are_grouped_per_distinct_color_with_hardware_ids() {
        let model = probe(VID, 0x0551).expect("g");
        let (mock, _, feat) = MockHid::new(vec![config(0x0E07, 4), ack(), ack()]);
        let mut d = open(Box::new(mock), &model).expect("open");
        let (red, blue) = (Color::new(255, 0, 0), Color::new(0, 0, 255));
        d.set_colors(&[red, blue, red, red]).expect("set");
        d.set_colors(&[red, blue, red, red]).expect("identique : rien");
        let f = feat.lock();
        assert_eq!(f.len(), 3);
        assert_eq!(&f[1][..11], &[0, 3, 0x27, 255, 0, 0, 0, 3, 0x10, 0x12, 0x13]);
        assert_eq!(&f[2][..9], &[0, 3, 0x27, 0, 0, 255, 0, 1, 0x11]);
        assert!(f.iter().all(|r| r.len() == LEN));
        assert!(f.iter().skip(1).all(|r| r[2] == 0x27), "jamais de commande d'animation, de reset ou d'effacement");
    }

    #[test]
    fn echoed_frame_is_an_error() {
        let model = probe(VID, 0x0550).expect("g");
        let mut echo = color_frame(Color::BLACK, &[0, 1, 2, 3]).to_vec();
        echo[1] = 3;
        let (mock, _, _) = MockHid::new(vec![config(0x0C01, 4), echo]);
        let mut d = open(Box::new(mock), &model).expect("open");
        assert!(d.set_colors(&[Color::BLACK; 4]).is_err());
    }
}
