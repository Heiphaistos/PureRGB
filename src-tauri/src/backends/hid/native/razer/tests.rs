//! Tests d'octets : trames écrites à la main d'après razer.md / signalrgb.md (CRC = XOR des octets de trame 2..=87,
//! donc indices 3..=88 du rapport ; le txid, à l'indice 2, n'y entre pas).

use super::*;
use crate::backends::hid::native::{probe as global_probe, MockHid};

/// Rapport de 91 octets, zéros sauf les `(indice, valeur)` donnés.
fn rep(set: &[(usize, u8)]) -> Vec<u8> {
    let mut v = vec![0u8; 91];
    for &(i, x) in set {
        v[i] = x;
    }
    v
}

/// Réponse « OK » (statut 2) avec un serial non nul.
fn ok_serial() -> Vec<u8> {
    rep(&[(1, 2), (9, b'P')])
}
fn ok() -> Vec<u8> {
    rep(&[(1, 2)])
}

fn open_pid(pid: u16, replies: Vec<Vec<u8>>) -> (Result<Box<dyn NativeRgb>>, super::super::Log, super::super::Log) {
    let (mock, sent, feats) = MockHid::new(replies);
    let model = probe(VID, pid).expect("pid revendiqué");
    (open(Box::new(mock), &model), sent, feats)
}

#[test]
fn probe_claims_only_listed_pids_with_interface() {
    assert!(probe(0x046D, 0x0099).is_none());
    assert!(probe(VID, 0x0F1F).is_none()); // contrôleur ARGB : non lu
    assert!(probe(VID, 0x0225).is_none()); // portable : dimensions INCONNUES
    let m = probe(VID, 0x0099).expect("Basilisk V3");
    assert_eq!((m.interface, m.usage_page, m.led_count()), (Some(3), Some(0x000C), 11));
    let k = probe(VID, 0x0228).expect("Elite");
    assert_eq!((k.interface, k.usage_page, k.led_count()), (Some(2), Some(0x0001), 132));
    assert_eq!(probe(VID, 0x024E).expect("V3").interface, Some(3));
    assert_eq!(table::MODELS.len(), 7);
    assert!(global_probe(VID, 0x0221).map(|(f, _)| f.id) == Some("razer"));
}

#[test]
fn open_sends_probe_detect_and_software_mode() {
    let (dev, _, feats) = open_pid(0x0099, vec![ok_serial(), ok(), ok()]);
    assert!(dev.is_ok());
    let f = feats.lock();
    assert_eq!(f.len(), 3);
    // serial : txid 1F, size 2, classe 00, cmd 82 ; CRC = 02^00^82 = 80
    assert_eq!(f[0], rep(&[(2, 0x1F), (6, 2), (7, 0x00), (8, 0x82), (89, 0x80)]));
    // matrice moderne : size 6, classe 0F, cmd 82, arg 00 ; CRC = 06^0F^82 = 8B
    assert_eq!(f[1], rep(&[(2, 0x1F), (6, 6), (7, 0x0F), (8, 0x82), (89, 0x8B)]));
    // mode logiciel : size 6, classe 0F, cmd 02, args 00 00 08 00 01 ; CRC = 06^0F^02^08^01 = 02
    assert_eq!(f[2], rep(&[(2, 0x1F), (6, 6), (7, 0x0F), (8, 0x02), (11, 0x08), (13, 0x01), (89, 0x02)]));
}

#[test]
fn txid_probe_moves_on_when_serial_is_empty() {
    let (dev, _, feats) = open_pid(0x0099, vec![ok(), ok_serial(), ok(), ok()]);
    assert!(dev.is_ok());
    let f = feats.lock();
    assert_eq!(f[0][2], 0x1F);
    assert_eq!(f[1], rep(&[(2, 0x2F), (6, 2), (8, 0x82), (89, 0x80)]));
    assert_eq!(f[2][2], 0x2F); // la suite utilise le txid trouvé
}

