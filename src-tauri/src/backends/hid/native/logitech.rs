//! Famille « logitech » : claviers G410 / G810 / G910, couleur par touche via la feature HID++ 0x8080 (PER_KEY_LIGHTING).
//! Source : docs/rgb-protocols/logitech.md (sections « Cadre HID++ » et « Feature 0x8080 »).
//!
//! Livré : IRoot (recherche de l'index de la feature 0x8080) puis SET_KEY_COLORS + FLUSH_LEDS (14 touches max par trame 0x12).
//! INCONNU (donc refusé / non revendiqué) :
//! - G815 / G915 / G213 / G813 / G512 / G513 / Pro TKL : table des keycodes et protocoles non transcrits dans la note ;
//! - souris (0x8071 : index d'effet, octets SW_CONTROL `mode`/`flags` et format de GET_INFO inconnus), Powerplay, casques, récepteurs ;
//! - format exact d'une réponse d'erreur HID++ (seul « feature_index 0xFF » est indiqué, « non revérifié ») ;
//! - mappage touche -> LED par modèle (G-keys, logo, multimédia = autres keyTypes, non pilotés) ;
//! - retour au mode matériel : aucune commande documentée, `release()` ne fait rien.
//!
//! À VALIDER sur matériel : IRoot envoyé en rapport 0x12 (la note ne donne que le format générique), usage 0x0604 ouvert.

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};

const VID: u16 = 0x046D;
/// Index de périphérique d'un appareil filaire.
const DEVICE: u8 = 0xFF;
/// Identifiant d'émetteur (4 bits) ; le firmware le renvoie tel quel.
const SW_ID: u8 = 0x07;
const REPORT_VLONG: u8 = 0x12;
const FRAME_LEN: usize = 64;
const FEATURE_PER_KEY: u16 = 0x8080;
const FN_SET_KEY_COLORS: u8 = 0x30;
const FN_FLUSH_LEDS: u8 = 0x50;
/// keyType « clavier » (bit 0x01) : keyId = usage HID.
const KEY_TYPE_KEYBOARD: u16 = 0x0001;
const KEYS_PER_FRAME: usize = 14;
const REPLY_TRIES: usize = 8;
const REPLY_TIMEOUT_MS: i32 = 500;

/// LED i -> usage HID clavier (standard USB HID : 0x04-0x65 alphanumérique/ponctuation/F1-F12/navigation, 0xE0-0xE7 modificateurs).
/// INCONNU : les touches réellement présentes par modèle ; les absentes sont envoyées telles quelles (comportement du firmware non documenté).
fn key_ids() -> impl Iterator<Item = u8> {
    (0x04u8..=0x65).chain(0xE0..=0xE7)
}
const LEDS: usize = 0x62 + 8;

/// (PID, nom). PID issus de signalrgb-devices.csv (famille « 8080 Devices (Legacy) »).
const MODELS: &[(u16, &str)] = &[
    (0xC330, "Logitech G410 Atlas Spectrum"),
    (0xC331, "Logitech G810"),
    (0xC337, "Logitech G810"),
    (0xC32B, "Logitech G910"),
    (0xC335, "Logitech G910"),
];

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != VID {
        return None;
    }
    let (_, name) = MODELS.iter().find(|(p, _)| *p == pid)?;
    let mut m = Model::new(*name, DeviceType::Keyboard, vec![ZoneInfo::fixed("Touches (usages HID)", LEDS as u32)]);
    m.interface = Some(1);
    Some(m)
}

struct Logitech {
    hid: Box<dyn Hid>,
    /// Index de la feature 0x8080 dans la table de l'appareil.
    feature: u8,
}

fn frame(feature: u8, function: u8) -> [u8; FRAME_LEN] {
    let mut f = [0u8; FRAME_LEN];
    f[0] = REPORT_VLONG;
    f[1] = DEVICE;
    f[2] = feature;
    f[3] = function | SW_ID;
    f
}

/// IRoot (feature index 0, fonction 0) : id de feature -> index.
fn find_feature(hid: &dyn Hid, id: u16) -> Result<u8> {
    let mut req = frame(0x00, 0x00);
    req[4..6].copy_from_slice(&id.to_be_bytes());
    hid.write(&req)?;
    for _ in 0..REPLY_TRIES {
        let mut buf = [0u8; FRAME_LEN];
        let n = hid.read_timeout(&mut buf, REPLY_TIMEOUT_MS)?;
        if n == 0 {
            bail!("Logitech : pas de réponse à la recherche de la feature {id:#06x}");
        }
        if n < 5 || !matches!(buf[0], 0x11 | 0x12) || buf[1] != DEVICE {
            continue;
        }
        if buf[2] == 0xFF {
            bail!("Logitech : erreur HID++ à la recherche de la feature {id:#06x}");
        }
        if buf[2] == 0x00 && buf[3] == SW_ID {
            return match buf[4] {
                0 => bail!("Logitech : feature {id:#06x} absente de cet appareil"),
                i => Ok(i),
            };
        }
    }
    bail!("Logitech : réponse IRoot introuvable")
}

fn open(hid: Box<dyn Hid>, _model: &Model) -> Result<Box<dyn NativeRgb>> {
    let feature = find_feature(hid.as_ref(), FEATURE_PER_KEY)?;
    Ok(Box::new(Logitech { hid, feature }))
}

