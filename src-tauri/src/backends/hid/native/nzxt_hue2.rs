//! Famille « nzxt_hue2 » : NZXT HUE 2, Smart Device V2, « Motherboard Device », RGB & Fan Controller (VID 0x1E71).
//! Sources : docs/rgb-protocols/nzxt.md et docs/rgb-protocols/analyse-nzxt-cam.md (§9, recoupement liquidctl `smart_device.py` +
//! OpenRGB `NZXTHue2Controller`, lus comme documentation). Rapports de 64 o, ID 0 (octet 0 ajouté à l'écriture), zéros de fin.
//!
//! - Énumération : `20 03` -> réponse `21 03` (octet 14 = nb de canaux, 6 octets d'ID d'accessoire par canal dès l'octet 15, 0 = vide).
//! - Couleur directe : `22 10 <1<<canal> 00` + 20 LED (G,R,B), `22 11 …` + 20 LED suivantes, puis validation `22 A0 …` (octets de liquidctl ;
//!   OpenRGB diffère aux octets 7 et 10 : `28` et `80` au lieu de `00` et `64`, rôle INCONNU, on suit liquidctl).
//! - Ventilateurs : `62 01 <1<<canal> 00 00 00`, consigne à l'octet `3+canal`, bornée à 20..=100 % (borne de l'interface CAM, note §5.1).
//! - Nombre de LED par accessoire : table OpenRGB seule (note §9.2) ; un ID absent de cette table = canal ignoré (rien n'est écrit).
//!
//! - Statut ventilateurs (docs/rgb-protocols/nzxt.md, « Statut ventilos ») : `60 02 01 E8 <iv> 01 E8 <iv>` + `60 03` démarrent le rapport,
//!   puis `67 02` : mode du canal à l'octet `16+c` (0 absent, 1 DC, 2 PWM), duty % à l'octet `40+c`.
//!
//! NON utilisés (persistance INCONNUE ou source unique) : effets matériels `28 03` / `2A 04`, mode « wings », firmware.
//! Kraken X3/Z3, 2012/2021 (« 2023 RGB Controller », protocole voisin), 200C, 2016, 2023, 2029, 3016 : non revendiqués.
//!
//! Règle ventilateurs (exigence de l'utilisateur) : PureRGB ne touche à AUCUN ventilateur tant que l'utilisateur n'a rien réglé (ouverture,
//! scan, énumération : zéro écriture de vitesse). À la PREMIÈRE consigne utilisateur, le statut `67 02` est lu une fois pour capturer le duty
//! courant de chaque canal ; `release_fans` ré-écrit cette valeur capturée sur les seuls canaux réglés. Si le statut n'a pas pu être lu (ou
//! canal absent), `release_fans` n'écrit RIEN : jamais de valeur devinée, jamais 100 %.

use super::{split_by_zone, Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};
use std::time::{Duration, Instant};

const VID: u16 = 0x1E71;
const REPORT: usize = 65;
const LEDS_PER_PACKET: usize = 20;
const MAX_LEDS: usize = 2 * LEDS_PER_PACKET;
const DEFAULT_LEDS: u32 = 10;
const SLOTS_PER_CHANNEL: usize = 6;
const FIRST_SLOT: usize = 15;
const READ_TIMEOUT_MS: i32 = 300;
const MAX_READS: usize = 12;
/// Cadence maximale : une image toutes les 20 ms (50 i/s).
const MIN_FRAME_GAP: Duration = Duration::from_millis(20);
const FAN_MIN: u8 = 20;
const FAN_MAX: u8 = 100;
const STATUS_MODE: usize = 16;
const STATUS_DUTY: usize = 40;

