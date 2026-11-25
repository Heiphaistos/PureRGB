//! MSI Mystic Light 185 octets, mode « LED par LED » (rapport 0x53). Faits de protocole repris de l'analyse du contrôleur de référence
//! (OpenRGB, `MSIMotherboard185Controller`, lui-même fondé sur le projet mystic-why) ; table des cartes extraite par
//! `tools/gen_msi_perled_boards.py` dans `assets/msi_perled_boards.tsv` (85 cartes). Aucun code copié.
//!
//! Principe : un rapport 0x52 de 185 octets « d'activation » met les connecteurs en mode direct, puis des rapports 0x53 de 725 octets portent les
//! couleurs (5 octets d'en-tête `53 25 06 00 00` puis jusqu'à 240 LEDs RGB). Tout est volatil : rien n'est jamais enregistré en flash
//! (`save_data` = 0) ; `release()` renvoie l'état d'origine lu à l'ouverture.
//! Deux dispositions : « synchro » (JRAINBOW 1 et 2 <= 40 LEDs, JCORSAIR <= 120) = UN rapport 0x53 qui contient embarqué + JPIPE + JRGB puis les trois
//! connecteurs ARGB à la suite ; sinon un rapport par connecteur ARGB (en-tête `53 25 04 00 00` JRAINBOW 1 jusqu'à 200 LEDs, `53 25 04 01 00`
//! JRAINBOW 2 jusqu'à 240, `53 25 05 00 00` JCORSAIR jusqu'à 240, 13 ms entre rapports) puis le rapport embarqué.
//! INCONNU : comportement exact d'une carte jamais essayée (aucune carte MSI sur la machine de développement) ; sens des octets 0x02/0x03 des
//! drapeaux d'activation repris tels quels. Le mode zone (une couleur par connecteur) reste le repli si l'activation échoue.

use super::{Hid, NativeRgb};
use crate::core::{Color, ZoneInfo};
use anyhow::{bail, Result};
use std::collections::HashMap;
use std::sync::OnceLock;

const LEN: usize = 185;
const PER_LED_LEN: usize = 5 + 240 * 3;
const DIRECT_EFFECT: u8 = 0x25;
const SYNC_RAINBOW: usize = 40;
const SYNC_CORSAIR: usize = 120;
const R1_MAX: u32 = 200;
const R2_MAX: u32 = 240;
const CORSAIR_MAX: u32 = 240;

/// Noms des zones exposées (ordre d'exposition = ordre de découpe du tampon de couleurs).
const ONBOARD: &str = "LED carte";
const PIPE1: &str = "JPIPE 1";
const PIPE2: &str = "JPIPE 2";
const JRGB: &str = "JRGB";
const RAINBOW1: &str = "JRAINBOW 1";
const RAINBOW2: &str = "JRAINBOW 2";
const CORSAIR: &str = "JCORSAIR";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    pub onboard: usize,
    pub pipe1: usize,
    pub pipe2: usize,
    pub jrgbs: usize,
    pub has_onboard: bool,
    pub has_pipe1: bool,
    pub has_pipe2: bool,
    pub has_jrgb: bool,
    pub has_rainbow1: bool,
    pub has_rainbow2: bool,
    pub has_corsair: bool,
}

fn parse(text: &str) -> HashMap<u16, Board> {
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            let pid = u16::from_str_radix(f.first()?, 16).ok()?;
            let n = |i: usize| f.get(i)?.parse::<usize>().ok();
            let zones = *f.get(5)?;
            let has = |z: &str| zones.split(',').any(|x| x == z);
            Some((
                pid,
                Board {
                    onboard: n(1)?,
                    pipe1: n(2)?,
                    pipe2: n(3)?,
                    jrgbs: n(4)?,
                    has_onboard: has("ON_BOARD_LED_0"),
                    has_pipe1: has("J_PIPE_1"),
                    has_pipe2: has("J_PIPE_2"),
                    has_jrgb: has("J_RGB_1"),
                    has_rainbow1: has("J_RAINBOW_1"),
                    has_rainbow2: has("J_RAINBOW_2"),
                    has_corsair: has("J_CORSAIR") || has("J_RAINBOW_3"),
                },
            ))
        })
        .collect()
}