impl NativeRgb for Logitech {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        if colors.len() > LEDS {
            bail!("Logitech : {} couleurs pour {LEDS} touches", colors.len());
        }
        let all: Vec<(u8, Color)> = key_ids().enumerate().map(|(i, k)| (k, colors.get(i).copied().unwrap_or(Color::BLACK))).collect();
        for chunk in all.chunks(KEYS_PER_FRAME) {
            let mut f = frame(self.feature, FN_SET_KEY_COLORS);
            f[4..6].copy_from_slice(&KEY_TYPE_KEYBOARD.to_be_bytes());
            f[6..8].copy_from_slice(&(chunk.len() as u16).to_be_bytes());
            for (i, (k, c)) in chunk.iter().enumerate() {
                f[8 + 4 * i..12 + 4 * i].copy_from_slice(&[*k, c.r, c.g, c.b]);
            }
            self.hid.write(&f)?;
        }
        self.hid.write(&frame(self.feature, FN_FLUSH_LEDS))?;
        Ok(())
    }
}

pub const FAMILY: Family = Family { id: "logitech", doc: "docs/rgb-protocols/logitech.md", probe, open };

#[cfg(test)]
mod tests {
    use super::super::MockHid;
    use super::*;

    type Sent = std::sync::Arc<parking_lot::Mutex<Vec<Vec<u8>>>>;

    fn iroot_reply(index: u8) -> Vec<u8> {
        let mut r = vec![0u8; 20];
        r[..5].copy_from_slice(&[0x11, 0xFF, 0x00, 0x07, index]);
        r
    }

    fn driver(replies: Vec<Vec<u8>>) -> (Result<Box<dyn NativeRgb>>, Sent) {
        let model = probe(VID, 0xC330).expect("pid");
        let (mock, sent, _) = MockHid::new(replies);
        (open(Box::new(mock), &model), sent)
    }

    #[test]
    fn probe_claims_only_documented_pids() {
        for pid in [0xC330, 0xC331, 0xC337, 0xC32B, 0xC335] {
            let m = probe(VID, pid).expect("pid");
            assert_eq!(m.interface, Some(1));
            assert_eq!(m.led_count(), 106);
        }
        assert!(probe(VID, 0xC33F).is_none()); // G815 : keycodes INCONNUS
        assert!(probe(VID, 0xC336).is_none()); // G213 : INCONNU
        assert!(probe(VID, 0xC08B).is_none()); // souris : INCONNU
        assert!(probe(0x1038, 0xC330).is_none());
    }

    #[test]
    fn iroot_request_is_exact() {
        let (d, sent) = driver(vec![iroot_reply(0x0B)]);
        assert!(d.is_ok());
        let f = sent.lock()[0].clone();
        let mut want = vec![0u8; 64];
        want[..6].copy_from_slice(&[0x12, 0xFF, 0x00, 0x07, 0x80, 0x80]);
        assert_eq!(f, want);
    }

    #[test]
    fn open_refuses_absent_feature_error_and_silence() {
        assert!(driver(vec![iroot_reply(0)]).0.is_err());
        let mut err = iroot_reply(0);
        err[2] = 0xFF;
        assert!(driver(vec![err]).0.is_err());
        assert!(driver(vec![]).0.is_err());
    }

    #[test]
    fn open_skips_unrelated_reports_before_the_answer() {
        let mut noise = vec![0u8; 20];
        noise[..4].copy_from_slice(&[0x11, 0xFF, 0x05, 0x10]);
        assert!(driver(vec![noise, iroot_reply(0x0B)]).0.is_ok());
    }

    #[test]
    fn set_colors_frames_and_flush_match_the_note() {
        let (d, sent) = driver(vec![iroot_reply(0x0B)]);
        let mut d = d.expect("open");
        let mut c = vec![Color::BLACK; 106];
        c[0] = Color::new(1, 2, 3); // usage 0x04
        c[13] = Color::new(9, 8, 7); // usage 0x11, dernière touche de la trame 1
        c[14] = Color::new(4, 5, 6); // usage 0x12, première de la trame 2
        d.set_colors(&c).expect("set");
        let s = sent.lock();
        assert_eq!(s.len(), 1 + 8 + 1); // IRoot + 8 trames (7 x 14 + 8) + FLUSH
        let f1 = &s[1];
        assert_eq!(f1.len(), 64);
        assert_eq!(&f1[..8], &[0x12, 0xFF, 0x0B, 0x37, 0x00, 0x01, 0x00, 0x0E]);
        assert_eq!(&f1[8..12], &[0x04, 1, 2, 3]);
        assert_eq!(&f1[8 + 52..8 + 56], &[0x11, 9, 8, 7]);
        assert_eq!(&s[2][6..8], &[0x00, 0x0E]);
        assert_eq!(&s[2][8..12], &[0x12, 4, 5, 6]);
        assert_eq!(&s[8][6..8], &[0x00, 0x08]); // dernière trame : 8 touches
        assert_eq!(&s[8][8 + 28..8 + 32], &[0xE7, 0, 0, 0]);
        assert!(s[8][8 + 32..].iter().all(|b| *b == 0));
        let mut flush = vec![0u8; 64];
        flush[..4].copy_from_slice(&[0x12, 0xFF, 0x0B, 0x57]);
        assert_eq!(s[9], flush);
    }

    #[test]
    fn too_many_colors_is_rejected_and_nothing_more_is_sent() {
        let (d, sent) = driver(vec![iroot_reply(0x0B)]);
        let mut d = d.expect("open");
        assert!(d.set_colors(&vec![Color::BLACK; 107]).is_err());
        assert_eq!(sent.lock().len(), 1); // seulement l'IRoot
    }
}
