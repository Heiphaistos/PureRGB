//! Famille « coolermaster » (VID 0x2516) : ARGB gen 1 (0x1011) et ARGB Gen 2 A1 (0x0173, 0x01C9, 0x01CB).
//! Sources : docs/rgb-protocols/analyse-coolermaster-masterplus.md (faits [V] lus dans MasterPlus+ 1.9.6) et coolermaster.md.
//!
//! Paquet = 65 octets (report-id 0 + 64), octets numérotés comme les notes à partir de 1.
//! - Gen 1 : si chaque port est d'une seule couleur -> mode statique matériel ; sinon mode direct (3 trames par port, 60 LED max).
//! - Gen 2 : mode temps réel (`80 01 02 01` / `80 01 02 00`), image de 60 LED par (canal, sous-port) en 3 trames.
//!
//! Non revendiqués / INCONNU (jamais devinés) :
//! - Small ARGB (0x1000) : mode statique et octet de vitesse non confirmés. ARGB2 (0x1007) : ordre des couleurs INCONNU.
//! - RGB_BOX (0x004F) : encodage de l'octet « port » des commandes ASCII et en-tête de la trame directe INCONNUS.
//! - Gen 2 : pré-roll à 2 puis 5 trames de `coolermaster.md` absent des fonctions de MasterPlus+ (hypothèse retenue : aucun) ;
//!   cadence d'envoi, commandes de taille/type/remap (liste de remap non documentée), topologie réelle des sous-ports : INCONNU.
//! - Gen 1 : nombre de LED utile par port (`setLEDSize`) INCONNU, usage page 0xFF00 issue d'OpenRGB non reconfirmée ;
//!   octets de `setLightingUninit` INCONNUS (le retour au matériel utilise le passthrough `80 01 02`, confirmé).
//!
//! Aucune commande de sauvegarde (flash) ni de firmware n'est envoyée.

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};

const VID: u16 = 0x2516;
const PID_ARGB_GEN1: u16 = 0x1011;
const PIDS_GEN2: [u16; 2] = [0x0173, 0x01C9];
const PID_GEN2_MINI: u16 = 0x01CB;
const PACKET: usize = 65;
/// LED par image (3 trames) : 0x3C pour Gen 2, plafond retenu pour Gen 1.
const LEDS: usize = 60;
const NAME_GEN1: &str = "Cooler Master ARGB Controller";
/// Octet « zone » des ports ARGB 1 à 4 (gen 1).
const PORTS: [u8; 4] = [0x01, 0x02, 0x04, 0x08];
const MODE_STATIC: u8 = 0x0A;
/// Index de couleur « fixe » (0x10), vitesse et luminosité de l'exemple de la note.
const COLOR_FIXED: u8 = 0x10;
const SPEED: u8 = 0x02;
const BRIGHTNESS: u8 = 0xFF;
/// Sous-ports pilotés par canal Gen 2 : un seul (masque 01), comme les 60 LED par canal d'OpenRGB ; INCONNU : le nombre réel de sous-ports chaînés (masques 01..10 documentés).
const SUBPORTS: usize = 1;

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != VID {
        return None;
    }
    let (name, zones): (&str, Vec<ZoneInfo>) = match pid {
        PID_ARGB_GEN1 => (NAME_GEN1, (1..=PORTS.len()).map(|i| ZoneInfo::fixed(format!("ARGB {i}"), LEDS as u32)).collect()),
        p if PIDS_GEN2.contains(&p) => ("Cooler Master ARGB Gen 2 A1", gen2_zones(3)),
        PID_GEN2_MINI => ("Cooler Master ARGB Gen 2 A1 Mini", gen2_zones(1)),
        _ => return None,
    };
    let mut m = Model::new(name, DeviceType::LedStrip, zones);
    m.interface = Some(0); // DeviceList.cfg de MasterPlus+ : interface 0 pour tous ces PID
    if pid == PID_ARGB_GEN1 {
        m.usage_page = Some(0xFF00);
    }
    Some(m)
}

fn gen2_zones(channels: usize) -> Vec<ZoneInfo> {
    (1..=channels).flat_map(|c| (1..=SUBPORTS).map(move |s| ZoneInfo::fixed(format!("Canal {c} sous-port {s}"), LEDS as u32))).collect()
}

