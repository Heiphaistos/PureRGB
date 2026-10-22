//! Famille « logitech » (VID 0x046D) : claviers HID++ 2.0. Cadre commun (trames, IRoot, table des PID) ; un fichier par protocole.
//! Sources : docs/rgb-protocols/logitech.md et docs/rgb-protocols/analyse-logitech-ghub.md (§3 claviers, §3.4 prise/restitution du contrôle, §7 sécurité).
//!
//! - `per_key` : feature 0x8080 (G410 / G512 / G513 / G610 / G810 / G910), rapports 0x12.
//! - `keyboard` : G815 (rapports fixes) et G915 / G915 TKL filaires (0x8071 + 0x8081), rapports 0x11.
//!
//! NON revendiqué (INCONNU ou hors périmètre) :
//! - souris (G203 Lightsync, 0x8070/0x8071) : la note ne donne ni le nombre de LEDs/zones par modèle ni le format exact des réponses d'énumération ;
//! - G813, G213, G413, G515, G Pro (C339) : collection ou trames non établies ; récepteurs Lightspeed/Unifying, Powerplay, casques : hors périmètre ;
//! - format exact d'une réponse d'erreur HID++ 2.0 (seul « feature_index 0xFF » est indiqué, « non revérifié »).
//! Jamais envoyé : persist = 1 (SET_EFFECT), NV_CONFIG, BIN_INFO, SYNC, PWR_CONFIG, SHUTDOWN, écriture de profil.

mod keyboard;
mod per_key;

use super::{Family, Hid, Model, NativeRgb};
use crate::core::{DeviceType, ZoneInfo};
use anyhow::{bail, Result};

const VID: u16 = 0x046D;
/// Index de périphérique d'un appareil filaire.
const DEVICE: u8 = 0xFF;
/// Identifiant d'émetteur (4 bits) des requêtes IRoot ; le firmware le renvoie tel quel.
const SW_ID: u8 = 0x07;
const REPORT_LONG: u8 = 0x11;
const REPORT_VLONG: u8 = 0x12;
const REPLY_TRIES: usize = 8;
const REPLY_TIMEOUT_MS: i32 = 500;
const FEATURE_ERROR: u8 = 0xFF;

#[derive(Clone, Copy)]
enum Kind {
    PerKey,
    G815,
    G915 { tkl: bool },
}

/// (PID, nom, protocole). PID : note logitech.md / analyse-logitech-ghub.md §3.1 et signalrgb-devices.csv.
const MODELS: &[(u16, &str, Kind)] = &[
    (0xC330, "Logitech G410 Atlas Spectrum", Kind::PerKey),
    (0xC331, "Logitech G810", Kind::PerKey),
    (0xC337, "Logitech G810", Kind::PerKey),
    (0xC32B, "Logitech G910", Kind::PerKey),
    (0xC335, "Logitech G910", Kind::PerKey),
    (0xC342, "Logitech G512", Kind::PerKey),
    (0xC33C, "Logitech G513", Kind::PerKey),
    (0xC333, "Logitech G610", Kind::PerKey),
    (0xC338, "Logitech G610", Kind::PerKey),
    (0xC33F, "Logitech G815", Kind::G815),
    (0xC33E, "Logitech G915", Kind::G915 { tkl: false }),
    (0xC343, "Logitech G915 TKL", Kind::G915 { tkl: true }),
];

fn probe(vid: u16, pid: u16) -> Option<Model> {
    if vid != VID {
        return None;
    }
    let (_, name, kind) = MODELS.iter().find(|(p, ..)| *p == pid)?;
    let (leds, zone) = match kind {
        Kind::PerKey => (per_key::LEDS, "Touches (usages HID)"),
        Kind::G815 => (keyboard::codes(false).len(), "Touches"),
        Kind::G915 { tkl } => (keyboard::codes(*tkl).len(), "Touches"),
    };
    let mut m = Model::new(*name, DeviceType::Keyboard, vec![ZoneInfo::fixed(zone, leds as u32)]);
    match kind {
        Kind::PerKey => m.interface = Some(1),
        // Collection de rapports 20 octets (la note : interface 1, page 0xFF43, usage 0x0602).
        Kind::G815 => (m.interface, m.usage_page, m.usage) = (Some(1), Some(0xFF43), Some(0x0602)),
        // HID++ long moderne (la note : interface 2, page 0xFF00, usage 2).
        Kind::G915 { .. } => (m.interface, m.usage_page, m.usage) = (Some(2), Some(0xFF00), Some(2)),
    }
    Some(m)
}

