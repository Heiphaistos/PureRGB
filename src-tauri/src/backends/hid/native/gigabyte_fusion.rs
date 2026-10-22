//! Famille « gigabyte_fusion » : Gigabyte RGB Fusion 2 USB (ITE IT8297 / IT5702 / IT8950), headers ARGB D_LED1 et D_LED2 en mode direct.
//! Source : docs/rgb-protocols/gigabyte-rgb-fusion.md
//!
//! Périmètre minimal et sûr : UNIQUEMENT les 2 headers ARGB numériques. Les zones carte / 12 V (LED1..LED11) dépendent de la table
//! carte par carte (`device_num` -> layout) qui est INCONNUE dans la note : non pilotées. Non revendiqués : 048D:5711 (6 headers,
//! calibration étendue 0x61 et masque 0x7B non vérifiés ici) et 0414:A100 (comportement « tous les LED » à part).
//! À l'ouverture : demande du rapport d'info (`CC 60`), lecture du nombre de LEDs des headers et de la calibration d'ordre des
//! couleurs (rien n'est écrit en calibration `0x33`), puis `CC 32 03` pour couper les effets internes des 2 headers.
//! Jamais envoyé : 0x5E (sauvegarde flash), 0x47 (persistance), 0x34 (taille des headers), 0x33 (calibration), 0x48.
//! La taille de chaque header est celle déjà configurée dans le contrôleur (plafonnée à 256 LEDs, limite IT57xx).
//! `release()` : aucun retour au mode matériel documenté (INCONNU), donc rien n'est envoyé.

use super::{split_by_zone, Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};
use std::time::Duration;

const REPORT_ID: u8 = 0xCC;
const PKT: usize = 64;
const LEDS_PER_PACKET: usize = 19;
const HEADERS: [u8; 2] = [0x58, 0x59]; // D_LED1, D_LED2
const MAX_LEDS: usize = 256;
const MASK_DISABLE_INTERNAL: u8 = 0x03; // bit0 = D_LED1, bit1 = D_LED2
const ARGB01_OFFSET: usize = 8;
const CAL_STRIP_OFFSETS: [usize; 2] = [44, 48];
const LED_ENUM: [usize; 5] = [32, 64, 256, 512, 1024];

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != 0x048D || !matches!(pid, 0x8297 | 0x8950 | 0x5702) {
        return None;
    }
    let zones = ["D_LED1", "D_LED2"]
        .iter()
        .map(|n| ZoneInfo { name: (*n).into(), led_count: 1, zone_type: 1, leds_min: 1, leds_max: MAX_LEDS as u32 })
        .collect();
    let mut m = Model::new("Gigabyte RGB Fusion 2 (ARGB)", DeviceType::Motherboard, zones);
    m.usage_page = Some(0xFF89);
    Some(m)
}

fn frame(bytes: &[u8]) -> [u8; PKT] {
    let mut p = [0u8; PKT];
    p[0] = REPORT_ID;
    p[1..1 + bytes.len()].copy_from_slice(bytes);
    p
}

/// Position (0..2) de R, G, B dans le triplet envoyé, d'après `(R_idx<<16)|(G_idx<<8)|B_idx`.
fn decode_order(v: u32) -> Result<[usize; 3]> {
    let o = [(v >> 16 & 0xFF) as usize, (v >> 8 & 0xFF) as usize, (v & 0xFF) as usize];
    let mut seen = [false; 3];
    for p in o {
        if p > 2 || seen[p] {
            bail!("Gigabyte Fusion : calibration d'ordre des couleurs invalide ({v:#x})");
        }
        seen[p] = true;
    }
    Ok(o)
}

struct Fusion {
    hid: Box<dyn Hid>,
    zones: Vec<ZoneInfo>,
    counts: [usize; 2],
    order: [[usize; 3]; 2],
}

impl NativeRgb for Fusion {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        let parts = split_by_zone(colors, &self.zones);
        for (h, part) in parts.iter().enumerate() {
            let n = self.counts[h];
            let fill = part.last().copied().unwrap_or(Color::BLACK);
            let mut bytes = Vec::with_capacity(n * 3);
            for k in 0..n {
                let c = part.get(k).copied().unwrap_or(fill);
                let mut t = [0u8; 3];
                for (v, pos) in [c.r, c.g, c.b].into_iter().zip(self.order[h]) {
                    t[pos] = v;
                }
                bytes.extend_from_slice(&t);
            }
            for (i, chunk) in bytes.chunks(LEDS_PER_PACKET * 3).enumerate() {
                let off = (i * LEDS_PER_PACKET * 3) as u16;
                let mut p = frame(&[HEADERS[h], off as u8, (off >> 8) as u8, chunk.len() as u8]);
                p[5..5 + chunk.len()].copy_from_slice(chunk);
                self.hid.send_feature_report(&p)?;
            }
        }
        Ok(())
    }
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    if model.zones.len() != HEADERS.len() {
        bail!("Gigabyte Fusion : disposition de modèle inattendue");
    }
    hid.send_feature_report(&frame(&[0x60, 0x00]))?;
    let mut info = [0u8; PKT];
    info[0] = REPORT_ID;
    let n = hid.get_feature_report(&mut info)?;
    if n != PKT || info[0] != REPORT_ID {
        bail!("Gigabyte Fusion : rapport d'info invalide ({n} octets)");
    }
    let mut counts = [0usize; 2];
    let mut order = [[0usize; 3]; 2];
    let nib = [info[ARGB01_OFFSET] & 0x0F, info[ARGB01_OFFSET] >> 4];
    for h in 0..2 {
        counts[h] = match LED_ENUM.get(nib[h] as usize) {
            Some(c) => (*c).min(MAX_LEDS),
            None => bail!("Gigabyte Fusion : taille de header inconnue (code {})", nib[h]),
        };
        let o = CAL_STRIP_OFFSETS[h];
        order[h] = decode_order(u32::from_le_bytes([info[o], info[o + 1], info[o + 2], info[o + 3]]))?;
    }
    hid.send_feature_report(&frame(&[0x32, MASK_DISABLE_INTERNAL]))?;
    std::thread::sleep(Duration::from_millis(50)); // délai documenté après 0x32
    Ok(Box::new(Fusion { hid, zones: model.zones.clone(), counts, order }))
}

