//! Famille « coolermaster » (VID 0x2516) : ARGB Controller gen 1 (PID 0x1011), couleur statique par port ARGB.
//! Source : docs/rgb-protocols/coolermaster.md (§ ARGB Controller gen 1).
//!
//! Paquet = 65 octets (report-id 0 + 64), octets numérotés comme la note à partir de 1.
//! Non revendiqués (faits insuffisants dans la note) :
//! - Small ARGB Controller (0x1000) : aucun mode statique documenté (spectrum/reload/.../off/direct seulement), octet 7 inconnu.
//! - ARGB Gen 2 A1 (0x0173, 0x01C9, 0x01CB) : séquence d'entrée en temps réel INCONNUE.
//! - Port RGB 12 V (zone 0xFE) : « champs décalés de -2 » trop vague ; mode direct gen 1 : en-tête de paquet ambigu.
//!
//! INCONNU : contrôle de la version du firmware (numéro de série USB, hors du trait `Hid`) ; le firmware doit être 0023 ou 0028
//! d'après la note ; effet d'une trame sur la mémoire du contrôleur (profils : INCONNU, aucune commande de sauvegarde n'est envoyée).

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};

const VID: u16 = 0x2516;
const PID_ARGB_GEN1: u16 = 0x1011;
const PACKET: usize = 65;
/// Octet « zone » des ports ARGB 1 à 4.
const PORTS: [u8; 4] = [0x01, 0x02, 0x04, 0x08];
const MODE_STATIC: u8 = 0x0A;
const MODE_PASSTHROUGH: u8 = 0xFF;
/// Index de couleur « fixe » (0x10), vitesse et luminosité de l'exemple de la note.
const COLOR_FIXED: u8 = 0x10;
const SPEED: u8 = 0x02;
const BRIGHTNESS: u8 = 0xFF;

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if (vid, pid) != (VID, PID_ARGB_GEN1) {
        return None;
    }
    let zones = (1..=PORTS.len())
        .map(|i| {
            let mut z = ZoneInfo::fixed(format!("ARGB {i}"), 1);
            z.zone_type = 0; // une seule couleur par port (mode statique)
            z
        })
        .collect();
    let mut m = Model::new("Cooler Master ARGB Controller", DeviceType::LedStrip, zones);
    m.interface = Some(0);
    m.usage_page = Some(0xFF00);
    Some(m)
}

struct Argb {
    hid: Box<dyn Hid>,
    last: [Option<Color>; 4],
}

fn packet(bytes: &[u8]) -> [u8; PACKET] {
    let mut p = [0u8; PACKET];
    p[1..1 + bytes.len()].copy_from_slice(bytes);
    p
}

impl Argb {
    /// `80 0B 02 <zone> <mode> <index couleur> <vitesse> <luminosité> R G B`.
    fn port_mode(&self, zone: u8, mode: u8, c: Color) -> Result<()> {
        let p = packet(&[0x80, 0x0B, 0x02, zone, mode, COLOR_FIXED, SPEED, BRIGHTNESS, c.r, c.g, c.b]);
        self.hid.write(&p)?;
        Ok(())
    }
}

fn open(hid: Box<dyn Hid>, _model: &Model) -> Result<Box<dyn NativeRgb>> {
    // Fonction « normal » (01) ; le passthrough carte mère serait 02.
    hid.write(&packet(&[0x80, 0x01, 0x01]))?;
    Ok(Box::new(Argb { hid, last: [None; 4] }))
}

impl NativeRgb for Argb {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        if colors.len() > PORTS.len() {
            bail!("Cooler Master ARGB : {} couleurs pour {} ports", colors.len(), PORTS.len());
        }
        for (i, &zone) in PORTS.iter().enumerate() {
            let c = colors.get(i).copied().unwrap_or(Color::BLACK);
            if self.last[i] != Some(c) {
                self.port_mode(zone, MODE_STATIC, c)?;
                self.last[i] = Some(c);
            }
        }
        Ok(())
    }

    fn release(&mut self) {
        // Retour au passthrough carte mère ; rien à rattraper si l'envoi échoue à la fermeture.
        for &zone in &PORTS {
            let _ = self.port_mode(zone, MODE_PASSTHROUGH, Color::BLACK);
        }
    }
}