#[test]
fn open_fails_without_txid_or_without_modern_matrix() {
    assert!(open_pid(0x0099, vec![]).0.is_err());
    // détection de matrice : statut 5 (non supporté) -> chemin ancien refusé
    let refused = rep(&[(1, 5)]);
    let (r, _, feats) = open_pid(0x0099, vec![ok_serial(), refused]);
    assert!(r.is_err());
    assert_eq!(feats.lock().len(), 2); // aucun mode logiciel envoyé
    // mode logiciel refusé
    assert!(open_pid(0x0099, vec![ok_serial(), ok(), rep(&[(1, 3)])]).0.is_err());
}

#[test]
fn mouse_color_row_is_single_packet() {
    let (dev, _, feats) = open_pid(0x0096, vec![ok_serial(), ok(), ok()]);
    let mut dev = dev.expect("ouverture");
    dev.set_colors(&[Color::new(1, 2, 3), Color::new(4, 5, 6)]).expect("couleurs");
    let f = feats.lock();
    assert_eq!(f.len(), 4);
    // size 3*2+5 = 0B, classe 0F, cmd 03, args 00 00 00 00 01 | 01 02 03 | 04 05 06 ; CRC = 01
    let want = rep(&[(2, 0x1F), (6, 0x0B), (7, 0x0F), (8, 0x03), (13, 0x01), (14, 1), (15, 2), (16, 3), (17, 4), (18, 5), (19, 6), (89, 0x01)]);
    assert_eq!(f[3], want);
}

#[test]
fn keyboard_sends_six_rows_and_pads_with_black() {
    let (dev, _, feats) = open_pid(0x0228, vec![ok_serial(), ok(), ok()]);
    let mut dev = dev.expect("ouverture");
    dev.set_colors(&[Color::new(0xAA, 0xBB, 0xCC)]).expect("couleurs");
    let f = feats.lock();
    assert_eq!(f.len(), 3 + 6);
    // ligne 0 : size 0x47, args 00 00 00 00 15 | AA BB CC | zéros ; CRC = 47^0F^03^15^AA^BB^CC = 83
    assert_eq!(f[3], rep(&[(2, 0x1F), (6, 0x47), (7, 0x0F), (8, 0x03), (13, 0x15), (14, 0xAA), (15, 0xBB), (16, 0xCC), (89, 0x83)]));
    // ligne 5 : tout noir ; CRC = 47^0F^03^05^15 = 5B
    assert_eq!(f[8], rep(&[(2, 0x1F), (6, 0x47), (7, 0x0F), (8, 0x03), (11, 0x05), (13, 0x15), (89, 0x5B)]));
}

#[test]
fn set_colors_rejects_too_many_colors() {
    let (dev, _, feats) = open_pid(0x0096, vec![ok_serial(), ok(), ok()]);
    let mut dev = dev.expect("ouverture");
    assert!(dev.set_colors(&[Color::BLACK; 3]).is_err());
    assert_eq!(feats.lock().len(), 3); // rien envoyé
}

#[test]
fn frame_rejects_oversized_args() {
    assert!(frame(0x1F, 0, 0, 0, &[0u8; 81]).is_err());
    assert!(frame(0x1F, 0, 0, 0, &[0u8; 80]).is_ok());
}

#[test]
fn release_restores_hardware_mode() {
    let (dev, _, feats) = open_pid(0x0228, vec![ok_serial(), ok(), ok()]);
    dev.expect("ouverture").release();
    // clavier : effet spectre (classe 0F cmd 02, args 00 00 03, size 6) ; CRC = 06^0F^02^03 = 08
    assert_eq!(feats.lock()[3], rep(&[(2, 0x1F), (6, 6), (7, 0x0F), (8, 0x02), (11, 0x03), (89, 0x08)]));
    let (dev, _, feats) = open_pid(0x0099, vec![ok_serial(), ok(), ok()]);
    dev.expect("ouverture").release();
    // souris : mode appareil matériel (classe 00 cmd 04, args 00 00, size 2) ; CRC = 02^00^04 = 06
    assert_eq!(feats.lock()[3], rep(&[(2, 0x1F), (6, 2), (7, 0x00), (8, 0x04), (89, 0x06)]));
}