/// Carte à mode LED par LED connue pour ce PID.
pub fn board(pid: u16) -> Option<Board> {
    static T: OnceLock<HashMap<u16, Board>> = OnceLock::new();
    T.get_or_init(|| parse(include_str!("../../../../assets/msi_perled_boards.tsv"))).get(&pid).cloned()
}

/// Défaut de 60 LEDs par connecteur ARGB (comme le serveur OpenRGB de référence), réglable de 1 au maximum du connecteur : c'est ce nombre
/// qui fait du pilote intégré le pilote principal face à OpenRGB (le registre retient celui qui offre le plus de LEDs).
const ARGB_DEFAULT: u32 = 60;

fn argb(name: &str, max: u32) -> ZoneInfo {
    ZoneInfo { name: name.into(), led_count: ARGB_DEFAULT, zone_type: 1, leds_min: 1, leds_max: max }
}

/// Zones exposées pour cette carte, dans l'ordre : embarqué, JPIPE 1, JPIPE 2, JRGB, JRAINBOW 1, JRAINBOW 2, JCORSAIR.
pub fn zones(b: &Board) -> Vec<ZoneInfo> {
    let mut z = Vec::new();
    if b.has_onboard && b.onboard > 0 {
        z.push(ZoneInfo::fixed(ONBOARD, b.onboard as u32));
    }
    if b.has_pipe1 && b.pipe1 > 0 {
        z.push(ZoneInfo::fixed(PIPE1, b.pipe1 as u32));
    }
    if b.has_pipe2 && b.pipe2 > 0 {
        z.push(ZoneInfo::fixed(PIPE2, b.pipe2 as u32));
    }
    if b.has_jrgb && b.jrgbs > 0 {
        z.push(ZoneInfo::fixed(JRGB, b.jrgbs as u32));
    }
    if b.has_rainbow1 {
        z.push(argb(RAINBOW1, R1_MAX));
    }
    if b.has_rainbow2 {
        z.push(argb(RAINBOW2, R2_MAX));
    }
    if b.has_corsair {
        z.push(argb(CORSAIR, CORSAIR_MAX));
    }
    z
}

/// Offset (dans le rapport 0x52) du premier octet de chaque bloc de zone.
const Z_JRGB1: usize = 1;
const Z_PIPE1: usize = 11;
const Z_PIPE2: usize = 21;
const Z_RAINBOW1: usize = 31;
const Z_RAINBOW2: usize = 42;
const Z_CORSAIR: usize = 53;
const Z_OUTER: usize = 64;
const Z_ONBOARD: usize = 74;
const Z_JRGB2: usize = 174;

/// Rapport 0x52 d'activation du mode LED par LED. `sync` = disposition synchronisée (voir l'en-tête du module).
pub fn enable_message(b: &Board, sync: bool) -> [u8; LEN] {
    let mut m = [0u8; LEN];
    m[0] = 0x52;
    // Tous les blocs de zone : effet statique par défaut (1) ; chaque bloc fait 10 octets (11 pour JRAINBOW et JCORSAIR).
    for z in [Z_JRGB1, Z_PIPE1, Z_PIPE2, Z_RAINBOW1, Z_RAINBOW2, Z_CORSAIR, Z_OUTER, Z_ONBOARD, Z_JRGB2] {
        m[z] = 1;
    }
    for i in 1..=9 {
        m[Z_ONBOARD + 10 * i] = 1;
    }
    let flags = |m: &mut [u8; LEN], z: usize, sb: u8, cf: u8| {
        m[z + 4] = sb;
        m[z + 8] = cf;
    };
    flags(&mut m, Z_JRGB1, 0x08, 0x80);
    flags(&mut m, Z_PIPE1, 0x2A, 0x80);
    flags(&mut m, Z_PIPE2, 0x2A, 0x80);
    flags(&mut m, Z_RAINBOW1, 0x29, 0x80);
    m[Z_RAINBOW1 + 10] = 0x28;
    flags(&mut m, Z_RAINBOW2, 0x29, 0x80);
    m[Z_RAINBOW2 + 10] = 0x28;
    m[Z_CORSAIR + 4] = 0x29; // fan_flags
    m[Z_CORSAIR + 5] = 0x00; // corsair_quantity
    m[Z_CORSAIR + 8] = 0x82; // padding[2]
    m[Z_CORSAIR + 10] = 0x78; // is_individual
    flags(&mut m, Z_OUTER, 0x28, 0x80);
    m[Z_ONBOARD] = DIRECT_EFFECT;
    m[Z_ONBOARD + 4] = 0x29 | 0x80;
    for i in 1..=9 {
        flags(&mut m, Z_ONBOARD + 10 * i, 0x28, 0x80);
    }
    flags(&mut m, Z_JRGB2, 0x2A, 0x80);
    // PER_LED_BASIC_SYNC_MODE = custom | onboard | jpipe1 | jpipe2 ; FULL = BASIC | jrainbow1 | jrainbow2 | jcorsair.
    const BASIC: u8 = 0x80 | 0x01 | 0x10 | 0x20;
    const FULL: u8 = BASIC | 0x02 | 0x04 | 0x08;
    if sync {
        m[Z_RAINBOW1] = 1;
        m[Z_RAINBOW1 + 10] = SYNC_RAINBOW as u8;
        m[Z_RAINBOW2] = 1;
        m[Z_RAINBOW2 + 10] = SYNC_RAINBOW as u8;
        m[Z_CORSAIR] = 1;
        m[Z_CORSAIR + 10] = SYNC_CORSAIR as u8;
        m[Z_ONBOARD + 8] = FULL;
    } else {
        if b.has_rainbow1 {
            m[Z_RAINBOW1] = DIRECT_EFFECT;
            m[Z_RAINBOW1 + 10] = R1_MAX as u8;
        }
        if b.has_rainbow2 {
            m[Z_RAINBOW2] = DIRECT_EFFECT;
            m[Z_RAINBOW2 + 10] = R2_MAX as u8;
        }
        if b.has_corsair {
            m[Z_CORSAIR + 10] = CORSAIR_MAX as u8;
        }
        m[Z_ONBOARD + 8] = BASIC;
    }
    m
}

