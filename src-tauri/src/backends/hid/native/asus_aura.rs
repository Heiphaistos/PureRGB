//! Famille « asus_aura » : cartes mères ASUS Aura USB (HID 65 octets, report ID 0xEC), mode direct uniquement.
//! Source : docs/rgb-protocols/asus-aura.md
//!
//! Périmètre : PID carte mère documentés. Contrôleurs « Addressable » (1867/1872/18A3/18A5) et pompe LC III (1B29) non
//! revendiqués : le numéro de canal direct y est INCONNU dans la note.
//! Le nombre de LEDs réel vient de la table de configuration lue à l'ouverture (`EC B0`) ; le `Model` (figé au scan) déclare une
//! zone « Carte mère » redimensionnable et 3 connecteurs ARGB : on tronque aux tailles réelles, et la zone carte est complétée
//! avec sa dernière couleur jusqu'au nombre de LEDs de la table. Jamais de commit flash (`EC 3F 55`), jamais d'effet matériel.
//! `release()` : aucun retour au mode matériel documenté (INCONNU), donc rien n'est envoyé.

use super::{split_by_zone, Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};

const REPORT_ID: u8 = 0xEC;
const PKT: usize = 65;
const LEDS_PER_PACKET: usize = 20;
const BOARD_DIRECT_CHANNEL: u8 = 0x04;
const MODE_DIRECT: u8 = 0xFF;
const ARGB_MAX_LEDS: u32 = 500;
/// Connecteurs ARGB déclarés au scan (la table réelle peut en annoncer moins : les zones en trop sont ignorées).
const ARGB_ZONES: usize = 3;
const REPLY_TIMEOUT_MS: i32 = 1000;

const PIDS: [u16; 5] = [0x18F3, 0x1939, 0x19AF, 0x1AA6, 0x1BED];

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != 0x0B05 || !PIDS.contains(&pid) {
        return None;
    }
    let mut zones = vec![ZoneInfo { name: "Carte mère".into(), led_count: 1, zone_type: 1, leds_min: 1, leds_max: 255 }];
    for i in 1..=ARGB_ZONES {
        zones.push(ZoneInfo { name: format!("ARGB {i}"), led_count: 1, zone_type: 1, leds_min: 1, leds_max: ARGB_MAX_LEDS });
    }
    let mut m = Model::new("ASUS Aura (carte mère)", DeviceType::Motherboard, zones);
    if pid == 0x19AF {
        m.usage_page = Some(0xFF72); // usage 0x00A1 non exprimable dans Model
    }
    Some(m)
}

fn cmd(bytes: &[u8]) -> [u8; PKT] {
    let mut p = [0u8; PKT];
    p[0] = REPORT_ID;
    p[1..1 + bytes.len()].copy_from_slice(bytes);
    p
}

struct Aura {
    hid: Box<dyn Hid>,
    /// LEDs fixes de la carte (table de config, octet 0x1B).
    board_leds: usize,
    /// Connecteurs ARGB réellement présents (table, octet 0x02), plafonnés à `ARGB_ZONES`.
    argb: usize,
    zones: Vec<ZoneInfo>,
}

impl Aura {
    fn request(&self, c: [u8; PKT], expect: u8) -> Result<[u8; PKT]> {
        self.hid.write(&c)?;
        let mut buf = [0u8; PKT];
        let n = self.hid.read_timeout(&mut buf, REPLY_TIMEOUT_MS)?;
        if n < 2 || buf[1] != expect {
            bail!("ASUS Aura : réponse invalide à la commande {:#04x} (attendu {expect:#04x})", c[1]);
        }
        Ok(buf)
    }

