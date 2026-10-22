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
    assert_eq!(table::MODELS.len(), 82);
    let ids: std::collections::HashSet<_> = table::MODELS.iter().map(|e| e.pid).collect();
    let names: std::collections::HashSet<_> = table::MODELS.iter().map(|e| e.name).collect();
    assert_eq!((ids.len(), names.len()), (82, 82)); // `open` retrouve l'entree par nom : noms uniques
    let cobra = probe(VID, 0x00A3).expect("Cobra");
    assert_eq!((cobra.interface, cobra.usage_page, cobra.led_count()), (Some(3), Some(0x000C), 10));
    assert_eq!(probe(VID, 0x006C).expect("Mamba Elite").led_count(), 20);
    assert!(probe(VID, 0x0068).is_none()); // Hyperflux : init supplementaire INCONNUE ici
    assert!(probe(VID, 0x007C).is_none()); // sans fil : non repris
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

// --- Entrées de analyse-razer-synapse.md : TID connu, pas de sondage du serial. CRC calculés à la main. ---

#[test]
fn new_entries_are_claimed_with_note_facts() {
    let k = probe(VID, 0x0257).expect("Huntsman Mini");
    assert_eq!((k.interface, k.usage_page, k.led_count()), (Some(2), Some(0x0001), 75));
    let v4 = probe(VID, 0x0287).expect("BlackWidow V4");
    assert_eq!((v4.interface, v4.usage_page, v4.led_count()), (Some(3), None, 184)); // page selon firmware < / >= 1.5
    assert_eq!(probe(VID, 0x02A5).expect("V4 75%").usage_page, None);
    assert_eq!(probe(VID, 0x02B3).expect("V4 Pro 75%").usage_page, Some(0x000C));
    let elite = probe(VID, 0x0226).expect("Huntsman Elite");
    assert_eq!((elite.zones.len(), elite.led_count()), (2, 198));
    assert_eq!((elite.zones[0].led_count, elite.zones[1].led_count), (132, 66));
    let pad = probe(VID, 0x0F26).expect("Charging Pad");
    assert_eq!((pad.interface, pad.usage_page, pad.led_count()), (Some(0), Some(0x000C), 10));
    assert_eq!(probe(VID, 0x0C04).expect("Firefly V2").led_count(), 19);
    let mamba = probe(VID, 0x006C).expect("Mamba Elite");
    assert_eq!(mamba.zones.iter().map(|z| z.led_count).collect::<Vec<_>>(), vec![1, 1, 9, 9]);
    assert_eq!(probe(VID, 0x0099).expect("Basilisk V3").zones.len(), 3);
    // Non revendiqués : sans fil / dongle, Bluetooth, Basilisk Ultimate (14 ou 18 LED : INCONNU), ARGB, Kraken, Hanbo, Mouse Dock Pro, STA.
    for pid in [0x0271u16, 0x02D5, 0x02D2, 0x0290, 0x0296, 0x025C, 0x025B, 0x0086, 0x0088, 0x0F1F, 0x0501, 0x056B, 0x0F35, 0x00A4, 0x0209, 0x0215, 0x0C00, 0x0068] {
        assert!(probe(VID, pid).is_none(), "{pid:04X} ne doit pas etre revendique");
    }
    let ids: std::collections::HashSet<_> = table::MODELS.iter().map(|e| e.pid).collect();
    let names: std::collections::HashSet<_> = table::MODELS.iter().map(|e| e.name).collect();
    assert_eq!((ids.len(), names.len(), table::MODELS.len()), (table::MODELS.len(), table::MODELS.len(), 82));
    assert!(table::MODELS.iter().all(|e| matches!(e.tid, None | Some(0x1F | 0x3F | 0x9F))));
}

#[test]
fn known_tid_skips_serial_probe_and_keeps_note_tid() {
    // Huntsman Mini (TID 0x3F) : détection puis mode logiciel, aucune commande serial.
    let (dev, _, feats) = open_pid(0x0257, vec![ok(), ok()]);
    assert!(dev.is_ok());
    let f = feats.lock();
    assert_eq!(f.len(), 2);
    assert_eq!(f[0], rep(&[(2, 0x3F), (6, 6), (7, 0x0F), (8, 0x82), (89, 0x8B)]));
    assert_eq!(f[1], rep(&[(2, 0x3F), (6, 6), (7, 0x0F), (8, 0x02), (11, 0x08), (13, 0x01), (89, 0x02)]));
}

#[test]
fn known_tid_open_fails_without_modern_matrix() {
    let (r, _, feats) = open_pid(0x0257, vec![rep(&[(1, 5)])]);
    assert!(r.is_err());
    assert_eq!(feats.lock().len(), 1);
}

#[test]
fn wired_hyperspeed_keyboard_uses_tid_9f() {
    // DeathStalker V2 Pro filaire (0292) : TID 0x9F.
    let (dev, _, feats) = open_pid(0x0292, vec![ok(), ok()]);
    assert!(dev.is_ok());
    assert_eq!(feats.lock()[0], rep(&[(2, 0x9F), (6, 6), (7, 0x0F), (8, 0x82), (89, 0x8B)]));
}

