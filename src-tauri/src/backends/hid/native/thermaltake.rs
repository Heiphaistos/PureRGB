//! Famille « thermaltake » : Riing Plus / TT RGB PLUS (VID 0x264A, PID 1FA4..=1FB5), 5 ports ventilo + RGB.
//! Source : docs/rgb-protocols/thermaltake.md (section « Protocole Riing Plus »).
//!
//! Non revendiqués (note insuffisante) : Riing Quad (paquet de 193 o, contenu INCONNU) et Riing Trio (découpage en morceaux INCONNU),
//! dont la vitesse n'est que « supposée » [S] ; Toughliquid/Floe (INCONNU). Lecture RPM : hors trait `NativeRgb`, non exposée.

use super::{split_by_zone, Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};

const VID: u16 = 0x264A;
const PORTS: u8 = 5;
/// Rapport : ID 00 + 64 octets.
const REPORT: usize = 65;
/// Un paquet RGB = 4 octets d'en-tête + 3 par LED : 20 LED au plus. // INCONNU : découpage au-delà (la note annonce 54 LED/port sans l'expliquer).
const MAX_LEDS: usize = (REPORT - 1 - 4) / 3;
/// Mode « par LED » (note : octet mode 0x18).
const MODE_PER_LED: u8 = 0x18;
const READ_TIMEOUT_MS: i32 = 100;

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != VID || !(0x1FA4..=0x1FB5).contains(&pid) {
        return None;
    }
    let zones = (1..=PORTS)
        .map(|p| ZoneInfo { name: format!("Port {p}"), led_count: MAX_LEDS as u32, zone_type: 1, leds_min: 1, leds_max: MAX_LEDS as u32 })
        .collect();
    let mut m = Model::new("Thermaltake Riing Plus", DeviceType::Fan, zones);
    m.fans = PORTS;
    m.interface = Some(0);
    Some(m)
}

struct Riing {
    hid: Box<dyn Hid>,
    zones: Vec<ZoneInfo>,
}

impl Riing {
    /// Envoie une commande (zéro-complétée à 65 o) puis consomme la réponse (contenu non exploité, la note ne décrit pas de code d'erreur).
    fn command(&self, payload: &[u8]) -> Result<()> {
        let mut buf = [0u8; REPORT];
        buf[1..=payload.len()].copy_from_slice(payload);
        self.hid.write(&buf)?;
        let mut reply = [0u8; REPORT];
        self.hid.read_timeout(&mut reply, READ_TIMEOUT_MS)?;
        Ok(())
    }
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    let dev = Riing { hid, zones: model.zones.clone() };
    dev.command(&[0xFE, 0x33])?;
    Ok(Box::new(dev))
}

impl NativeRgb for Riing {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        for (i, leds) in split_by_zone(colors, &self.zones).iter().enumerate() {
            if leds.len() > MAX_LEDS {
                bail!("Riing Plus : {} LED sur le port {} (max {MAX_LEDS})", leds.len(), i + 1);
            }
            let mut p = vec![0x32, 0x52, i as u8 + 1, MODE_PER_LED];
            for c in leds {
                p.extend_from_slice(&[c.g, c.r, c.b]); // ordre G,R,B
            }
            self.command(&p)?;
        }
        Ok(())
    }

    fn set_fan_duty(&mut self, channel: u8, percent: u8) -> Result<()> {
        if channel >= PORTS {
            bail!("Riing Plus : ventilateur {channel} inexistant (0..{})", PORTS - 1);
        }
        if percent > 100 {
            bail!("Riing Plus : vitesse {percent} % hors plage 0-100");
        }
        self.command(&[0x32, 0x51, channel + 1, 0x01, percent])
    }
}

pub const FAMILY: Family = Family { id: "thermaltake", doc: "docs/rgb-protocols/thermaltake.md", probe, open };

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::hid::native::MockHid;

    fn dev() -> (Box<dyn NativeRgb>, super::super::Log) {
        let m = probe(VID, 0x1FA5).unwrap();
        let (hid, sent, _) = MockHid::new(vec![]);
        (open(Box::new(hid), &m).unwrap(), sent)
    }

    fn frame(bytes: &[u8]) -> Vec<u8> {
        let mut v = vec![0u8; REPORT];
        v[1..=bytes.len()].copy_from_slice(bytes);
        v
    }

    #[test]
    fn probe_claims_only_documented_pids() {
        assert!(probe(VID, 0x1FA4).is_some() && probe(VID, 0x1FB5).is_some());
        assert!(probe(VID, 0x1FB6).is_none() && probe(VID, 0x2260).is_none() && probe(0x1234, 0x1FA5).is_none());
        let m = probe(VID, 0x1FA5).unwrap();
        assert_eq!((m.fans, m.zones.len(), m.interface), (5, 5, Some(0)));
    }

    #[test]
    fn init_then_fan_speed_frames() {
        let (mut d, sent) = dev();
        d.set_fan_duty(0, 75).unwrap();
        d.set_fan_duty(4, 0).unwrap();
        let s = sent.lock();
        assert_eq!(s[0], frame(&[0xFE, 0x33]));
        assert_eq!(s[1], frame(&[0x32, 0x51, 0x01, 0x01, 0x4B]));
        assert_eq!(s[2], frame(&[0x32, 0x51, 0x05, 0x01, 0x00]));
    }

    #[test]
    fn fan_validation() {
        let (mut d, sent) = dev();
        assert!(d.set_fan_duty(0, 101).is_err());
        assert!(d.set_fan_duty(5, 50).is_err());
        assert_eq!(sent.lock().len(), 1); // seule l'init est partie
    }

    #[test]
    fn colors_are_grb_per_port() {
        let (mut d, sent) = dev();
        let mut cols = vec![Color::BLACK; 100];
        cols[0] = Color::new(1, 2, 3);
        cols[20] = Color::new(9, 8, 7); // 1re LED du port 2
        d.set_colors(&cols).unwrap();
        let s = sent.lock();
        assert_eq!(s.len(), 6); // init + 5 ports
        let mut p1 = vec![0x32, 0x52, 0x01, 0x18, 2, 1, 3];
        p1.extend([0u8; 57]);
        assert_eq!(s[1], frame(&p1));
        assert_eq!(&s[2][1..8], &[0x32, 0x52, 0x02, 0x18, 8, 9, 7]);
        assert!(s.iter().all(|f| f.len() == REPORT));
    }
}
