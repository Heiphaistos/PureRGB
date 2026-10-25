//! Famille « lianli_uni » : hubs ENE 6K77 Uni Fan (VID 0x0CF2) : SL (7750, A100), AL (A101), SL-Infinity (A102), SL V2 (A103),
//! AL V2 (A104), SL V2 « V2A » (A105), SL Redragon (A106). Rapport 0xE0.
//! Sources : docs/rgb-protocols/analyse-lianli-lconnect.md (§3, faits confirmés dans L-Connect 3 v2.1.29 / v1.7.4) et docs/rgb-protocols/lianli.md.
//!
//! - RGB : couleurs en output report `E0 <0x30|port>` + triplets R,B,G ; effet statique en feature `E0 <0x10|port> 01 00 00 00`
//!   (mode 01 = statique sur tous ces modèles, vitesse normale, direction G->D, luminosité 100 %) ; commit `E0 60 00 <1|4>` (4 pour SL V2/V2A).
//!   Chaque rapport est suivi d'un délai de 20 ms (`writeDelayTime` officiel). Le rapport couleur est complété de noir jusqu'à la taille
//!   maximale du port (SL 64, SL V2 96 ; AL / SL-Inf : intérieur 32, extérieur 48 ; AL V2 : 48 / 72), comme le logiciel officiel.
//! - LED : 16 par ventilateur (SL, SL V2, Redragon : port = groupe) ; 8 intérieures + 12 extérieures (AL, SL-Inf, AL V2 : port 2g intérieur,
//!   2g+1 extérieur). 4 groupes, 4 ventilos max (6 pour SL V2 / AL V2). Le nombre de ventilos n'est pas détectable : les zones sont
//!   redimensionnables (3 ventilos par défaut) et la déclaration `E0 10 <sub> ...` au hub n'est pas envoyée (réglage absent du modèle).
//! - Ventilateurs : `set_fan_duty(groupe 0..3, %)`. Consigne = pourcentage 0..100 (feature `E0 <0x20|groupe> 00 <%>`, fait officiel), MINIMUM 10 % :
//!   une valeur plus basse est RAMENÉE à 10 (jamais d'arrêt : la valeur 1 « arrêt » du logiciel officiel n'est pas utilisée, plus sûr pour le refroidissement).
//!   Avant la consigne, le groupe passe en vitesse manuelle (`E0 10 <sub> <0x10<<g>`) ; `release()` rend la main à la synchro PWM carte mère
//!   (`0x11<<g`) si une consigne a été envoyée. Le logiciel officiel renvoie la consigne chaque seconde (sinon elle ne persiste pas) : à faire par l'appelant.
//! - Non revendiqués : Strimer (A200, octets de mode INCONNUS), TL / Trinity (échelles vitesse/luminosité et LED par ventilateur INCONNUES), sans-fil.
//! - INCONNU : longueur réelle des rapports HID (descripteur), usage page des modèles autres que SL (lianli.md donne 0xFF72 pour tous,
//!   le logiciel officiel ne filtre pas).

use super::{split_by_zone, Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};
use std::time::Duration;

const VID: u16 = 0x0CF2;
const REPORT_ID: u8 = 0xE0;
const GROUPS: usize = 4;
const DEFAULT_FANS: u32 = 3;
const CMD_DELAY: Duration = Duration::from_millis(20);
const MODE_STATIC: u8 = 0x01;
const MIN_DUTY: u8 = 10;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Sl,
    SlV2,
    Al,
    SlInf,
    AlV2,
}

impl Kind {
    /// Anneau double : un port intérieur (8 LED/ventilo) + un port extérieur (12 LED/ventilo) par groupe.
    fn dual(self) -> bool {
        matches!(self, Kind::Al | Kind::SlInf | Kind::AlV2)
    }
    fn max_fans(self) -> u32 {
        if matches!(self, Kind::SlV2 | Kind::AlV2) {
            6
        } else {
            4
        }
    }
    fn commit(self) -> u8 {
        if self == Kind::SlV2 {
            4
        } else {
            1
        }
    }
    /// Sous-commande de la synchro vitesse carte mère.
    fn sync_sub(self) -> u8 {
        match self {
            Kind::Sl => 0x31,
            Kind::Al => 0x42,
            _ => 0x62,
        }
    }
    /// LED par ventilateur de chaque port d'un groupe.
    fn leds_per_fan(self) -> &'static [u32] {
        if self.dual() {
            &[8, 12]
        } else {
            &[16]
        }
    }
}

