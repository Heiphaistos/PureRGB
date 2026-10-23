//! Claviers Roccat / Turtle Beach à deux collections HID : Vulcan 100 Aimo (0x307A), Vulcan 120 Aimo (0x3098), Magma (0x3124), Magma Mini (0x69A0).
//! Sources : docs/rgb-protocols/analyse-roccat-swarm.md (§ 2, 3), docs/rgb-protocols/roccat-turtlebeach.md (source O = OpenRGB, NON
//! confirmée par les binaires officiels : tout est à valider sur matériel).
//!
//! - Deux collections : led (collection principale du `Model`, écriture de 65 octets) et ctrl (`Hid::aux()`, rapports feature).
//! - Ouverture : lecture de l'état sur ctrl (feature 0x0F de 8 octets en v1, 0x09 de 9 octets en v2), faite deux fois ; l'octet 1 doit
//!   valoir 1 (prêt), l'octet 2 est la version du firmware (`major = b / 100`, `minor = b % 100`). Puis prise de contrôle : v1 `15 00 01`,
//!   v2 `0E 05 01 00 00` ; sortie = mêmes trames avec 0.
//! - Trame couleur : flux de `length` octets `A1 <n°> <longueur 1 octet si <= 255, sinon 2 octets> <données>`, découpé en paquets de 64 octets
//!   (`ceil(length / 64)`) posés à partir de l'octet 1 d'un rapport de 65 octets. Longueur sur 2 octets : poids fort d'abord si firmware >= 1.16,
//!   poids faible d'abord sinon. Données : trois plans R, G, B par colonne de `column_length` LEDs, LED i -> colonne i / cl, ligne i % cl,
//!   R à `en-tête + colonne*3*cl + ligne`, G à `+cl`, B à `+2*cl`. Vulcan 100/120 : 436 octets, cl 12, 144 LEDs d'index colonne*6 + ligne
//!   (matrice 6 x 24). Magma : 64 octets, cl 5, 5 LEDs (0..4).
//! - Après les paquets : lecture de réponse avec 20 ms de délai.
//!
//! INCONNU / à ne pas deviner : noms des touches (layouts non transcrits), Vulcan 121 et Kain (PID inconnus), Pyro (314C : nombre de LEDs
//! et layout non donnés), Vulcan Pro/II/TKL (autres variantes v2 multi-paquets : la note dit « n° 1..N sur chaque paquet » ET « ceil(longueur/64)
//! paquets » sans dire comment les paquets suivants portent leur en-tête, donc non revendiqués). Pour le Vulcan 100/120 (v1, n° `0x01` sur le
//! premier paquet seulement) les paquets suivants sont les tranches brutes du flux : c'est l'interprétation retenue, à valider. Aucun mode
//! matériel (rapports 0x0D/0x11), aucune écriture de profil.

use crate::backends::hid::native::{Hid, HidSelect, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{anyhow, bail, Result};

const PID_VULCAN_100: u16 = 0x307A;
const PID_VULCAN_120: u16 = 0x3098;
const PID_MAGMA: u16 = 0x3124;
const PID_MAGMA_MINI: u16 = 0x69A0;
/// Premier firmware dont la longueur (2 octets) est envoyée poids fort d'abord : 1.16 -> 116.
const FIRMWARE_BIG_ENDIAN: u8 = 116;

/// Variantes documentées.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    /// Vulcan 100 / 120 Aimo : protocole v1.
    VulcanV1,
    /// Magma / Magma Mini : protocole v2, un seul paquet.
    Magma,
}

