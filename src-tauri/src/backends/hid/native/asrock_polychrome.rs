//! Famille « asrock_polychrome » : ASRock Polychrome USB (26CE:01A2 carte mère, 26CE:01A6 ruban Deskmini), mode direct.
//! Source : docs/rgb-protocols/asrock-polychrome.md (variantes SMBus V1/V2/ASR RGB : hors périmètre, pas de HID).
//!
//! Transport : écriture de 65 octets (octet 0 = 0, pas de report ID) puis lecture OBLIGATOIRE de l'accusé de 64 octets.
//! Les tailles de zones, le masque de zones présentes et le masque RGSWAP sont lus à l'ouverture (commande 0x14).
//! Jamais écrit : tables 0x15 (taille de zone / RGSWAP) et commit 0x12 (persistance flash INCONNUE) ; `release()` ne fait rien
//! (aucun retour au mode matériel documenté). Le `Model` est figé au scan : chaque zone est complétée avec sa dernière couleur
//! (ou tronquée) à la taille réelle lue dans le contrôleur.
//! INCONNU / à valider sur matériel : indices des octets d'une réponse de lecture (pris tels que notés : `[4]`, `[4..11]`),
//! formule exacte du remplissage `(Σ leds_max + 8)×3` (ici : Σ des tailles fixes + 100 par zone adressable), cadence.

use super::{split_by_zone, Family, Hid, Model, NativeRgb};
use crate::core::{Color, DeviceType, ZoneInfo};
use anyhow::{bail, Result};

const PKT: usize = 65;
const ACK: usize = 64;
const ACK_TIMEOUT_MS: i32 = 1000;
const ZONES: usize = 8;
const ZONE_UNAVAILABLE: u8 = 0x1E;
const ADDRESSABLE_MAX: usize = 100;
const MODE_DIRECT: u8 = 0x0F;
const SPEED_DEFAULT: u8 = 0xE0;
const FIRST_LEDS: usize = 18;
const NEXT_LEDS: usize = 19;

const NAMES: [&str; ZONES] = ["RGB 1", "RGB 2", "ARGB 1", "ARGB 2", "PCH", "Cache E/S", "PCB", "Audio / ARGB 3"];
const ADDRESSABLE: [bool; ZONES] = [false, false, true, true, false, false, false, true];

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != 0x26CE || !matches!(pid, 0x01A2 | 0x01A6) {
        return None;
    }
    let zones = (0..ZONES)
        .map(|i| {
            if ADDRESSABLE[i] {
                ZoneInfo { name: NAMES[i].into(), led_count: 1, zone_type: 1, leds_min: 1, leds_max: ADDRESSABLE_MAX as u32 }
            } else {
                ZoneInfo::fixed(NAMES[i], 1)
            }
        })
        .collect();
    Some(Model::new("ASRock Polychrome", DeviceType::Motherboard, zones))
}

struct Polychrome {
    hid: Box<dyn Hid>,
    zones: Vec<ZoneInfo>,
    /// Taille réelle de chaque zone (0 = absente).
    sizes: [usize; ZONES],
    /// Bit n = zone n en R,G,B ; sinon G,R,B.
    rgswap: u8,
}

fn xfer(hid: &dyn Hid, buf: &[u8; PKT]) -> Result<[u8; ACK]> {
    hid.write(buf)?;
    let mut ack = [0u8; ACK];
    if hid.read_timeout(&mut ack, ACK_TIMEOUT_MS)? == 0 {
        bail!("ASRock Polychrome : accusé de réception absent (commande {:#04x})", buf[1]);
    }
    Ok(ack)
}

fn read_table(hid: &dyn Hid, table: u8) -> Result<[u8; ACK]> {
    let mut b = [0u8; PKT];
    b[1] = 0x14;
    b[3] = table;
    xfer(hid, &b)
}