const VARIANTS: [(u16, &str, Kind); 8] = [
    (0x7750, "Lian Li Uni Hub SL", Kind::Sl),
    (0xA100, "Lian Li Uni Hub SL", Kind::Sl),
    (0xA101, "Lian Li Uni Hub AL", Kind::Al),
    (0xA102, "Lian Li Uni Hub SL Infinity", Kind::SlInf),
    (0xA103, "Lian Li Uni Hub SL V2", Kind::SlV2),
    (0xA104, "Lian Li Uni Hub AL V2", Kind::AlV2),
    (0xA105, "Lian Li Uni Hub SL V2 (V2A)", Kind::SlV2),
    (0xA106, "Lian Li Uni Hub SL Redragon", Kind::Sl),
];

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != VID {
        return None;
    }
    let (_, name, kind) = VARIANTS.iter().find(|v| v.0 == pid)?;
    let mut zones = Vec::new();
    for g in 1..=GROUPS {
        for (i, per_fan) in kind.leds_per_fan().iter().enumerate() {
            let label = match (kind.dual(), i) {
                (false, _) => format!("Groupe {g}"),
                (true, 0) => format!("Groupe {g} intérieur"),
                (true, _) => format!("Groupe {g} extérieur"),
            };
            zones.push(ZoneInfo { name: label, led_count: DEFAULT_FANS * per_fan, zone_type: 1, leds_min: *per_fan, leds_max: kind.max_fans() * per_fan });
        }
    }
    let mut m = Model::new(*name, DeviceType::Fan, zones);
    m.fans = GROUPS as u8;
    m.usage_page = Some(0xFF72);
    Some(m)
}

struct UniHub {
    hid: Box<dyn Hid>,
    zones: Vec<ZoneInfo>,
    kind: Kind,
    manual: bool,
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    // Le PID n'est pas dans `Model` : le type se retrouve par le nom (même table que `probe`).
    let Some((_, _, kind)) = VARIANTS.iter().find(|v| v.1 == model.name) else {
        bail!("Uni Hub : modèle « {} » inconnu du driver", model.name);
    };
    let expected = GROUPS * kind.leds_per_fan().len();
    if model.zones.len() != expected {
        bail!("Uni Hub : {} zones attendues, {} reçues", expected, model.zones.len());
    }
    Ok(Box::new(UniHub { hid, zones: model.zones.clone(), kind: *kind, manual: false }))
}

impl UniHub {
    fn feature(&self, data: &[u8]) -> Result<()> {
        self.hid.send_feature_report(data)?;
        std::thread::sleep(CMD_DELAY);
        Ok(())
    }

    /// LED par ventilateur du port `port` (indice de zone).
    fn per_fan(&self, port: usize) -> u32 {
        let per = self.kind.leds_per_fan();
        per[port % per.len()]
    }
}