fn packet(hdr1: u8, hdr2: u8) -> Vec<u8> {
    let mut p = vec![0u8; PER_LED_LEN];
    p[..5].copy_from_slice(&[0x53, DIRECT_EFFECT, hdr1, hdr2, 0]);
    p
}

fn put(p: &mut [u8], led: usize, c: Color) {
    if led < 240 {
        p[5 + led * 3..8 + led * 3].copy_from_slice(&[c.r, c.g, c.b]);
    }
}

/// Tampons de couleurs par zone, dans l'ordre d'exposition.
pub struct Parts<'a> {
    pub onboard: &'a [Color],
    pub pipe1: &'a [Color],
    pub pipe2: &'a [Color],
    pub jrgb: &'a [Color],
    pub rainbow1: &'a [Color],
    pub rainbow2: &'a [Color],
    pub corsair: &'a [Color],
}

/// Rapports 0x53 à envoyer, dans l'ordre. `sync` = disposition synchronisée.
pub fn frames(b: &Board, sync: bool, p: &Parts) -> Vec<Vec<u8>> {
    let mut base = packet(0x06, 0);
    let mut led = 0;
    for (n, cols) in [(b.onboard, p.onboard), (b.pipe1, p.pipe1), (b.pipe2, p.pipe2), (b.jrgbs, p.jrgb)] {
        for i in 0..n {
            put(&mut base, led + i, cols.get(i).copied().unwrap_or(Color::BLACK));
        }
        led += n;
    }
    if sync {
        for (start, cols, max) in [(led, p.rainbow1, SYNC_RAINBOW), (led + SYNC_RAINBOW, p.rainbow2, SYNC_RAINBOW), (led + 2 * SYNC_RAINBOW, p.corsair, SYNC_CORSAIR)] {
            for (i, c) in cols.iter().take(max).enumerate() {
                put(&mut base, start + i, *c);
            }
        }
        return vec![base];
    }
    let mut out = Vec::new();
    for (present, hdr1, hdr2, cols) in [(b.has_rainbow1, 4u8, 0u8, p.rainbow1), (b.has_rainbow2, 4, 1, p.rainbow2), (b.has_corsair, 5, 0, p.corsair)] {
        if present {
            let mut pk = packet(hdr1, hdr2);
            for (i, c) in cols.iter().enumerate() {
                put(&mut pk, i, *c);
            }
            out.push(pk);
        }
    }
    out.push(base);
    out
}

pub struct PerLed {
    hid: Box<dyn Hid>,
    board: Board,
    zones: Vec<ZoneInfo>,
    /// État d'origine lu à l'ouverture (`save_data` = 0), renvoyé par `release()`.
    original: [u8; LEN],
    sync: bool,
    last: Vec<Vec<u8>>,
}