impl Kind {
    fn leds(self) -> usize {
        match self {
            Kind::VulcanV1 => 144,
            Kind::Magma => 5,
        }
    }
    /// Longueur de trame (champ d'en-tête).
    fn length(self) -> usize {
        match self {
            Kind::VulcanV1 => 436,
            Kind::Magma => 64,
        }
    }
    fn column_length(self) -> usize {
        match self {
            Kind::VulcanV1 => 12,
            Kind::Magma => 5,
        }
    }
    /// (identifiant, taille) du rapport d'état lu sur ctrl.
    fn status_report(self) -> (u8, usize) {
        match self {
            Kind::VulcanV1 => (0x0F, 8),
            Kind::Magma => (0x09, 9),
        }
    }
    /// Trame de prise de contrôle (`on` = true) ou de sortie.
    fn control(self, on: bool) -> Vec<u8> {
        match self {
            Kind::VulcanV1 => vec![0x15, 0x00, on as u8],
            Kind::Magma => vec![0x0E, 0x05, on as u8, 0x00, 0x00],
        }
    }
    fn from_leds(leds: u32) -> Option<Kind> {
        [Kind::VulcanV1, Kind::Magma].into_iter().find(|k| k.leds() == leds as usize)
    }
}

/// (variante, nom, led (interface, page), ctrl (interface, page)) d'après la note.
fn kind_of(pid: u16) -> Option<(Kind, &'static str, (i32, u16), (i32, u16))> {
    match pid {
        PID_VULCAN_100 => Some((Kind::VulcanV1, "Roccat Vulcan 100 Aimo", (3, 0x0001), (1, 0x000B))),
        PID_VULCAN_120 => Some((Kind::VulcanV1, "Roccat Vulcan 120 Aimo", (3, 0x0001), (1, 0x000B))),
        PID_MAGMA => Some((Kind::Magma, "Roccat Magma", (3, 0xFF00), (1, 0xFF01))),
        PID_MAGMA_MINI => Some((Kind::Magma, "Roccat Magma Mini", (3, 0xFF00), (1, 0xFF01))),
        _ => None,
    }
}

pub fn probe(pid: u16) -> Option<Model> {
    let (kind, name, led, ctrl) = kind_of(pid)?;
    let mut m = Model::new(name, DeviceType::Keyboard, vec![ZoneInfo::fixed("Touches", kind.leds() as u32)]);
    m.interface = Some(led.0);
    m.usage_page = Some(led.1);
    m.aux = Some(HidSelect { interface: Some(ctrl.0), usage_page: Some(ctrl.1), usage: None });
    Some(m)
}

struct Keyboard {
    /// Collection led (écritures) ; ctrl = `hid.aux()`.
    hid: Box<dyn Hid>,
    kind: Kind,
    big_endian: bool,
}

impl Keyboard {
    /// Trame complète (flux de `length` octets) pour des couleurs déjà validées (<= nombre de LEDs).
    fn frame(&self, colors: &[Color]) -> Vec<u8> {
        let (len, cl) = (self.kind.length(), self.kind.column_length());
        let mut f = vec![0u8; len];
        f[0] = 0xA1;
        f[1] = 0x01;
        let hdr = if len <= 255 {
            f[2] = len as u8;
            3
        } else {
            let (hi, lo) = ((len >> 8) as u8, len as u8);
            (f[2], f[3]) = if self.big_endian { (hi, lo) } else { (lo, hi) };
            4
        };
        for (i, c) in colors.iter().enumerate() {
            let base = hdr + (i / cl) * 3 * cl + i % cl;
            f[base] = c.r;
            f[base + cl] = c.g;
            f[base + 2 * cl] = c.b;
        }
        f
    }
}

pub fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    let kind = Kind::from_leds(model.led_count()).ok_or_else(|| anyhow!("Roccat : modèle de clavier inconnu"))?;
    let ctrl = hid.aux().ok_or_else(|| anyhow!("Roccat : seconde collection HID (feature) introuvable, clavier non pilotable"))?;
    let (id, size) = kind.status_report();
    let mut firmware = 0u8;
    // Lu deux fois : la première réponse peut être périmée, seule la seconde compte.
    for _ in 0..2 {
        let mut buf = vec![0u8; size];
        buf[0] = id;
        let n = ctrl.get_feature_report(&mut buf)?;
        if n < 3 || buf[0] != id {
            bail!("Roccat : rapport d'état 0x{id:02X} invalide ({n} octets), version du firmware illisible");
        }
        if buf[1] != 1 {
            bail!("Roccat : clavier non prêt (état {})", buf[1]);
        }
        firmware = buf[2];
    }
    ctrl.send_feature_report(&kind.control(true))?;
    Ok(Box::new(Keyboard { hid, kind, big_endian: firmware >= FIRMWARE_BIG_ENDIAN }))
}