impl NativeRgb for UniHub {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        let total: u32 = self.zones.iter().map(|z| z.led_count).sum();
        if colors.len() > total as usize {
            bail!("Uni Hub : {} couleurs reçues pour {} LED", colors.len(), total);
        }
        for (port, z) in self.zones.iter().enumerate() {
            let per_fan = self.per_fan(port);
            if z.led_count == 0 || z.led_count % per_fan != 0 || z.led_count > self.kind.max_fans() * per_fan {
                bail!("Uni Hub : zone « {} » de {} LED invalide ({} LED par ventilateur, {} ventilateurs max)", z.name, z.led_count, per_fan, self.kind.max_fans());
            }
        }
        for (port, leds) in split_by_zone(colors, &self.zones).iter().enumerate() {
            let size = (self.kind.max_fans() * self.per_fan(port)) as usize;
            let mut out = vec![REPORT_ID, 0x30 | port as u8];
            for c in leds {
                out.extend_from_slice(&[c.r, c.b, c.g]); // ordre R,B,G
            }
            out.resize(2 + 3 * size, 0); // ventilateurs absents : noir
            self.hid.write(&out)?;
            std::thread::sleep(CMD_DELAY);
            self.feature(&[REPORT_ID, 0x10 | port as u8, MODE_STATIC, 0x00, 0x00, 0x00])?;
        }
        self.feature(&[REPORT_ID, 0x60, 0x00, self.kind.commit()])
    }

    fn set_fan_duty(&mut self, channel: u8, percent: u8) -> Result<()> {
        if channel as usize >= GROUPS {
            bail!("Uni Hub : groupe {channel} invalide (0..{})", GROUPS - 1);
        }
        if percent > 100 {
            bail!("Uni Hub : vitesse {percent} % invalide (0..100)");
        }
        let duty = percent.max(MIN_DUTY); // minimum de fonctionnement officiel : 10 %
        self.manual = true;
        self.feature(&[REPORT_ID, 0x10, self.kind.sync_sub(), 0x10 << channel])?;
        self.feature(&[REPORT_ID, 0x20 | channel, 0x00, duty])
    }

    fn release(&mut self) {
        self.release_fans();
    }

    fn fans_taken(&self) -> bool {
        self.manual
    }

    fn release_fans(&mut self) {
        if self.manual {
            for g in 0..GROUPS as u8 {
                // Meilleur effort : la fermeture ne doit jamais échouer.
                let _ = self.feature(&[REPORT_ID, 0x10, self.kind.sync_sub(), 0x11 << g]);
            }
            self.manual = false;
        }
    }
}