fn packet(bytes: &[u8]) -> [u8; PACKET] {
    let mut p = [0u8; PACKET];
    p[1..1 + bytes.len()].copy_from_slice(bytes);
    p
}

/// Image de `LEDS` LED en 3 trames : chaque en-tête est posé dès l'octet 1, puis le flux continu R,G,B remplit le reste.
fn image_frames(headers: [&[u8]; 3], colors: &[Color]) -> [[u8; PACKET]; 3] {
    let stream: Vec<u8> = colors.iter().flat_map(|c| [c.r, c.g, c.b]).collect();
    let mut at = 0;
    headers.map(|h| {
        let mut p = packet(h);
        for b in &mut p[1 + h.len()..] {
            *b = stream.get(at).copied().unwrap_or(0);
            at += 1;
        }
        p
    })
}

/// Découpe le tampon en `zones` images de 60 LED (complétées en noir) ; refus si trop long, avant toute écriture.
fn pad(colors: &[Color], zones: usize) -> Result<Vec<Vec<Color>>> {
    if colors.len() > zones * LEDS {
        bail!("Cooler Master : {} couleurs pour {} LED", colors.len(), zones * LEDS);
    }
    Ok((0..zones).map(|z| (0..LEDS).map(|i| colors.get(z * LEDS + i).copied().unwrap_or(Color::BLACK)).collect()).collect())
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    if model.name == NAME_GEN1 {
        // Fonction « normal » (01) ; le passthrough carte mère serait 02.
        hid.write(&packet(&[0x80, 0x01, 0x01]))?;
        return Ok(Box::new(Argb { hid, last: [None; 4], direct: false }));
    }
    // Entrée en mode temps réel Gen 2 (note : `80 01 02 01`).
    hid.write(&packet(&[0x80, 0x01, 0x02, 0x01]))?;
    Ok(Box::new(Gen2 { hid, last: vec![None; model.zones.len()] }))
}

// ---------------------------------------------------------------- gen 1

struct Argb {
    hid: Box<dyn Hid>,
    last: [Option<Color>; 4],
    direct: bool,
}

/// Modes acceptés par MasterPlus+ pour l'octet 5 : 0..=6 et 8..=11 (7 refusé ; 12 = passthrough, trame distincte).
fn check_mode(mode: u8) -> Result<()> {
    if mode > 11 || mode == 7 {
        bail!("Cooler Master ARGB : mode {mode} refusé (0-6 et 8-11 seulement)");
    }
    Ok(())
}

impl Argb {
    /// `80 0B 02 <zone> <mode> <index couleur> <vitesse> <luminosité> R G B`.
    fn port_mode(&self, zone: u8, mode: u8, c: Color) -> Result<()> {
        check_mode(mode)?;
        self.hid.write(&packet(&[0x80, 0x0B, 0x02, zone, mode, COLOR_FIXED, SPEED, BRIGHTNESS, c.r, c.g, c.b]))?;
        Ok(())
    }

    fn set_static(&mut self, ports: &[Vec<Color>]) -> Result<()> {
        if self.direct {
            // Sortie du mode direct avant un mode normal (la note envoie `80 01 01` avant chaque mode).
            self.hid.write(&packet(&[0x80, 0x01, 0x01]))?;
            self.direct = false;
            self.last = [None; 4];
        }
        for (i, &zone) in PORTS.iter().enumerate() {
            let c = ports[i][0];
            if self.last[i] != Some(c) {
                self.port_mode(zone, MODE_STATIC, c)?;
                self.last[i] = Some(c);
            }
        }
        Ok(())
    }

    fn set_direct(&mut self, ports: &[Vec<Color>]) -> Result<()> {
        if !self.direct {
            self.hid.write(&packet(&[0x80, 0x01, 0x01, 0x02]))?;
            self.hid.write(&packet(&[0x80, 0x01, 0x03, 0x02]))?;
            self.direct = true;
        }
        for (i, leds) in ports.iter().enumerate() {
            // [4] = 0x30 : constante du logiciel officiel (ne dépend pas du nombre de LED).
            let port = u8::try_from(i)?;
            for f in image_frames([&[0x00, 0x07, port, 0x30], &[0x01], &[0x82]], leds) {
                self.hid.write(&f)?;
            }
        }
        Ok(())
    }
}