pub const FAMILY: Family = Family { id: "gigabyte_fusion", doc: "docs/rgb-protocols/gigabyte-rgb-fusion.md", probe, open };

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::hid::native::MockHid;

    /// Rapport d'info : LED1 = code 0 (32 LEDs) ordre RGB, LED2 = code 2 (256) ordre BGR.
    fn info(code01: u8) -> Vec<u8> {
        let mut r = vec![0u8; PKT];
        r[0] = 0xCC;
        r[ARGB01_OFFSET] = code01;
        r[44..48].copy_from_slice(&[0x02, 0x01, 0x00, 0x00]); // u32 0x000102 : R@0 G@1 B@2
        r[48..52].copy_from_slice(&[0x00, 0x01, 0x02, 0x00]); // u32 0x020100 : R@2 G@1 B@0
        r
    }
    fn padded(b: &[u8]) -> Vec<u8> {
        let mut v = b.to_vec();
        v.resize(PKT, 0);
        v
    }

    #[test]
    fn probe_claims_only_documented_pids() {
        assert!(probe(0x048D, 0x8297).is_some());
        assert!(probe(0x048D, 0x8950).is_some());
        assert_eq!(probe(0x048D, 0x5702).and_then(|m| m.usage_page), Some(0xFF89));
        assert!(probe(0x048D, 0x5711).is_none());
        assert!(probe(0x0414, 0xA100).is_none());
        assert!(probe(0x048D, 0x1234).is_none());
    }

    #[test]
    fn open_requests_info_then_disables_internal_effects() {
        let (mock, _, features) = MockHid::new(vec![info(0x20)]);
        let m = probe(0x048D, 0x8297).unwrap();
        assert!(open(Box::new(mock), &m).is_ok());
        let f = features.lock();
        assert_eq!(f.len(), 2);
        assert_eq!(f[0], padded(&[0xCC, 0x60, 0x00]));
        assert_eq!(f[1], padded(&[0xCC, 0x32, 0x03]));
    }

    #[test]
    fn open_rejects_bad_info() {
        let m = probe(0x048D, 0x8297).unwrap();
        let (mock, _, _) = MockHid::new(vec![vec![0xCC; 10]]);
        assert!(open(Box::new(mock), &m).is_err());
        let (mock, _, _) = MockHid::new(vec![info(0x05)]); // code 5 : hors énumération
        assert!(open(Box::new(mock), &m).is_err());
        let mut bad_cal = info(0x20);
        bad_cal[44..48].copy_from_slice(&[0x01, 0x01, 0x00, 0x00]); // pas une permutation
        let (mock, _, _) = MockHid::new(vec![bad_cal]);
        assert!(open(Box::new(mock), &m).is_err());
    }

    #[test]
    fn direct_packets_follow_calibration_and_19_led_split() {
        let (mock, _, features) = MockHid::new(vec![info(0x20)]);
        let m = probe(0x048D, 0x8297).unwrap();
        let mut d = open(Box::new(mock), &m).unwrap();
        features.lock().clear();
        // D_LED1 : 32 LEDs (RGB), une seule couleur fournie -> étendue ; D_LED2 : 256 LEDs (BGR).
        d.set_colors(&[Color::new(1, 2, 3), Color::new(10, 20, 30)]).unwrap();
        let f = features.lock();
        // 32 LEDs = 96 octets -> 57 + 39 ; 256 LEDs = 768 octets -> 14 paquets (13*57 + 27)
        assert_eq!(f.len(), 2 + 14);
        let mut p0 = vec![0xCC, 0x58, 0, 0, 57];
        p0.extend_from_slice(&[1, 2, 3].repeat(19));
        assert_eq!(f[0], padded(&p0));
        let mut p1 = vec![0xCC, 0x58, 57, 0, 39];
        p1.extend_from_slice(&[1, 2, 3].repeat(13));
        assert_eq!(f[1], padded(&p1));
        let mut q0 = vec![0xCC, 0x59, 0, 0, 57];
        q0.extend_from_slice(&[30, 20, 10].repeat(19));
        assert_eq!(f[2], padded(&q0));
        // dernier paquet : offset 13*57 = 741 = 0x02E5, 27 octets
        assert_eq!(f[15][..5], [0xCC, 0x59, 0xE5, 0x02, 27]);
    }
}