pub const FAMILY: Family = Family { id: "lianli_uni", doc: "docs/rgb-protocols/analyse-lianli-lconnect.md", probe, open };

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::hid::native::MockHid;
    use std::time::Instant;

    type Log = super::super::Log;

    fn dev(pid: u16) -> (Box<dyn NativeRgb>, Log, Log) {
        let (hid, sent, feats) = MockHid::new(vec![]);
        (open(Box::new(hid), &probe(VID, pid).unwrap()).unwrap(), sent, feats)
    }

    #[test]
    fn probe_claims_only_documented_pids() {
        for (pid, zones, leds) in [(0x7750, 4, 192), (0xA100, 4, 192), (0xA103, 4, 192), (0xA105, 4, 192), (0xA106, 4, 192), (0xA101, 8, 240), (0xA102, 8, 240), (0xA104, 8, 240)] {
            let m = probe(VID, pid).unwrap();
            assert_eq!((m.fans, m.zones.len(), m.led_count(), m.usage_page), (4, zones, leds, Some(0xFF72)), "{pid:04x}");
        }
        for pid in [0xA107, 0xA200, 0x7372] {
            assert!(probe(VID, pid).is_none(), "{pid:04x}");
        }
        assert!(probe(0x1234, 0xA100).is_none());
        assert!(probe(0x0416, 0x7372).is_none() && probe(0x0416, 0x7371).is_none() && probe(0x0416, 0x7373).is_none());
    }

    #[test]
    fn zones_are_resizable_per_fan() {
        let sl = probe(VID, 0xA100).unwrap();
        assert_eq!((sl.zones[0].leds_min, sl.zones[0].leds_max), (16, 64));
        let v2 = probe(VID, 0xA103).unwrap();
        assert_eq!(v2.zones[0].leds_max, 96);
        let al = probe(VID, 0xA101).unwrap();
        assert_eq!((al.zones[0].led_count, al.zones[0].leds_min, al.zones[0].leds_max), (24, 8, 32));
        assert_eq!((al.zones[1].led_count, al.zones[1].leds_min, al.zones[1].leds_max), (36, 12, 48));
        let alv2 = probe(VID, 0xA104).unwrap();
        assert_eq!((alv2.zones[0].leds_max, alv2.zones[1].leds_max), (48, 72));
    }

    #[test]
    fn sl_colors_are_rbg_padded_then_effect_then_commit() {
        let (mut d, sent, feats) = dev(0xA100);
        let mut cols = vec![Color::BLACK; 192];
        cols[0] = Color::new(1, 2, 3);
        cols[48] = Color::new(7, 8, 9); // groupe 2
        let t = Instant::now();
        d.set_colors(&cols).unwrap();
        assert!(t.elapsed() >= Duration::from_millis(9 * 20), "20 ms après chaque rapport (4 + 4 + commit)");
        let s = sent.lock();
        let f = feats.lock();
        assert_eq!(s.len(), 4);
        assert_eq!(s[0].len(), 2 + 64 * 3); // 194 o
        assert_eq!(&s[0][..5], &[0xE0, 0x30, 1, 3, 2]); // R,B,G
        assert_eq!(&s[1][..5], &[0xE0, 0x31, 7, 9, 8]);
        assert!(s[0][2 + 48 * 3..].iter().all(|b| *b == 0), "4e ventilateur absent = noir");
        assert_eq!(s[3][1], 0x33);
        assert_eq!(f.len(), 5);
        assert_eq!(f[0], [0xE0, 0x10, 0x01, 0, 0, 0]);
        assert_eq!(f[3], [0xE0, 0x13, 0x01, 0, 0, 0]);
        assert_eq!(f[4], [0xE0, 0x60, 0x00, 0x01]);
    }

    #[test]
    fn redragon_behaves_like_sl() {
        let (mut d, sent, feats) = dev(0xA106);
        d.set_colors(&[Color::new(4, 5, 6)]).unwrap();
        assert_eq!(&sent.lock()[0][..5], &[0xE0, 0x30, 4, 6, 5]);
        assert_eq!(feats.lock().last().unwrap(), &[0xE0, 0x60, 0x00, 0x01]);
    }

    #[test]
    fn v2_commit_is_4_and_report_is_290_bytes() {
        let (mut d, sent, feats) = dev(0xA103);
        d.set_colors(&[Color::new(1, 1, 1)]).unwrap();
        assert_eq!(feats.lock().last().unwrap(), &[0xE0, 0x60, 0x00, 0x04]);
        assert_eq!(sent.lock()[0].len(), 2 + 96 * 3);
    }

    #[test]
    fn al_uses_two_ports_per_group_with_8_and_12_leds() {
        let (mut d, sent, feats) = dev(0xA101);
        let mut cols = vec![Color::BLACK; 240];
        cols[0] = Color::new(1, 2, 3); // groupe 1 intérieur, 1re LED
        cols[24] = Color::new(4, 5, 6); // groupe 1 extérieur, 1re LED
        cols[60] = Color::new(7, 8, 9); // groupe 2 intérieur (24 + 36)
        d.set_colors(&cols).unwrap();
        let s = sent.lock();
        let f = feats.lock();
        assert_eq!(s.len(), 8);
        assert_eq!((s[0].len(), s[1].len()), (2 + 32 * 3, 2 + 48 * 3)); // 98 et 146 o
        assert_eq!(&s[0][..5], &[0xE0, 0x30, 1, 3, 2]);
        assert_eq!(&s[1][..5], &[0xE0, 0x31, 4, 6, 5]);
        assert_eq!(&s[2][..5], &[0xE0, 0x32, 7, 9, 8]);
        assert_eq!(s[7][1], 0x37);
        assert_eq!(f.len(), 9);
        assert_eq!(f[7], [0xE0, 0x17, 0x01, 0, 0, 0]);
        assert_eq!(f[8], [0xE0, 0x60, 0x00, 0x01]);
    }

    #[test]
    fn sl_infinity_and_al_v2_sizes() {
        let (mut d, sent, feats) = dev(0xA102);
        d.set_colors(&[]).unwrap();
        assert_eq!(sent.lock().len(), 8);
        assert_eq!(feats.lock().last().unwrap(), &[0xE0, 0x60, 0x00, 0x01]);
        let (mut d, sent, feats) = dev(0xA104);
        d.set_colors(&[]).unwrap();
        let s = sent.lock();
        assert_eq!((s[0].len(), s[1].len()), (2 + 48 * 3, 2 + 72 * 3));
        assert_eq!(feats.lock().last().unwrap(), &[0xE0, 0x60, 0x00, 0x01]);
    }

    #[test]
    fn too_many_colors_or_bad_zone_sends_nothing() {
        let (mut d, sent, feats) = dev(0xA100);
        assert!(d.set_colors(&vec![Color::BLACK; 193]).is_err());
        assert!(sent.lock().is_empty() && feats.lock().is_empty());
        let mut m = probe(VID, 0xA101).unwrap();
        m.zones[1].led_count = 13; // pas un multiple de 12
        let (hid, sent, feats) = MockHid::new(vec![]);
        let mut d = open(Box::new(hid), &m).unwrap();
        assert!(d.set_colors(&[Color::BLACK]).is_err());
        m.zones[1].led_count = 60; // 5 ventilos > 4
        let (hid2, sent2, feats2) = MockHid::new(vec![]);
        let mut d2 = open(Box::new(hid2), &m).unwrap();
        assert!(d2.set_colors(&[Color::BLACK]).is_err());
        assert!(sent.lock().is_empty() && feats.lock().is_empty() && sent2.lock().is_empty() && feats2.lock().is_empty());
    }

    #[test]
    fn fan_duty_sl_manual_mode_then_percent() {
        let (mut d, sent, feats) = dev(0xA100);
        d.set_fan_duty(1, 50).unwrap();
        assert!(sent.lock().is_empty());
        assert_eq!(feats.lock().as_slice(), [vec![0xE0, 0x10, 0x31, 0x20], vec![0xE0, 0x21, 0x00, 0x32]]);
    }

    #[test]
    fn fan_duty_is_clamped_to_10_percent_minimum() {
        let (mut d, _, feats) = dev(0xA101);
        d.set_fan_duty(0, 0).unwrap();
        d.set_fan_duty(3, 9).unwrap();
        d.set_fan_duty(2, 100).unwrap();
        let f = feats.lock();
        assert_eq!(f[0], [0xE0, 0x10, 0x42, 0x10]); // AL : sous-commande 0x42
        assert_eq!(f[1], [0xE0, 0x20, 0x00, 0x0A]);
        assert_eq!(f[2], [0xE0, 0x10, 0x42, 0x80]);
        assert_eq!(f[3], [0xE0, 0x23, 0x00, 0x0A]);
        assert_eq!(f[5], [0xE0, 0x22, 0x00, 0x64]);
    }

    #[test]
    fn fan_duty_sync_sub_for_infinity_and_v2() {
        for pid in [0xA102, 0xA103, 0xA104, 0xA105] {
            let (mut d, _, feats) = dev(pid);
            d.set_fan_duty(2, 75).unwrap();
            assert_eq!(feats.lock().as_slice(), [vec![0xE0, 0x10, 0x62, 0x40], vec![0xE0, 0x22, 0x00, 0x4B]], "{pid:04x}");
        }
    }

    #[test]
    fn fan_duty_rejects_invalid_input_without_sending() {
        let (mut d, sent, feats) = dev(0xA100);
        assert!(d.set_fan_duty(4, 50).is_err());
        assert!(d.set_fan_duty(0, 101).is_err());
        assert!(sent.lock().is_empty() && feats.lock().is_empty());
    }

    #[test]
    fn release_returns_fans_to_motherboard_sync_only_after_a_duty() {
        let (mut d, _, feats) = dev(0xA100);
        d.release();
        assert!(feats.lock().is_empty());
        d.set_fan_duty(0, 30).unwrap();
        feats.lock().clear();
        d.release();
        let f = feats.lock();
        assert_eq!(f.as_slice(), [vec![0xE0, 0x10, 0x31, 0x11], vec![0xE0, 0x10, 0x31, 0x22], vec![0xE0, 0x10, 0x31, 0x44], vec![0xE0, 0x10, 0x31, 0x88]]);
    }

    #[test]
    fn open_colors_and_release_without_a_user_duty_write_no_fan_command() {
        let (mut d, sent, feats) = dev(0xA100);
        assert!(!d.fans_taken());
        d.release_fans();
        d.release();
        assert!(sent.lock().is_empty() && feats.lock().is_empty());
        d.set_fan_duty(0, 30).unwrap();
        assert!(d.fans_taken());
        d.release_fans();
        assert!(!d.fans_taken(), "rendu une fois, plus rien à rendre");
        let n = feats.lock().len();
        d.release();
        assert_eq!(feats.lock().len(), n, "idempotent");
    }
}