#[test]
fn five_by_fifteen_keyboard_sends_five_rows_of_fifteen() {
    let (dev, _, feats) = open_pid(0x0257, vec![ok(), ok()]);
    let mut dev = dev.expect("ouverture");
    dev.set_colors(&[Color::new(0x10, 0x20, 0x30)]).expect("couleurs");
    let f = feats.lock();
    assert_eq!(f.len(), 2 + 5);
    // ligne 0 : size 5+45 = 0x32, args 00 00 00 00 0E | 10 20 30 ; CRC = 32^0F^03^0E^10^20^30 = 30
    assert_eq!(f[2], rep(&[(2, 0x3F), (6, 0x32), (7, 0x0F), (8, 0x03), (13, 0x0E), (14, 0x10), (15, 0x20), (16, 0x30), (89, 0x30)]));
    // ligne 4 : noire ; CRC = 32^0F^03^04^0E = 34
    assert_eq!(f[6], rep(&[(2, 0x3F), (6, 0x32), (7, 0x0F), (8, 0x03), (11, 0x04), (13, 0x0E), (89, 0x34)]));
}

#[test]
fn nine_by_twenty_two_keyboard_sends_nine_rows() {
    let (dev, _, feats) = open_pid(0x0226, vec![ok(), ok()]);
    let mut dev = dev.expect("ouverture");
    dev.set_colors(&[]).expect("couleurs");
    let f = feats.lock();
    assert_eq!(f.len(), 2 + 9);
    // ligne 8 : size 0x47, args 00 00 08 00 15 ; CRC = 47^0F^03^08^15 = 56
    assert_eq!(f[10], rep(&[(2, 0x3F), (6, 0x47), (7, 0x0F), (8, 0x03), (11, 0x08), (13, 0x15), (89, 0x56)]));
    drop(f);
    assert!(dev.set_colors(&[Color::BLACK; 199]).is_err());
}

#[test]
fn v4_eight_by_twenty_three_rows() {
    let (dev, _, feats) = open_pid(0x0287, vec![ok(), ok()]);
    let mut dev = dev.expect("ouverture");
    dev.set_colors(&[Color::BLACK; 184]).expect("couleurs");
    let f = feats.lock();
    assert_eq!(f.len(), 2 + 8);
    // size 5+69 = 0x4A, dernière colonne 0x16 ; ligne 0 : CRC = 4A^0F^03^16 = 50 ; ligne 7 : 50^07 = 57
    assert_eq!(f[2], rep(&[(2, 0x1F), (6, 0x4A), (7, 0x0F), (8, 0x03), (13, 0x16), (89, 0x50)]));
    assert_eq!(f[9], rep(&[(2, 0x1F), (6, 0x4A), (7, 0x0F), (8, 0x03), (11, 0x07), (13, 0x16), (89, 0x57)]));
}

#[test]
fn single_row_accessory_and_hardware_release() {
    // Firefly V2 : 1 x 19, TID 0x3F, size 5+57 = 0x3E ; CRC (R = 1) = 3E^0F^03^12^01 = 21
    let (dev, _, feats) = open_pid(0x0C04, vec![ok(), ok()]);
    let mut dev = dev.expect("ouverture");
    dev.set_colors(&[Color::new(1, 0, 0)]).expect("couleurs");
    dev.release();
    let f = feats.lock();
    assert_eq!(f[2], rep(&[(2, 0x3F), (6, 0x3E), (7, 0x0F), (8, 0x03), (13, 0x12), (14, 1), (89, 0x21)]));
    // retour au mode matériel : classe 00 cmd 04 args 00 00 ; CRC = 02^00^04 = 06
    assert_eq!(f[3], rep(&[(2, 0x3F), (6, 2), (7, 0x00), (8, 0x04), (89, 0x06)]));
}

#[test]
fn charging_pad_sends_software_mode_first() {
    let (dev, _, feats) = open_pid(0x0F26, vec![ok(), ok(), ok()]);
    assert!(dev.is_ok());
    let f = feats.lock();
    assert_eq!(f.len(), 3);
    // mode logiciel : classe 00 cmd 04 args 03 00, size 2 ; CRC = 02^00^04^03 = 05
    assert_eq!(f[0], rep(&[(2, 0x1F), (6, 2), (7, 0x00), (8, 0x04), (9, 0x03), (89, 0x05)]));
    assert_eq!(f[1][8], 0x82); // puis détection de la matrice moderne
    drop(f);
    // refus du mode logiciel : rien d'autre n'est envoyé
    let (r, _, feats) = open_pid(0x0F26, vec![rep(&[(1, 3)])]);
    assert!(r.is_err());
    assert_eq!(feats.lock().len(), 1);
}

#[test]
fn mamba_elite_single_row_of_twenty() {
    let (dev, _, feats) = open_pid(0x006C, vec![ok_serial(), ok(), ok()]);
    let mut dev = dev.expect("ouverture");
    dev.set_colors(&[Color::BLACK; 20]).expect("couleurs");
    // size 5+60 = 0x41, dernière colonne 0x13 ; CRC : 41^0F=4E, ^03=4D, ^13=5E
    assert_eq!(feats.lock()[3], rep(&[(2, 0x1F), (6, 0x41), (7, 0x0F), (8, 0x03), (13, 0x13), (89, 0x5E)]));
}