pub const FAMILY: Family = Family { id: "coolermaster", doc: "docs/rgb-protocols/coolermaster.md", probe, open };

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::hid::native::{probe as global_probe, MockHid};

    fn pkt(bytes: &[u8]) -> Vec<u8> {
        let mut v = vec![0u8; 65];
        v[..bytes.len()].copy_from_slice(bytes);
        v
    }

    #[test]
    fn probe_claims_only_documented_gen1_pid() {
        assert!(probe(VID, 0x1000).is_none());
        assert!(probe(VID, 0x0173).is_none());
        assert!(probe(0x1532, 0x1011).is_none());
        let m = probe(VID, 0x1011).expect("1011");
        assert_eq!((m.interface, m.usage_page, m.led_count(), m.zones.len()), (Some(0), Some(0xFF00), 4, 4));
        assert_eq!(global_probe(VID, 0x1011).map(|(f, _)| f.id), Some("coolermaster"));
    }

    #[test]
    fn open_sends_normal_function_then_static_ports() {
        let (mock, sent, _) = MockHid::new(vec![]);
        let m = probe(VID, 0x1011).expect("1011");
        let mut dev = open(Box::new(mock), &m).expect("open");
        dev.set_colors(&[Color::new(0xFF, 0, 0)]).expect("set");
        let s = sent.lock();
        assert_eq!(s[0], pkt(&[0x00, 0x80, 0x01, 0x01]));
        // port 1 : zone 01, mode static 0A, index 10, vitesse 02, luminosité FF, rouge
        assert_eq!(s[1], pkt(&[0x00, 0x80, 0x0B, 0x02, 0x01, 0x0A, 0x10, 0x02, 0xFF, 0xFF, 0x00, 0x00]));
        // ports 2, 3, 4 : noirs, octets de zone 02, 04, 08
        assert_eq!(s[2], pkt(&[0x00, 0x80, 0x0B, 0x02, 0x02, 0x0A, 0x10, 0x02, 0xFF, 0, 0, 0]));
        assert_eq!(s[3], pkt(&[0x00, 0x80, 0x0B, 0x02, 0x04, 0x0A, 0x10, 0x02, 0xFF, 0, 0, 0]));
        assert_eq!(s[4], pkt(&[0x00, 0x80, 0x0B, 0x02, 0x08, 0x0A, 0x10, 0x02, 0xFF, 0, 0, 0]));
        assert_eq!(s.len(), 5);
    }

    #[test]
    fn unchanged_ports_are_not_resent_and_too_many_colors_are_refused() {
        let (mock, sent, _) = MockHid::new(vec![]);
        let m = probe(VID, 0x1011).expect("1011");
        let mut dev = open(Box::new(mock), &m).expect("open");
        let c = [Color::new(1, 2, 3); 4];
        dev.set_colors(&c).expect("set");
        let n = sent.lock().len();
        dev.set_colors(&c).expect("set");
        assert_eq!(sent.lock().len(), n);
        assert!(dev.set_colors(&[Color::BLACK; 5]).is_err());
    }

    #[test]
    fn release_hands_ports_back_to_motherboard_passthrough() {
        let (mock, sent, _) = MockHid::new(vec![]);
        let m = probe(VID, 0x1011).expect("1011");
        let mut dev = open(Box::new(mock), &m).expect("open");
        dev.release();
        let s = sent.lock();
        assert_eq!(s.len(), 5);
        assert_eq!(s[1], pkt(&[0x00, 0x80, 0x0B, 0x02, 0x01, 0xFF, 0x10, 0x02, 0xFF, 0, 0, 0]));
        assert_eq!(s[4], pkt(&[0x00, 0x80, 0x0B, 0x02, 0x08, 0xFF, 0x10, 0x02, 0xFF, 0, 0, 0]));
    }
}