impl Polychrome {
    fn stream(&self, colors: &[Color]) -> Vec<u8> {
        let parts = split_by_zone(colors, &self.zones);
        let mut s = Vec::new();
        let mut target = 8usize; // « + 8 » de la note
        for (i, part) in parts.iter().enumerate() {
            let n = self.sizes[i];
            if n == 0 {
                continue;
            }
            target += if ADDRESSABLE[i] { ADDRESSABLE_MAX } else { n };
            let fill = part.last().copied().unwrap_or(Color::BLACK);
            for k in 0..n {
                let c = part.get(k).copied().unwrap_or(fill);
                if self.rgswap >> i & 1 == 1 {
                    s.extend_from_slice(&[c.r, c.g, c.b]);
                } else {
                    s.extend_from_slice(&[c.g, c.r, c.b]);
                }
            }
        }
        if s.len() < target * 3 {
            s.resize(target * 3, 0);
        }
        s
    }
}

impl NativeRgb for Polychrome {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        let s = self.stream(colors);
        let (head, mut rest) = s.split_at((FIRST_LEDS * 3).min(s.len()));
        let mut b = [0u8; PKT];
        b[1] = 0x10;
        b[3] = 0xFF;
        b[4] = 0xE3;
        b[7] = 0xFF;
        b[64] = 0x65;
        b[9..9 + head.len()].copy_from_slice(head);
        xfer(self.hid.as_ref(), &b)?;
        while !rest.is_empty() {
            let (chunk, tail) = rest.split_at((NEXT_LEDS * 3).min(rest.len()));
            let mut b = [0u8; PKT];
            b[1] = 0x10;
            b[3] = 0xFF;
            b[4] = 0xE4;
            b[64] = 0x65;
            b[5..5 + chunk.len()].copy_from_slice(chunk);
            xfer(self.hid.as_ref(), &b)?;
            rest = tail;
        }
        Ok(())
    }
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    if model.zones.len() != ZONES {
        bail!("ASRock Polychrome : disposition de modèle inattendue");
    }
    let present = read_table(hid.as_ref(), 0x01)?[4];
    let raw = read_table(hid.as_ref(), 0x02)?;
    let rgswap = read_table(hid.as_ref(), 0x03)?[4];
    let mut sizes = [0usize; ZONES];
    for (i, size) in sizes.iter_mut().enumerate() {
        let v = raw[4 + i];
        if v == ZONE_UNAVAILABLE || present >> i & 1 == 0 {
            continue;
        }
        if v as usize > ADDRESSABLE_MAX {
            bail!("ASRock Polychrome : taille de zone {i} invalide ({v})");
        }
        *size = v as usize;
    }
    // Mode direct (0x0F) sur chaque zone présente ; couleur 0, vitesse par défaut, allzone = 0.
    for (i, _) in sizes.iter().enumerate().filter(|(_, n)| **n > 0) {
        let mut b = [0u8; PKT];
        b[1] = 0x10;
        b[3] = i as u8;
        b[4] = MODE_DIRECT;
        b[8] = SPEED_DEFAULT;
        b[9] = 0xFF;
        xfer(hid.as_ref(), &b)?;
    }
    Ok(Box::new(Polychrome { hid, zones: model.zones.clone(), sizes, rgswap }))
}