/// (PID, nom, type, canaux LED, ventilateurs).
const MODELS: [(u16, &str, DeviceType, usize, u8); 16] = [
    (0x2001, "NZXT HUE 2", DeviceType::LedStrip, 4, 0),
    (0x2002, "NZXT HUE 2 Ambient", DeviceType::LedStrip, 2, 0),
    (0x2005, "NZXT Motherboard Device", DeviceType::Hub, 2, 3),
    (0x200B, "NZXT Motherboard Device 2", DeviceType::Hub, 2, 3),
    (0x2006, "NZXT Smart Device V2", DeviceType::Hub, 2, 3),
    (0x200D, "NZXT Smart Device V2", DeviceType::Hub, 2, 3),
    (0x200F, "NZXT Smart Device V2", DeviceType::Hub, 2, 3),
    (0x2009, "NZXT RGB & Fan Controller", DeviceType::Hub, 2, 3),
    (0x200E, "NZXT RGB & Fan Controller", DeviceType::Hub, 2, 3),
    (0x2010, "NZXT RGB & Fan Controller", DeviceType::Hub, 2, 3),
    (0x2011, "NZXT RGB & Fan Controller (6 canaux)", DeviceType::Hub, 6, 3),
    (0x2019, "NZXT RGB & Fan Controller (6 canaux)", DeviceType::Hub, 6, 3),
    (0x201F, "NZXT RGB & Fan Controller (6 canaux)", DeviceType::Hub, 6, 3),
    (0x2020, "NZXT RGB & Fan Controller (6 canaux)", DeviceType::Hub, 6, 3),
    (0x201B, "NZXT B650E Motherboard", DeviceType::Hub, 6, 3),
    (0x2022, "NZXT RGB & Fan Controller 2024", DeviceType::Hub, 6, 3),
];

/// Nombre de LED par ID d'accessoire (OpenRGB, note §9.2). Absent = INCONNU.
fn accessory_leds(id: u8) -> Option<usize> {
    Some(match id {
        0x01 | 0x04 | 0x0A => 10,
        0x02 | 0x05 | 0x0B | 0x0C | 0x10 | 0x17 | 0x18 | 0x19 => 8,
        0x06 => 6,
        0x08 => 14,
        0x09 => 15,
        0x11 => 1,
        0x13 | 0x14 => 18,
        0x15 | 0x16 => 20,
        0x1C => 16,
        0x1D | 0x1E | 0x1F => 24,
        _ => return None,
    })
}

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != VID {
        return None;
    }
    let (_, name, dtype, channels, fans) = *MODELS.iter().find(|m| m.0 == pid)?;
    let zones = (1..=channels)
        .map(|i| ZoneInfo { name: format!("Canal {i}"), led_count: DEFAULT_LEDS, zone_type: 1, leds_min: 1, leds_max: MAX_LEDS as u32 })
        .collect();
    let mut m = Model::new(name, dtype, zones);
    m.fans = fans;
    Some(m)
}

/// LED réellement branchées par canal : `None` = accessoire inconnu (canal ignoré), `Some(0)` = canal vide.
fn parse_accessories(reply: &[u8], channels: usize) -> Vec<Option<usize>> {
    let reported = reply[14] as usize;
    (0..channels)
        .map(|c| {
            if c >= reported {
                return Some(0);
            }
            let slots = &reply[FIRST_SLOT + c * SLOTS_PER_CHANNEL..FIRST_SLOT + (c + 1) * SLOTS_PER_CHANNEL];
            let mut total = 0;
            for &id in slots.iter().take_while(|&&id| id != 0) {
                total += accessory_leds(id)?;
            }
            Some(total.min(MAX_LEDS))
        })
        .collect()
}

struct Hue2 {
    hid: Box<dyn Hid>,
    zones: Vec<ZoneInfo>,
    leds: Vec<Option<usize>>,
    fans: u8,
    duty: Vec<Option<u8>>,
    /// Duty de chaque canal avant la prise de contrôle (lu sur `67 02`) ; `None` = inconnu -> rien à restaurer.
    captured: Vec<Option<u8>>,
    /// Le statut a été lu (ou tenté) : une seule capture, avant la toute première consigne.
    capture_done: bool,
    last_frame: Option<Instant>,
}

impl Hue2 {
    fn send(&self, payload: &[u8]) -> Result<()> {
        let mut buf = [0u8; REPORT];
        buf[1..=payload.len()].copy_from_slice(payload);
        self.hid.write(&buf)?;
        Ok(())
    }

    fn write_channel(&self, channel: usize, colors: &[Color]) -> Result<()> {
        let mask = 1u8 << channel;
        for (group, page) in [0x10u8, 0x11].into_iter().enumerate() {
            let mut p = vec![0x22, page, mask, 0x00];
            for i in 0..LEDS_PER_PACKET {
                let c = colors.get(group * LEDS_PER_PACKET + i).copied().unwrap_or(Color::BLACK);
                p.extend_from_slice(&[c.g, c.r, c.b]);
            }
            self.send(&p)?;
        }
        self.send(&[0x22, 0xA0, mask, 0x00, 0x01, 0, 0, 0, 0, 0, 0x64, 0x00, 0x32, 0x00, 0x00, 0x01])
    }