/// Disposition synchronisée : JRAINBOW 1 et 2 <= 40 LEDs et JCORSAIR <= 120 (nombres réglés par l'utilisateur).
fn is_sync(zones: &[ZoneInfo]) -> bool {
    let n = |name: &str| zones.iter().find(|z| z.name == name).map_or(0, |z| z.led_count as usize);
    n(RAINBOW1) <= SYNC_RAINBOW && n(RAINBOW2) <= SYNC_RAINBOW && n(CORSAIR) <= SYNC_CORSAIR
}

/// Ouvre le mode LED par LED : lit l'état d'origine, envoie le rapport d'activation. Échec = erreur (l'appelant peut replier sur le mode zone).
pub fn open(hid: Box<dyn Hid>, board: Board, zones: Vec<ZoneInfo>, original: [u8; LEN]) -> Result<Box<dyn NativeRgb>> {
    let sync = is_sync(&zones);
    let msg = enable_message(&board, sync);
    if let Err(e) = hid.send_feature_report(&msg) {
        bail!("MSI Mystic : activation du mode LED par LED refusée : {e:#}");
    }
    Ok(Box::new(PerLed { hid, board, zones, original, sync, last: Vec::new() }))
}

impl NativeRgb for PerLed {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        let parts = super::split_by_zone(colors, &self.zones);
        let get = |name: &str| self.zones.iter().position(|z| z.name == name).map_or(&[][..], |i| &parts[i][..]);
        let p = Parts {
            onboard: get(ONBOARD),
            pipe1: get(PIPE1),
            pipe2: get(PIPE2),
            jrgb: get(JRGB),
            rainbow1: get(RAINBOW1),
            rainbow2: get(RAINBOW2),
            corsair: get(CORSAIR),
        };
        let next = frames(&self.board, self.sync, &p);
        if next == self.last {
            return Ok(());
        }
        let n = next.len();
        for (i, f) in next.iter().enumerate() {
            self.hid.send_feature_report(f)?;
            if !self.sync && i + 1 < n {
                std::thread::sleep(std::time::Duration::from_millis(13));
            }
        }
        self.last = next;
        Ok(())
    }

    fn release(&mut self) {
        // Rend la main à l'effet d'origine de la carte (jamais d'enregistrement : save_data = 0).
        let mut s = self.original;
        s[184] = 0;
        if let Err(e) = self.hid.send_feature_report(&s) {
            log::warn!("MSI Mystic : retour à l'état d'origine impossible : {e:#}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::hid::native::MockHid;

    fn z793() -> Board {
        board(0x7D93).expect("carte MS-7D93 dans la table")
    }

    #[test]
    fn the_z790_gaming_pro_wifi_is_in_the_table_with_its_layout() {
        let b = z793();
        assert_eq!((b.onboard, b.pipe1, b.pipe2, b.jrgbs), (6, 0, 0, 1));
        assert!(b.has_jrgb && b.has_onboard && b.has_rainbow1 && b.has_rainbow2 && !b.has_corsair && !b.has_pipe1);
        let names: Vec<String> = zones(&b).into_iter().map(|z| z.name).collect();
        assert_eq!(names, vec!["LED carte", "JRGB", "JRAINBOW 1", "JRAINBOW 2"]);
        let total: u32 = zones(&b).iter().map(|z| z.led_count).sum();
        assert_eq!(total, 6 + 1 + 60 + 60, "127 LEDs par défaut : plus que les 77 d'OpenRGB, le pilote intégré devient le principal");
        assert!(board(0x7C76).is_none(), "PID hors table : pas de mode LED par LED");
    }

    #[test]
    fn enable_message_matches_the_documented_bytes() {
        let b = z793();
        let m = enable_message(&b, true);
        assert_eq!(m.len(), 185);
        assert_eq!(m[0], 0x52);
        assert_eq!((m[74], m[74 + 4], m[74 + 8]), (0x25, 0xA9, 0xBF), "embarqué : mode direct, synchro complète");
        assert_eq!((m[31], m[31 + 10], m[42], m[42 + 10]), (1, 40, 1, 40), "JRAINBOW en synchro : statique, 40 LEDs");
        assert_eq!((m[53 + 4], m[53 + 8], m[53 + 10]), (0x29, 0x82, 120));
        assert_eq!(m[184], 0, "save_data toujours 0");
        let n = enable_message(&b, false);
        assert_eq!((n[31], n[31 + 10], n[42], n[42 + 10]), (0x25, 200, 0x25, 240));
        assert_eq!(n[74 + 8], 0xB1, "embarqué : synchro de base");
    }

    #[test]
    fn sync_layout_is_one_725_byte_report_with_onboard_jrgb_then_argb_areas() {
        let b = z793();
        let col = |v: u8| Color { r: v, g: v + 1, b: v + 2 };
        let on: Vec<Color> = (0..6u8).map(|i| col(i * 10)).collect();
        let r1: Vec<Color> = (0..3u8).map(|i| col(100 + i)).collect();
        let f = frames(&b, true, &Parts { onboard: &on, pipe1: &[], pipe2: &[], jrgb: &[col(7)], rainbow1: &r1, rainbow2: &[col(200)], corsair: &[] });
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].len(), 725);
        assert_eq!(&f[0][..5], &[0x53, 0x25, 0x06, 0, 0]);
        assert_eq!(&f[0][5..8], &[0, 1, 2], "1re LED embarquée");
        assert_eq!(&f[0][5 + 5 * 3..8 + 5 * 3], &[50, 51, 52], "6e LED embarquée");
        assert_eq!(&f[0][5 + 6 * 3..8 + 6 * 3], &[7, 8, 9], "JRGB juste après les 6 LEDs embarquées");
        assert_eq!(&f[0][5 + 7 * 3..8 + 7 * 3], &[100, 101, 102], "JRAINBOW 1 commence à l'index 7");
        assert_eq!(&f[0][5 + 47 * 3..8 + 47 * 3], &[200, 201, 202], "JRAINBOW 2 commence 40 LEDs plus loin");
    }

    #[test]
    fn long_strips_use_one_report_per_connector_then_the_onboard_report() {
        let b = z793();
        let r1 = vec![Color { r: 9, g: 9, b: 9 }; 60];
        let f = frames(&b, false, &Parts { onboard: &[], pipe1: &[], pipe2: &[], jrgb: &[], rainbow1: &r1, rainbow2: &[Color { r: 1, g: 2, b: 3 }], corsair: &[] });
        assert_eq!(f.len(), 3, "JRAINBOW 1, JRAINBOW 2, puis embarqué");
        assert_eq!(&f[0][..5], &[0x53, 0x25, 0x04, 0, 0]);
        assert_eq!(&f[0][5 + 59 * 3..8 + 59 * 3], &[9, 9, 9], "60 LEDs sur JRAINBOW 1");
        assert_eq!(&f[1][..5], &[0x53, 0x25, 0x04, 0x01, 0]);
        assert_eq!(&f[1][5..8], &[1, 2, 3]);
        assert_eq!(&f[2][..5], &[0x53, 0x25, 0x06, 0, 0]);
    }

    #[test]
    fn open_sends_the_enable_message_and_release_restores_the_original_state() {
        let b = z793();
        let (mock, _, log) = MockHid::new(vec![]);
        let mut original = [0xAAu8; 185];
        original[0] = 0x52;
        original[184] = 1;
        let mut dev = open(Box::new(mock), b.clone(), zones(&b), original).expect("ouverture");
        assert_eq!(log.lock().len(), 1, "un seul rapport : l'activation");
        dev.release();
        let sent = log.lock().clone();
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[1].len(), 185);
        assert_eq!(sent[1][184], 0, "jamais d'enregistrement");
        assert_eq!(sent[1][1], 0xAA, "état d'origine rendu");
    }

    #[test]
    fn unchanged_frames_are_not_resent_and_a_failed_enable_is_an_error() {
        let b = z793();
        let (mock, _, log) = MockHid::new(vec![]);
        let mut dev = open(Box::new(mock), b.clone(), zones(&b), [0u8; 185]).expect("ouverture");
        let colors = vec![Color { r: 1, g: 2, b: 3 }; (b.onboard + b.jrgbs + 2) as usize];
        dev.set_colors(&colors).expect("ok");
        let n = log.lock().len();
        dev.set_colors(&colors).expect("identique");
        assert_eq!(log.lock().len(), n, "image identique : rien renvoyé");
    }
}