pub const FAMILY: Family = Family { id: "asrock_polychrome", doc: "docs/rgb-protocols/asrock-polychrome.md", probe, open };

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::hid::native::MockHid;

    fn ack() -> Vec<u8> {
        vec![0u8; ACK]
    }
    fn table(idx4: &[u8]) -> Vec<u8> {
        let mut r = vec![0u8; ACK];
        r[4..4 + idx4.len()].copy_from_slice(idx4);
        r
    }
    fn padded(b: &[u8]) -> Vec<u8> {
        let mut v = b.to_vec();
        v.resize(PKT, 0);
        v
    }

    /// Carte : zones présentes 0 (RGB1, 2 LEDs) et 2 (ARGB1, 20 LEDs) ; RGSWAP bit 0 seul.
    fn replies() -> Vec<Vec<u8>> {
        let mask = 0b0000_0101;
        let mut sizes = [ZONE_UNAVAILABLE; 8];
        sizes[0] = 2;
        sizes[2] = 20;
        vec![table(&[mask]), table(&sizes), table(&[0b0000_0001]), ack(), ack()]
    }

    #[test]
    fn probe_claims_only_documented_pids() {
        assert_eq!(probe(0x26CE, 0x01A2).map(|m| m.zones.len()), Some(8));
        assert!(probe(0x26CE, 0x01A6).is_some());
        assert!(probe(0x26CE, 0x0012).is_none());
        assert!(probe(0x26CE, 0x01C4).is_none());
        assert!(probe(0x0B05, 0x01A2).is_none());
    }

    #[test]
    fn open_reads_tables_and_sets_direct_mode() {
        let (mock, sent, _) = MockHid::new(replies());
        let m = probe(0x26CE, 0x01A2).unwrap();
        assert!(open(Box::new(mock), &m).is_ok());
        let s = sent.lock();
        assert_eq!(s[0], padded(&[0, 0x14, 0, 0x01]));
        assert_eq!(s[1], padded(&[0, 0x14, 0, 0x02]));
        assert_eq!(s[2], padded(&[0, 0x14, 0, 0x03]));
        assert_eq!(s[3], padded(&[0, 0x10, 0, 0, 0x0F, 0, 0, 0, 0xE0, 0xFF]));
        assert_eq!(s[4], padded(&[0, 0x10, 0, 2, 0x0F, 0, 0, 0, 0xE0, 0xFF]));
        assert_eq!(s.len(), 5);
    }

    #[test]
    fn missing_ack_and_bad_size_are_errors() {
        let m = probe(0x26CE, 0x01A2).unwrap();
        let (mock, _, _) = MockHid::new(vec![]);
        assert!(open(Box::new(mock), &m).is_err());
        let mut bad = [ZONE_UNAVAILABLE; 8];
        bad[0] = 101;
        let (mock, _, _) = MockHid::new(vec![table(&[1]), table(&bad), table(&[0])]);
        assert!(open(Box::new(mock), &m).is_err());
    }

    #[test]
    fn direct_stream_packets() {
        // Flux : 2 + 20 LEDs = 66 octets ; cible (2 + 100 + 8)*3 = 330 -> 54 + 57*5 = 339 >= 330 : 1 + 5 paquets.
        let mut r = replies();
        r.extend(vec![ack(); 6]);
        let (mock, sent, _) = MockHid::new(r);
        let m = probe(0x26CE, 0x01A2).unwrap();
        let mut d = open(Box::new(mock), &m).unwrap();
        sent.lock().clear();
        let mut colors = vec![Color::BLACK; 8];
        colors[0] = Color::new(1, 2, 3); // zone 0 (RGSWAP : R,G,B), complétée à 2 LEDs
        colors[2] = Color::new(10, 20, 30); // zone 2 (G,R,B), complétée à 20 LEDs
        d.set_colors(&colors).unwrap();
        let s = sent.lock();
        assert_eq!(s.len(), 6);
        let mut p0 = vec![0, 0x10, 0, 0xFF, 0xE3, 0, 0, 0xFF, 0, 1, 2, 3, 1, 2, 3];
        for _ in 0..16 {
            p0.extend_from_slice(&[20, 10, 30]); // LEDs 3..18 du flux = début de la zone 2
        }
        let mut exp0 = padded(&p0);
        exp0[64] = 0x65;
        assert_eq!(s[0], exp0);
        // 2e paquet : LEDs 18.. du flux : 4 LEDs restantes de la zone 2 (octets 54..66), puis zéros.
        let mut p1 = vec![0, 0x10, 0, 0xFF, 0xE4];
        p1.extend_from_slice(&[20, 10, 30].repeat(4));
        let mut exp1 = padded(&p1);
        exp1[64] = 0x65;
        assert_eq!(s[1], exp1);
        for k in 2..6 {
            assert_eq!(s[k][..5], [0, 0x10, 0, 0xFF, 0xE4]);
            assert_eq!(s[k][64], 0x65);
        }
        assert!(s[2][5..64].iter().all(|b| *b == 0));
    }
}