    fn write_fan(&self, channel: u8, duty: u8) -> Result<()> {
        let mut p = [0x62, 0x01, 1 << channel, 0, 0, 0];
        p[3 + channel as usize] = duty;
        self.send(&p)
    }

    /// Lit une fois le duty courant de chaque ventilateur (`67 02`). Échec ou délai : rien n'est capturé, donc rien ne sera restauré.
    fn capture_fans(&mut self) {
        self.capture_done = true;
        let start: [&[u8]; 2] = [&[0x60, 0x02, 0x01, 0xE8, 0x01, 0x01, 0xE8, 0x01], &[0x60, 0x03]];
        if start.iter().any(|p| self.send(p).is_err()) {
            return;
        }
        let mut buf = [0u8; 64];
        for _ in 0..MAX_READS {
            let Ok(n) = self.hid.read_timeout(&mut buf, READ_TIMEOUT_MS) else { return };
            if n == buf.len() && buf[0] == 0x67 && buf[1] == 0x02 {
                for c in 0..self.fans as usize {
                    let (mode, duty) = (buf[STATUS_MODE + c], buf[STATUS_DUTY + c]);
                    self.captured[c] = (mode != 0 && duty <= 100).then_some(duty);
                }
                return;
            }
            if n == 0 {
                return;
            }
        }
    }
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    let dev = Hue2 { hid, zones: model.zones.clone(), leds: vec![], fans: model.fans, duty: vec![None; model.fans as usize], captured: vec![None; model.fans as usize], capture_done: false, last_frame: None };
    dev.send(&[0x20, 0x03])?;
    let mut buf = [0u8; 64];
    for _ in 0..MAX_READS {
        let n = dev.hid.read_timeout(&mut buf, READ_TIMEOUT_MS)?;
        // Réponse valide = `21 03` complet ; une réponse tronquée ou d'un autre type est ignorée.
        if n == buf.len() && buf[0] == 0x21 && buf[1] == 0x03 {
            let leds = parse_accessories(&buf, model.zones.len());
            return Ok(Box::new(Hue2 { leds, ..dev }));
        }
        if n == 0 {
            break;
        }
    }
    bail!("NZXT : pas de réponse valide à la liste des accessoires (`21 03`) ; NZXT CAM ouvert ? mauvaise interface ?")
}

impl NativeRgb for Hue2 {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        if let Some(t) = self.last_frame {
            std::thread::sleep(MIN_FRAME_GAP.saturating_sub(t.elapsed()));
        }
        let parts = split_by_zone(colors, &self.zones);
        for (c, part) in parts.iter().enumerate() {
            match self.leds.get(c).copied().flatten() {
                Some(n) if n > 0 => self.write_channel(c, &part[..part.len().min(n)])?,
                _ => {} // canal vide ou accessoire inconnu : rien n'est écrit
            }
        }
        self.last_frame = Some(Instant::now());
        Ok(())
    }

    fn set_fan_duty(&mut self, channel: u8, percent: u8) -> Result<()> {
        if channel >= self.fans {
            bail!("NZXT : ventilateur {channel} inexistant (0..{})", self.fans.saturating_sub(1));
        }
        let duty = percent.clamp(FAN_MIN, FAN_MAX);
        if !self.capture_done {
            self.capture_fans();
        }
        if self.duty[channel as usize] == Some(duty) {
            return Ok(());
        }
        self.write_fan(channel, duty)?;
        self.duty[channel as usize] = Some(duty);
        Ok(())
    }

    fn release(&mut self) {
        self.release_fans();
    }

    fn fans_taken(&self) -> bool {
        self.duty.iter().any(Option::is_some)
    }

    fn release_fans(&mut self) {
        for c in 0..self.fans as usize {
            // Seuls les canaux réglés par l'utilisateur ET dont l'état d'origine a été lu sont ré-écrits.
            let (Some(cur), Some(orig)) = (self.duty[c], self.captured[c]) else {
                self.duty[c] = None;
                continue;
            };
            let orig = orig.clamp(FAN_MIN, FAN_MAX);
            if cur != orig {
                // Échec ignoré : l'appareil est peut-être déjà débranché ; rien d'autre à tenter.
                let _ = self.write_fan(c as u8, orig);
            }
            self.duty[c] = None;
        }
    }
}