impl NativeRgb for Argb {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        let ports = pad(colors, PORTS.len())?;
        if ports.iter().all(|p| p.iter().all(|c| *c == p[0])) {
            self.set_static(&ports)
        } else {
            self.set_direct(&ports)
        }
    }

    fn release(&mut self) {
        // Passthrough carte mère (mode 12 : une seule trame `80 01 02`) ; rien à rattraper si l'envoi échoue à la fermeture.
        let _ = self.hid.write(&packet(&[0x80, 0x01, 0x02]));
    }
}

// ---------------------------------------------------------------- gen 2

struct Gen2 {
    hid: Box<dyn Hid>,
    last: Vec<Option<Vec<Color>>>,
}

impl NativeRgb for Gen2 {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        let zones = pad(colors, self.last.len())?;
        for (z, leds) in zones.iter().enumerate() {
            if self.last[z].as_ref() == Some(leds) {
                continue;
            }
            // Masque de canal 01 << (canal-1), masque de sous-port 01 << (sous-port-1), 0x3C = 60 LED.
            let (chan, sub) = (1u8 << (z / SUBPORTS), 1u8 << (z % SUBPORTS));
            for f in image_frames([&[0x00, 0x09, chan, sub, 0x3C], &[0x01, 0x09], &[0x82, 0x09]], leds) {
                self.hid.write(&f)?;
            }
            self.last[z] = Some(leds.clone());
        }
        Ok(())
    }

    fn release(&mut self) {
        // Sortie du mode temps réel (`80 01 02 00`).
        let _ = self.hid.write(&packet(&[0x80, 0x01, 0x02, 0x00]));
    }
}