    /// Un canal direct : paquets de 20 LEDs, bit 0x80 sur le dernier, bit 0x10 si l'offset dépasse 255.
    fn send_channel(&self, channel: u8, colors: &[Color]) -> Result<()> {
        let chunks: Vec<&[Color]> = colors.chunks(LEDS_PER_PACKET).collect();
        for (i, chunk) in chunks.iter().enumerate() {
            let offset = i * LEDS_PER_PACKET;
            let mut c = channel;
            if i + 1 == chunks.len() {
                c |= 0x80;
            }
            if offset > 255 {
                c |= 0x10;
            }
            let mut p = cmd(&[0x40, c, (offset % 256) as u8, chunk.len() as u8]);
            for (k, col) in chunk.iter().enumerate() {
                p[5 + 3 * k..8 + 3 * k].copy_from_slice(&[col.r, col.g, col.b]);
            }
            self.hid.write(&p)?;
        }
        Ok(())
    }
}

impl NativeRgb for Aura {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        let parts = split_by_zone(colors, &self.zones);
        let mut board = parts[0].clone();
        let fill = board.last().copied().unwrap_or(Color::BLACK);
        board.resize(self.board_leds, fill);
        self.send_channel(BOARD_DIRECT_CHANNEL, &board)?;
        for (i, part) in parts.iter().skip(1).take(self.argb).enumerate() {
            self.send_channel(i as u8, part)?;
        }
        Ok(())
    }
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    if model.zones.len() != 1 + ARGB_ZONES {
        bail!("ASUS Aura : disposition de modèle inattendue");
    }
    let mut a = Aura { hid, board_leds: 0, argb: 0, zones: model.zones.clone() };
    a.request(cmd(&[0x82]), 0x02)?; // firmware (chaîne ASCII en [2..17], non utilisée)
    let table = a.request(cmd(&[0xB0]), 0x30)?;
    // Table de 60 octets dans [4..63] ; octets utilisés : 0x02 (ARGB), 0x1B (LEDs carte).
    a.argb = (table[4 + 0x02] as usize).min(ARGB_ZONES);
    a.board_leds = table[4 + 0x1B] as usize;
    a.hid.write(&cmd(&[0x52, 0x53, 0x00, 0x01]))?; // « Gen1 » : sémantique INCONNUE, envoyé tel que documenté
    // Sélection du mode direct (0xFF) : canal d'effet 0 = carte, i+1 = ARGB i ; shutdown = 0.
    for ch in 0..=a.argb as u8 {
        a.hid.write(&cmd(&[0x35, ch, 0x00, 0x00, MODE_DIRECT]))?;
    }
    Ok(Box::new(a))
}