pub const FAMILY: Family = Family { id: "nzxt_hue2", doc: "docs/rgb-protocols/nzxt.md", probe, open };

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::hid::native::MockHid;

    /// Réponse `21 03` fabriquée : `slots[c]` = 6 ID d'accessoires du canal c.
    fn reply(channels: u8, slots: &[[u8; 6]]) -> Vec<u8> {
        let mut r = vec![0u8; 64];
        r[0] = 0x21;
        r[1] = 0x03;
        r[14] = channels;
        for (c, s) in slots.iter().enumerate() {
            r[15 + 6 * c..21 + 6 * c].copy_from_slice(s);
        }
        r
    }

    fn frame(bytes: &[u8]) -> Vec<u8> {
        let mut v = vec![0u8; REPORT];
        v[1..=bytes.len()].copy_from_slice(bytes);
        v
    }

    fn dev(pid: u16, replies: Vec<Vec<u8>>) -> (Result<Box<dyn NativeRgb>>, super::super::Log) {
        let (hid, sent, _) = MockHid::new(replies);
        (open(Box::new(hid), &probe(VID, pid).unwrap()), sent)
    }

    #[test]
    fn probe_claims_only_established_pids() {
        for pid in [0x2001, 0x2002, 0x2005, 0x200B, 0x2006, 0x200D, 0x200F, 0x2009, 0x200E, 0x2010, 0x2011, 0x2019, 0x201F, 0x2020, 0x201B, 0x2022] {
            assert!(probe(VID, pid).is_some(), "{pid:04x}");
        }
        for pid in [0x200C, 0x2016, 0x2023, 0x2029, 0x3016, 0x2012, 0x2021, 0x2007, 0x3008, 0x1714] {
            assert!(probe(VID, pid).is_none(), "{pid:04x}");
        }
        assert!(probe(0x1234, 0x2006).is_none());
        let m = probe(VID, 0x2022).unwrap();
        assert_eq!((m.zones.len(), m.fans), (6, 3));
        let m = probe(VID, 0x2001).unwrap();
        assert_eq!((m.zones.len(), m.fans), (4, 0));
        assert_eq!(probe(VID, 0x2006).unwrap().zones.len(), 2);
    }

    #[test]
    fn open_sends_accessory_request_and_needs_a_valid_reply() {
        let (r, sent) = dev(0x2006, vec![]);
        assert!(r.is_err());
        assert_eq!(sent.lock()[0], frame(&[0x20, 0x03]));
        // Réponse tronquée (63 o) ou d'un autre type : refusée.
        let mut short = reply(2, &[]);
        short.pop();
        assert!(dev(0x2006, vec![short]).0.is_err());
        let mut other = reply(2, &[]);
        other[0] = 0x67;
        assert!(dev(0x2006, vec![other]).0.is_err());
        // Une réponse étrangère puis la bonne : acceptée.
        let mut other = reply(2, &[]);
        other[0] = 0x67;
        assert!(dev(0x2006, vec![other, reply(2, &[])]).0.is_ok());
    }

    #[test]
    fn accessories_decode_to_led_counts() {
        // Canal 1 : Aer 2 (0B, 8 LED) + bande Hue 2 (04, 10 LED) ; canal 2 : vide ; canal 3 : ID inconnu 0x7F.
        let r = reply(3, &[[0x0B, 0x04, 0, 0, 0, 0], [0; 6], [0x7F, 0, 0, 0, 0, 0]]);
        assert_eq!(parse_accessories(&r, 4), vec![Some(18), Some(0), None, Some(0)]);
        // Un inconnu après un connu invalide le canal entier (l'ordre des LED serait faux).
        let r = reply(1, &[[0x0B, 0x7F, 0, 0, 0, 0]]);
        assert_eq!(parse_accessories(&r, 1), vec![None]);
        // F360 Core x2 = 48 LED, plafonné à 40.
        let r = reply(1, &[[0x1D, 0x1D, 0, 0, 0, 0]]);
        assert_eq!(parse_accessories(&r, 1), vec![Some(40)]);
    }

    #[test]
    fn direct_colors_are_grb_in_three_packets_per_channel() {
        // Canal 1 : 2 accessoires Aer 2 = 16 LED ; canal 2 : vide. Zones du modèle : 10 LED chacune.
        let r = reply(2, &[[0x0B, 0x0B, 0, 0, 0, 0], [0; 6]]);
        let (hid, sent, _) = MockHid::new(vec![r]);
        let mut model = probe(VID, 0x2006).unwrap();
        model.zones[0].led_count = 16;
        let mut d = open(Box::new(hid), &model).unwrap();
        let mut colors = vec![Color::new(1, 2, 3); 16];
        colors[0] = Color::new(0x10, 0x20, 0x30);
        colors.extend(vec![Color::new(9, 9, 9); 10]); // zone du canal 2 : jamais écrite
        d.set_colors(&colors).unwrap();
        let s = sent.lock();
        assert_eq!(s.len(), 1 + 3, "requête + 3 paquets, canal vide ignoré");
        // Trame 1 écrite à la main : 22 10 mask=01 00, LED0 = G,R,B = 20 10 30, LED1..15 = 02 01 03, puis noir.
        let mut p1 = vec![0x22, 0x10, 0x01, 0x00, 0x20, 0x10, 0x30];
        for _ in 1..16 {
            p1.extend_from_slice(&[0x02, 0x01, 0x03]);
        }
        assert_eq!(s[1], frame(&p1));
        assert_eq!(s[2], frame(&[0x22, 0x11, 0x01, 0x00]), "2e page : 20 LED éteintes");
        assert_eq!(s[3], frame(&[0x22, 0xA0, 0x01, 0x00, 0x01, 0, 0, 0, 0, 0, 0x64, 0x00, 0x32, 0x00, 0x00, 0x01]));
    }

    #[test]
    fn leds_beyond_the_detected_accessories_are_not_lit() {
        let r = reply(2, &[[0x11, 0, 0, 0, 0, 0], [0x15, 0x15, 0, 0, 0, 0]]); // logo (1 LED) ; 2 x 20 LED
        let (d, sent) = dev(0x2006, vec![r]);
        let mut d = d.unwrap();
        d.set_colors(&[Color::new(255, 0, 0); 20]).unwrap(); // zone 1 = 10 LED, zone 2 = 10 LED
        let s = sent.lock();
        assert_eq!(s.len(), 1 + 6);
        // Canal 1 : seule la LED 0 est allumée (G,R,B = 00 FF 00).
        assert_eq!(s[1], frame(&[0x22, 0x10, 0x01, 0x00, 0x00, 0xFF, 0x00]));
        // Canal 2 (masque 02) : 10 LED rouges, reste noir.
        let mut p2 = vec![0x22, 0x10, 0x02, 0x00];
        for _ in 0..10 {
            p2.extend_from_slice(&[0x00, 0xFF, 0x00]);
        }
        assert_eq!(s[4], frame(&p2));
    }

    #[test]
    fn unknown_accessory_zone_is_ignored_and_empty_input_is_fine() {
        let (d, sent) = dev(0x2006, vec![reply(2, &[[0x7F, 0, 0, 0, 0, 0], [0; 6]])]);
        let mut d = d.unwrap();
        d.set_colors(&[Color::new(1, 1, 1); 20]).unwrap();
        d.set_colors(&[]).unwrap();
        assert_eq!(sent.lock().len(), 1, "seule la requête d'accessoires est partie");
    }

    #[test]
    fn six_channel_masks_double_each_time() {
        let slot = [0x11, 0, 0, 0, 0, 0];
        let (d, sent) = dev(0x2022, vec![reply(6, &[slot; 6])]);
        let mut d = d.unwrap();
        d.set_colors(&[Color::BLACK; 60]).unwrap();
        let masks: Vec<u8> = sent.lock().iter().skip(1).step_by(3).map(|f| f[3]).collect();
        assert_eq!(masks, vec![0x01, 0x02, 0x04, 0x08, 0x10, 0x20]);
    }

    /// Rapport de statut `67 02` fabriqué : (mode, duty) par canal.
    fn status(fans: &[(u8, u8)]) -> Vec<u8> {
        let mut r = vec![0u8; 64];
        r[0] = 0x67;
        r[1] = 0x02;
        for (c, (m, d)) in fans.iter().enumerate() {
            r[16 + c] = *m;
            r[40 + c] = *d;
        }
        r
    }

    #[test]
    fn open_scan_colors_and_release_without_user_setting_write_no_fan_speed() {
        let (d, sent) = dev(0x2006, vec![reply(2, &[[0x0B, 0, 0, 0, 0, 0], [0x0B, 0, 0, 0, 0, 0]])]);
        let mut d = d.unwrap();
        assert!(!d.fans_taken());
        d.set_colors(&[Color::new(5, 5, 5); 20]).unwrap();
        d.release_fans();
        d.release();
        assert!(sent.lock().iter().all(|f| f[1] != 0x62 && f[1] != 0x60), "aucune trame de vitesse ni de statut");
    }

    #[test]
    fn fan_frames_bounds_and_restore_of_captured_duty() {
        // Statut d'origine : canal 1 à 40 % (PWM), canal 2 à 65 % (DC), canal 3 absent.
        let (d, sent) = dev(0x2006, vec![reply(2, &[]), status(&[(2, 40), (1, 65), (0, 0)])]);
        let mut d = d.unwrap();
        d.set_fan_duty(0, 50).unwrap();
        {
            let s = sent.lock();
            // Statut lu une fois, AVANT la 1re consigne.
            assert_eq!(s[1], frame(&[0x60, 0x02, 0x01, 0xE8, 0x01, 0x01, 0xE8, 0x01]));
            assert_eq!(s[2], frame(&[0x60, 0x03]));
        }
        d.set_fan_duty(2, 100).unwrap();
        d.set_fan_duty(1, 0).unwrap(); // sous le minimum : relevé à 20 %
        d.set_fan_duty(1, 5).unwrap(); // même consigne après relèvement : rien d'écrit
        d.set_fan_duty(0, 250).unwrap(); // au-dessus : 100 %
        assert!(d.set_fan_duty(3, 50).is_err());
        assert!(d.fans_taken());
        let n = sent.lock().len();
        d.release();
        let s = sent.lock();
        assert_eq!(s[3], frame(&[0x62, 0x01, 0x01, 0x32, 0x00, 0x00]));
        assert_eq!(s[4], frame(&[0x62, 0x01, 0x04, 0x00, 0x00, 0x64]));
        assert_eq!(s[5], frame(&[0x62, 0x01, 0x02, 0x00, 0x14, 0x00]));
        assert_eq!(s[6], frame(&[0x62, 0x01, 0x01, 0x64, 0x00, 0x00]));
        // Restauration : canal 1 -> 40 %, canal 2 -> 65 % ; canal 3 (absent à l'origine) : rien d'écrit.
        assert_eq!(s.len(), n + 2);
        assert_eq!(s[n], frame(&[0x62, 0x01, 0x01, 0x28, 0x00, 0x00]));
        assert_eq!(s[n + 1], frame(&[0x62, 0x01, 0x02, 0x00, 0x41, 0x00]));
    }

    #[test]
    fn release_writes_nothing_when_the_original_state_is_unreadable_and_never_100_percent() {
        // Pas de réponse `67 02` : rien de capturé -> la restauration n'écrit rien (surtout pas 100 %).
        let (d, sent) = dev(0x2006, vec![reply(2, &[])]);
        let mut d = d.unwrap();
        d.set_fan_duty(0, 30).unwrap();
        let n = sent.lock().len();
        d.release();
        d.release_fans();
        assert_eq!(sent.lock().len(), n);
        // Statut tronqué (63 o) : ignoré aussi.
        let mut short = status(&[(2, 40)]);
        short.pop();
        let (d, sent) = dev(0x2006, vec![reply(2, &[]), short]);
        let mut d = d.unwrap();
        d.set_fan_duty(0, 30).unwrap();
        let n = sent.lock().len();
        d.release();
        assert_eq!(sent.lock().len(), n);
    }

    #[test]
    fn release_skips_a_channel_already_back_at_its_original_duty_and_is_idempotent() {
        let (d, sent) = dev(0x2006, vec![reply(2, &[]), status(&[(2, 50)])]);
        let mut d = d.unwrap();
        d.set_fan_duty(0, 50).unwrap();
        let n = sent.lock().len();
        d.release();
        d.release();
        assert_eq!(sent.lock().len(), n);
        assert!(!d.fans_taken());
    }

    #[test]
    fn fanless_model_rejects_fan_and_release_writes_nothing() {
        let (d, sent) = dev(0x2001, vec![reply(4, &[])]);
        let mut d = d.unwrap();
        assert!(d.set_fan_duty(0, 50).is_err());
        d.release();
        assert_eq!(sent.lock().len(), 1);
    }
}