fn open(hid: Box<dyn Hid>, model: &Model) -> Result<Box<dyn NativeRgb>> {
    // Le nom du modèle identifie le protocole (un nom = un protocole dans MODELS).
    let Some((.., kind)) = MODELS.iter().find(|(_, n, _)| *n == model.name) else {
        bail!("Logitech : modèle {} non géré", model.name);
    };
    match kind {
        Kind::PerKey => per_key::open(hid),
        Kind::G815 => keyboard::open_g815(hid),
        Kind::G915 { tkl } => keyboard::open_g915(hid, *tkl),
    }
}

/// Trame HID++ : `report` 0x11 = 20 octets, 0x12 = 64 octets ; `function` = octet « fonction << 4 | swId » complet.
fn frame(report: u8, feature: u8, function: u8) -> Vec<u8> {
    let mut f = vec![0u8; if report == REPORT_LONG { 20 } else { 64 }];
    f[0] = report;
    f[1] = DEVICE;
    f[2] = feature;
    f[3] = function;
    f
}

/// Écrit `req` puis attend la réponse de même (feature, fonction) ; ignore les autres rapports.
fn exchange(hid: &dyn Hid, req: &[u8]) -> Result<Vec<u8>> {
    hid.write(req)?;
    for _ in 0..REPLY_TRIES {
        let mut buf = [0u8; 64];
        let n = hid.read_timeout(&mut buf, REPLY_TIMEOUT_MS)?;
        if n == 0 {
            bail!("Logitech : pas de réponse (feature {:#04x}, fonction {:#04x})", req[2], req[3]);
        }
        if n < 5 || !matches!(buf[0], REPORT_LONG | REPORT_VLONG) || buf[1] != DEVICE {
            continue;
        }
        if buf[2] == FEATURE_ERROR {
            bail!("Logitech : erreur HID++ (feature {:#04x}, fonction {:#04x})", req[2], req[3]);
        }
        if buf[2] == req[2] && buf[3] == req[3] {
            return Ok(buf[..n].to_vec());
        }
    }
    bail!("Logitech : réponse introuvable (feature {:#04x}, fonction {:#04x})", req[2], req[3])
}

/// IRoot (feature index 0, fonction 0) : id de feature -> index ; `None` si absente de l'appareil.
fn lookup(hid: &dyn Hid, report: u8, id: u16) -> Result<Option<u8>> {
    let mut req = frame(report, 0x00, SW_ID);
    req[4..6].copy_from_slice(&id.to_be_bytes());
    let reply = exchange(hid, &req)?;
    Ok(Some(reply[4]).filter(|i| *i != 0))
}

pub const FAMILY: Family = Family { id: "logitech", doc: "docs/rgb-protocols/logitech.md", probe, open };

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_claims_only_documented_pids() {
        for (pid, interface, usage_page, usage, leds) in [
            (0xC330, Some(1), None, None, 106),
            (0xC331, Some(1), None, None, 106),
            (0xC337, Some(1), None, None, 106),
            (0xC32B, Some(1), None, None, 106),
            (0xC335, Some(1), None, None, 106),
            (0xC342, Some(1), None, None, 106),
            (0xC33C, Some(1), None, None, 106),
            (0xC333, Some(1), None, None, 106),
            (0xC338, Some(1), None, None, 106),
            (0xC33F, Some(1), Some(0xFF43), Some(0x0602), 106),
            (0xC33E, Some(2), Some(0xFF00), Some(2), 106),
            (0xC343, Some(2), Some(0xFF00), Some(2), 89),
        ] {
            let m = probe(VID, pid).expect("pid");
            assert_eq!((m.interface, m.usage_page, m.usage, m.led_count()), (interface, usage_page, usage, leds), "{pid:04x}");
        }
        for pid in [0xC232, 0xC336, 0xC33A, 0xC339, 0xC541, 0xC547, 0xC545, 0xC355, 0xC53A, 0xC539, 0xC092, 0xC09D, 0xC084, 0xC08B] {
            assert!(probe(VID, pid).is_none(), "{pid:04x} ne doit pas être revendiqué");
        }
        assert!(probe(0x1038, 0xC330).is_none());
    }

    #[test]
    fn names_identify_one_protocol() {
        for (_, name, kind) in MODELS {
            let same = MODELS.iter().filter(|(_, n, _)| n == name).all(|(.., k)| std::mem::discriminant(k) == std::mem::discriminant(kind));
            assert!(same, "{name}");
        }
    }
}