pub const FAMILY: Family = Family { id: "coolermaster", doc: "docs/rgb-protocols/analyse-coolermaster-masterplus.md", probe, open };

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::hid::native::{probe as global_probe, MockHid};

    type Sent = std::sync::Arc<parking_lot::Mutex<Vec<Vec<u8>>>>;

    fn pkt(bytes: &[u8]) -> Vec<u8> {
        let mut v = vec![0u8; 65];
        v[..bytes.len()].copy_from_slice(bytes);
        v
    }

    /// LED i -> (i, 100+i, 200-i) : chaque LED est reconnaissable dans les trames.
    fn ramp(n: usize) -> Vec<Color> {
        (0..n).map(|i| Color::new(i as u8, 100u8.wrapping_add(i as u8), 200u8.wrapping_sub(i as u8))).collect()
    }

    fn open_dev(pid: u16) -> (Box<dyn NativeRgb>, Sent) {
        let (mock, sent, _) = MockHid::new(vec![]);
        let m = probe(VID, pid).expect("pid");
        (open(Box::new(mock), &m).expect("open"), sent)
    }

    #[test]
    fn probe_claims_only_documented_pids() {
        for pid in [0x1000, 0x1007, 0x004F, 0x010B, 0x0155, 0x020A, 0x01C1] {
            assert!(probe(VID, pid).is_none(), "{pid:04x}");
        }
        assert!(probe(0x1532, 0x1011).is_none());
        let m = probe(VID, 0x1011).expect("1011");
        assert_eq!((m.interface, m.usage_page, m.led_count(), m.zones.len()), (Some(0), Some(0xFF00), 240, 4));
        assert_eq!(global_probe(VID, 0x1011).map(|(f, _)| f.id), Some("coolermaster"));
        for pid in [0x0173, 0x01C9] {
            let m = probe(VID, pid).expect("gen2");
            assert_eq!((m.interface, m.usage_page, m.zones.len(), m.led_count()), (Some(0), None, 3, 180));
            assert_eq!(global_probe(VID, pid).map(|(f, _)| f.id), Some("coolermaster"));
        }
        assert_eq!(probe(VID, 0x01CB).expect("mini").zones.len(), 1);
    }

    #[test]
    fn gen1_uniform_ports_use_hardware_static_mode() {
        let (mut dev, sent) = open_dev(0x1011);
        let mut c = vec![Color::BLACK; 240];
        c[..60].fill(Color::new(0xFF, 0, 0));
        dev.set_colors(&c).expect("set");
        let s = sent.lock();
        assert_eq!(s[0], pkt(&[0x00, 0x80, 0x01, 0x01]));
        assert_eq!(s[1], pkt(&[0x00, 0x80, 0x0B, 0x02, 0x01, 0x0A, 0x10, 0x02, 0xFF, 0xFF, 0x00, 0x00]));
        assert_eq!(s[2], pkt(&[0x00, 0x80, 0x0B, 0x02, 0x02, 0x0A, 0x10, 0x02, 0xFF, 0, 0, 0]));
        assert_eq!(s[3], pkt(&[0x00, 0x80, 0x0B, 0x02, 0x04, 0x0A, 0x10, 0x02, 0xFF, 0, 0, 0]));
        assert_eq!(s[4], pkt(&[0x00, 0x80, 0x0B, 0x02, 0x08, 0x0A, 0x10, 0x02, 0xFF, 0, 0, 0]));
        assert_eq!(s.len(), 5);
    }

    #[test]
    fn gen1_unchanged_static_is_not_resent_and_oversize_is_refused_silently() {
        let (mut dev, sent) = open_dev(0x1011);
        let c = [Color::new(1, 2, 3); 240];
        dev.set_colors(&c).expect("set");
        let n = sent.lock().len();
        dev.set_colors(&c).expect("set");
        assert_eq!(sent.lock().len(), n);
        assert!(dev.set_colors(&[Color::BLACK; 241]).is_err());
        assert_eq!(sent.lock().len(), n, "rien envoyé sur un tampon trop long");
    }

    #[test]
    fn gen1_per_led_colors_use_direct_mode_with_three_frames_per_port() {
        let (mut dev, sent) = open_dev(0x1011);
        let mut c = vec![Color::BLACK; 240];
        c[60..120].copy_from_slice(&ramp(60)); // port 2 uniquement
        dev.set_colors(&c).expect("set");
        let s = sent.lock();
        assert_eq!(s[1], pkt(&[0x00, 0x80, 0x01, 0x01, 0x02]));
        assert_eq!(s[2], pkt(&[0x00, 0x80, 0x01, 0x03, 0x02]));
        assert_eq!(s.len(), 3 + 4 * 3);
        // port 1 (noir) : en-têtes seuls
        assert_eq!(s[3], pkt(&[0x00, 0x00, 0x07, 0x00, 0x30]));
        assert_eq!(s[4], pkt(&[0x00, 0x01]));
        assert_eq!(s[5], pkt(&[0x00, 0x82]));
        // port 2 = index 1
        let (f1, f2, f3) = (&s[6], &s[7], &s[8]);
        assert_eq!(f1.len(), 65);
        assert_eq!(f1[..8], [0x00, 0x00, 0x07, 0x01, 0x30, 0, 100, 200]); // LED 0
        assert_eq!(f1[8..11], [1, 101, 199]); // LED 1
        assert_eq!(f1[62..65], [19, 119, 181]); // LED 19 = dernière de la trame 1
        assert_eq!(f2[..5], [0x00, 0x01, 20, 120, 180]); // LED 20 dès l'octet 2
        assert_eq!(f2[62..65], [40, 140, 160]); // LED 40
        assert_eq!(f3[..5], [0x00, 0x82, 41, 141, 159]); // LED 41 dès l'octet 2
        assert_eq!(f3[56..59], [59, 159, 141]); // LED 59
        assert!(f3[59..].iter().all(|&b| b == 0));
        // ports 3 et 4 : octets de port 02 et 03
        assert_eq!(s[9][..5], [0x00, 0x00, 0x07, 0x02, 0x30]);
        assert_eq!(s[12][..5], [0x00, 0x00, 0x07, 0x03, 0x30]);
    }

    #[test]
    fn gen1_back_to_uniform_leaves_direct_mode_then_sends_static() {
        let (mut dev, sent) = open_dev(0x1011);
        dev.set_colors(&ramp(240)).expect("direct");
        sent.lock().clear();
        dev.set_colors(&[Color::new(0, 0, 9); 240]).expect("static");
        let s = sent.lock();
        assert_eq!(s[0], pkt(&[0x00, 0x80, 0x01, 0x01]));
        assert_eq!(s[1], pkt(&[0x00, 0x80, 0x0B, 0x02, 0x01, 0x0A, 0x10, 0x02, 0xFF, 0, 0, 9]));
        assert_eq!(s.len(), 5);
    }

    #[test]
    fn gen1_mode_validation_rejects_7_and_out_of_range() {
        let (mock, sent, _) = MockHid::new(vec![]);
        let a = Argb { hid: Box::new(mock), last: [None; 4], direct: false };
        for m in [7u8, 12, 0xFF] {
            assert!(a.port_mode(1, m, Color::BLACK).is_err(), "{m}");
        }
        assert!(sent.lock().is_empty());
        for m in (0..=6).chain(8..=11) {
            assert!(a.port_mode(1, m, Color::BLACK).is_ok(), "{m}");
        }
        assert_eq!(sent.lock().len(), 11);
        assert_eq!(sent.lock()[0], pkt(&[0x00, 0x80, 0x0B, 0x02, 0x01, 0x00, 0x10, 0x02, 0xFF, 0, 0, 0]));
    }

    #[test]
    fn gen1_release_is_a_single_passthrough_frame() {
        let (mut dev, sent) = open_dev(0x1011);
        dev.release();
        let s = sent.lock();
        assert_eq!(s.len(), 2);
        assert_eq!(s[1], pkt(&[0x00, 0x80, 0x01, 0x02]));
    }

    #[test]
    fn gen2_open_enters_realtime_and_release_leaves_it() {
        let (mut dev, sent) = open_dev(0x01C9);
        assert_eq!(sent.lock().as_slice(), [pkt(&[0x00, 0x80, 0x01, 0x02, 0x01])]);
        dev.release();
        assert_eq!(sent.lock()[1], pkt(&[0x00, 0x80, 0x01, 0x02, 0x00]));
        assert_eq!(sent.lock().len(), 2);
    }

    #[test]
    fn gen2_image_is_three_frames_with_continuous_stream() {
        let (mut dev, sent) = open_dev(0x0173);
        dev.set_colors(&ramp(60)).expect("set"); // zone 0 = canal 1 / sous-port 1
        let s = sent.lock();
        // les 2 autres canaux sont noirs mais envoyés une fois (état initial inconnu)
        assert_eq!(s.len(), 1 + 3 * 3);
        let (f1, f2, f3) = (&s[1], &s[2], &s[3]);
        assert_eq!(f1[..9], [0x00, 0x00, 0x09, 0x01, 0x01, 0x3C, 0, 100, 200]); // LED 0 dès l'octet 6
        assert_eq!(f1[60..62], [18, 118]); // début de la LED 18
        assert_eq!(f1[63..65], [19, 119]); // R,G de la LED 19
        assert_eq!(f2[..6], [0x00, 0x01, 0x09, 181, 20, 120]); // B de la LED 19 puis LED 20 dès l'octet 4
        assert_eq!(f2[64], 40); // R de la LED 40
        assert_eq!(f3[..7], [0x00, 0x82, 0x09, 140, 160, 41, 141]); // G,B de la LED 40 puis LED 41 dès l'octet 5
        assert_eq!(f3[59..62], [59, 159, 141]); // LED 59
        assert!(f3[62..].iter().all(|&b| b == 0));
    }

    #[test]
    fn gen2_masks_per_channel_and_subport() {
        let (mut dev, sent) = open_dev(0x01C9);
        dev.set_colors(&[]).expect("set");
        let s = sent.lock();
        let heads: Vec<[u8; 2]> = s[1..].chunks(3).map(|f| [f[0][3], f[0][4]]).collect();
        let expect: Vec<[u8; 2]> = [0x01u8, 0x02, 0x04].iter().map(|&c| [c, 0x01]).collect();
        assert_eq!(heads, expect);
        assert!(s[1..].chunks(3).all(|f| f[0][5] == 0x3C && f[1][1] == 0x01 && f[2][1] == 0x82));
    }

    #[test]
    fn gen2_mini_has_one_channel_and_unchanged_zones_are_not_resent() {
        let (mut dev, sent) = open_dev(0x01CB);
        let c = ramp(60);
        dev.set_colors(&c).expect("set");
        assert_eq!(sent.lock().len(), 1 + 3);
        assert_eq!(sent.lock()[1][3..5], [0x01, 0x01]);
        dev.set_colors(&c).expect("set");
        assert_eq!(sent.lock().len(), 1 + 3);
    }

    #[test]
    fn gen2_oversize_buffer_is_refused_without_sending() {
        let (mut dev, sent) = open_dev(0x01CB);
        assert!(dev.set_colors(&[Color::BLACK; 301]).is_err());
        assert_eq!(sent.lock().len(), 1, "seule l'entrée en mode temps réel a été envoyée");
    }
}