impl NativeRgb for Keyboard {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        if colors.len() > self.kind.leds() {
            bail!("Roccat : {} couleurs pour {} LEDs", colors.len(), self.kind.leds());
        }
        for chunk in self.frame(colors).chunks(64) {
            let mut pkt = [0u8; 65];
            pkt[1..=chunk.len()].copy_from_slice(chunk);
            self.hid.write(&pkt)?;
        }
        let mut reply = [0u8; 65];
        self.hid.read_timeout(&mut reply, 20)?;
        Ok(())
    }

    fn release(&mut self) {
        // Rendre la main au mode matériel ; rien à rattraper si l'envoi échoue à la fermeture.
        if let Some(ctrl) = self.hid.aux() {
            let _ = ctrl.send_feature_report(&self.kind.control(false));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::hid::native::MockHid;

    type Log = std::sync::Arc<parking_lot::Mutex<Vec<Vec<u8>>>>;

    /// (hid, écritures led, features ctrl) ; ctrl répond `status` aux deux lectures.
    fn rig(status: Vec<u8>) -> (Box<dyn Hid>, Log, Log) {
        let (ctrl, _, ctrl_feats) = MockHid::new(vec![status.clone(), status]);
        let (led, led_sent, _) = MockHid::new(vec![]);
        (Box::new(led.with_aux(ctrl)), led_sent, ctrl_feats)
    }

    fn v1_status(fw: u8) -> Vec<u8> {
        vec![0x0F, 1, fw, 0, 0, 0, 0, 0]
    }

    #[test]
    fn probe_claims_only_documented_keyboards() {
        for pid in [0x314C, 0x2FEE, 0x30F7, 0x311A, 0x2F4E, 0x2EE2, 0x2F09, 0x3099] {
            assert!(probe(pid).is_none(), "{pid:04x}");
        }
        let m = probe(0x307A).expect("Vulcan 100");
        assert_eq!((m.interface, m.usage_page, m.usage, m.led_count()), (Some(3), Some(0x0001), None, 144));
        assert_eq!(m.aux, Some(HidSelect { interface: Some(1), usage_page: Some(0x000B), usage: None }));
        assert_eq!(probe(0x3098).expect("Vulcan 120").led_count(), 144);
        for pid in [0x3124, 0x69A0] {
            let m = probe(pid).expect("Magma");
            assert_eq!((m.interface, m.usage_page, m.led_count()), (Some(3), Some(0xFF00), 5));
            assert_eq!(m.aux, Some(HidSelect { interface: Some(1), usage_page: Some(0xFF01), usage: None }));
        }
    }

    #[test]
    fn vulcan_takes_control_with_15_00_01_then_releases() {
        let (hid, sent, feats) = rig(v1_status(116));
        let m = probe(0x3098).expect("modèle");
        let mut dev = open(hid, &m).expect("open");
        assert_eq!(feats.lock().as_slice(), [vec![0x15, 0x00, 0x01]]);
        dev.release();
        assert_eq!(feats.lock()[1], vec![0x15, 0x00, 0x00]);
        assert!(sent.lock().is_empty());
    }

    #[test]
    fn magma_takes_control_with_0e_05_01_and_sends_one_packet() {
        let status = vec![0x09, 1, 100, 0, 0, 0, 0, 0, 0];
        let (ctrl, _, feats) = MockHid::new(vec![status.clone(), status]);
        let (led, sent, _) = MockHid::new(vec![]);
        let m = probe(0x3124).expect("Magma");
        let mut dev = open(Box::new(led.with_aux(ctrl)), &m).expect("open");
        assert_eq!(feats.lock().as_slice(), [vec![0x0E, 0x05, 0x01, 0x00, 0x00]]);
        let colors = [Color::new(1, 2, 3), Color::new(4, 5, 6), Color::new(7, 8, 9), Color::new(10, 11, 12), Color::new(13, 14, 15)];
        dev.set_colors(&colors).expect("set");
        dev.release();
        assert_eq!(feats.lock()[1], vec![0x0E, 0x05, 0x00, 0x00, 0x00]);
        let s = sent.lock();
        assert_eq!(s.len(), 1);
        let mut want = vec![0x00, 0xA1, 0x01, 0x40, 1, 4, 7, 10, 13, 2, 5, 8, 11, 14, 3, 6, 9, 12, 15];
        want.resize(65, 0);
        assert_eq!(s[0], want);
    }

    fn vulcan_packets(fw: u8) -> Vec<Vec<u8>> {
        let (hid, sent, _) = rig(v1_status(fw));
        let m = probe(0x307A).expect("modèle");
        let mut dev = open(hid, &m).expect("open");
        let mut colors = vec![Color::BLACK; 144];
        colors[0] = Color::new(1, 2, 3);
        colors[13] = Color::new(4, 5, 6); // colonne 1, ligne 1 (plans de 12)
        colors[143] = Color::new(7, 8, 9); // dernière LED
        dev.set_colors(&colors).expect("set");
        let out = sent.lock().clone();
        out
    }

    #[test]
    fn vulcan_splits_436_bytes_into_seven_65_byte_packets_big_endian_from_1_16() {
        let p = vulcan_packets(116);
        assert_eq!(p.len(), 7);
        assert!(p.iter().all(|x| x.len() == 65 && x[0] == 0));
        assert_eq!(&p[0][1..5], [0xA1, 0x01, 0x01, 0xB4]); // 436 = 0x01B4, poids fort d'abord
        // LED 0 : R à 4, G à 16, B à 28 du flux -> octets 5, 17, 29 du rapport.
        assert_eq!((p[0][5], p[0][17], p[0][29]), (1, 2, 3));
        // LED 13 : R à 4+36+1 = 41, G à 53, B à 65 (flux) -> B passe dans le 2e paquet, octet 2.
        assert_eq!((p[0][42], p[0][54], p[1][2]), (4, 5, 6));
        // LED 143 : R à 4+11*36+11 = 411, G à 423, B à 435 (dernier octet du flux, 52 octets dans le 7e paquet).
        assert_eq!((p[6][1 + 411 - 384], p[6][1 + 423 - 384], p[6][1 + 435 - 384]), (7, 8, 9));
        assert!(p[6][53..].iter().all(|&b| b == 0));
        assert_eq!(p[1][1], 0); // pas de 2e en-tête : tranche brute du flux
    }

    #[test]
    fn vulcan_length_is_little_endian_before_firmware_1_16() {
        let p = vulcan_packets(115);
        assert_eq!(&p[0][1..5], [0xA1, 0x01, 0xB4, 0x01]);
    }

    #[test]
    fn too_many_colors_are_refused_without_sending() {
        let (hid, sent, feats) = rig(v1_status(116));
        let m = probe(0x3098).expect("modèle");
        let mut dev = open(hid, &m).expect("open");
        assert!(dev.set_colors(&vec![Color::BLACK; 145]).is_err());
        assert!(sent.lock().is_empty());
        assert_eq!(feats.lock().len(), 1); // seulement la prise de contrôle
    }

    #[test]
    fn missing_second_collection_is_a_clear_error_and_sends_nothing() {
        let (led, sent, feats) = MockHid::new(vec![]);
        let m = probe(0x3098).expect("modèle");
        let err = open(Box::new(led), &m).err().expect("erreur").to_string();
        assert!(err.contains("seconde collection"), "{err}");
        assert!(sent.lock().is_empty() && feats.lock().is_empty());
    }

    #[test]
    fn unreadable_or_not_ready_status_refuses_before_taking_control() {
        for status in [vec![], vec![0x0F, 1], vec![0x0F, 0, 116, 0, 0, 0, 0, 0], vec![0x09, 1, 116, 0, 0, 0, 0, 0]] {
            let (hid, _, feats) = rig(status);
            let m = probe(0x3098).expect("modèle");
            assert!(open(hid, &m).is_err());
            assert!(feats.lock().is_empty(), "aucune prise de contrôle");
        }
    }
}