pub const FAMILY: Family = Family { id: "asus_aura", doc: "docs/rgb-protocols/asus-aura.md", probe, open };

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::hid::native::MockHid;

    fn reply_fw() -> Vec<u8> {
        let mut r = vec![0u8; PKT];
        r[0] = 0xEC;
        r[1] = 0x02;
        r
    }
    fn reply_table(argb: u8, leds: u8) -> Vec<u8> {
        let mut r = vec![0u8; PKT];
        r[0] = 0xEC;
        r[1] = 0x30;
        r[4 + 0x02] = argb;
        r[4 + 0x1B] = leds;
        r
    }
    fn padded(b: &[u8]) -> Vec<u8> {
        let mut v = b.to_vec();
        v.resize(PKT, 0);
        v
    }

    #[test]
    fn probe_claims_only_documented_pids() {
        assert!(probe(0x0B05, 0x18F3).is_some());
        assert!(probe(0x0B05, 0x1BED).is_some());
        assert_eq!(probe(0x0B05, 0x19AF).and_then(|m| m.usage_page), Some(0xFF72));
        assert!(probe(0x0B05, 0x1867).is_none());
        assert!(probe(0x0B05, 0x1B29).is_none());
        assert!(probe(0x1234, 0x18F3).is_none());
    }

    #[test]
    fn init_sequence_bytes() {
        let (mock, sent, _) = MockHid::new(vec![reply_fw(), reply_table(2, 8)]);
        let m = probe(0x0B05, 0x18F3).unwrap();
        assert!(open(Box::new(mock), &m).is_ok());
        let s = sent.lock();
        assert_eq!(s[0], padded(&[0xEC, 0x82]));
        assert_eq!(s[1], padded(&[0xEC, 0xB0]));
        assert_eq!(s[2], padded(&[0xEC, 0x52, 0x53, 0x00, 0x01]));
        assert_eq!(s[3], padded(&[0xEC, 0x35, 0x00, 0x00, 0x00, 0xFF]));
        assert_eq!(s[4], padded(&[0xEC, 0x35, 0x01, 0x00, 0x00, 0xFF]));
        assert_eq!(s[5], padded(&[0xEC, 0x35, 0x02, 0x00, 0x00, 0xFF]));
        assert_eq!(s.len(), 6);
    }

    #[test]
    fn rejects_bad_replies() {
        let m = probe(0x0B05, 0x18F3).unwrap();
        let (mock, _, _) = MockHid::new(vec![]);
        assert!(open(Box::new(mock), &m).is_err());
        let mut bad = reply_table(1, 1);
        bad[1] = 0x31;
        let (mock, _, _) = MockHid::new(vec![reply_fw(), bad]);
        assert!(open(Box::new(mock), &m).is_err());
    }

    #[test]
    fn direct_board_padded_and_argb_split_in_packets() {
        // Carte : 2 LEDs en table, zone modèle de 1 LED -> complétée avec la dernière couleur.
        let (mock, sent, _) = MockHid::new(vec![reply_fw(), reply_table(1, 2)]);
        let mut m = probe(0x0B05, 0x18F3).unwrap();
        m.zones[1].led_count = 25; // ARGB 1 : 25 LEDs -> 20 + 5
        let mut d = open(Box::new(mock), &m).unwrap();
        sent.lock().clear();
        let mut colors = vec![Color::new(1, 2, 3)];
        colors.extend((0..25).map(|i| Color::new(i, 0, 0)));
        colors.extend([Color::new(9, 9, 9); 2]); // zones ARGB 2 et 3 ignorées : table = 1 connecteur
        d.set_colors(&colors).unwrap();
        let s = sent.lock();
        assert_eq!(s.len(), 3);
        assert_eq!(s[0], padded(&[0xEC, 0x40, 0x84, 0x00, 0x02, 1, 2, 3, 1, 2, 3]));
        let mut p1 = vec![0xEC, 0x40, 0x00, 0x00, 20];
        for i in 0..20u8 {
            p1.extend_from_slice(&[i, 0, 0]);
        }
        assert_eq!(s[1], padded(&p1));
        let mut p2 = vec![0xEC, 0x40, 0x80, 20, 5];
        for i in 20..25u8 {
            p2.extend_from_slice(&[i, 0, 0]);
        }
        assert_eq!(s[2], padded(&p2));
    }

    #[test]
    fn offset_overflow_sets_bit_0x10() {
        let (mock, sent, _) = MockHid::new(vec![reply_fw(), reply_table(1, 1)]);
        let mut m = probe(0x0B05, 0x18F3).unwrap();
        m.zones[1].led_count = 300;
        let mut d = open(Box::new(mock), &m).unwrap();
        sent.lock().clear();
        let mut colors = vec![Color::BLACK];
        colors.extend(vec![Color::new(5, 6, 7); 300]);
        d.set_colors(&colors).unwrap();
        let s = sent.lock();
        // carte (1 paquet) + 15 paquets ARGB (index 1..=15)
        assert_eq!(s.len(), 16);
        assert_eq!(s[13][..5], [0xEC, 0x40, 0x00, 240, 20]); // offset 240 : pas de débordement
        assert_eq!(s[14][..5], [0xEC, 0x40, 0x10, 4, 20]); // offset 260 : 260-256 = 4
        assert_eq!(s[15][..5], [0xEC, 0x40, 0x90, 24, 20]); // offset 280, dernier paquet
    }
}
