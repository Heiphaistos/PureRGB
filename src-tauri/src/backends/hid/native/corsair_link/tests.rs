//! Trames écrites à la main d'après docs/rgb-protocols/corsair.md (famille B) et analyse-corsair-icue.md §5, §12.

use super::*;
use crate::backends::hid::native::{probe as probe_all, MockHid};

type Sent = std::sync::Arc<parking_lot::Mutex<Vec<Vec<u8>>>>;

/// Rapport d'écriture attendu : `[00][00][01][commande…]` complété à 513 octets.
fn frame(cmd: &[u8]) -> Vec<u8> {
    let mut v = vec![0, 0, 1];
    v.extend_from_slice(cmd);
    v.resize(513, 0);
    v
}
fn ack() -> Vec<u8> {
    vec![0, 0, 0, 0]
}
fn fw() -> Vec<u8> {
    vec![0, 0, 0, 0, 1, 2, 0x34, 0x12] // 1.2.0x1234
}
/// Réponse à la lecture de `36` : en-tête de 3 o, statut, type `21 00`, nombre de canaux, fiches `(type, modèle, id)`.
fn chain(cards: &[(u8, u8, &str)]) -> Vec<u8> {
    let mut v = vec![0, 0, 0, 0, 0x21, 0x00, cards.len() as u8];
    for (t, m, id) in cards {
        v.extend_from_slice(&[0, 0, *t, *m, 0, 0, 0, id.len() as u8]);
        v.extend_from_slice(id.as_bytes());
    }
    v
}
/// Réponses de l'ouverture : firmware, fermer, ouvrir, lecture, fermer.
fn open_replies(chain: Vec<u8>) -> Vec<Vec<u8>> {
    vec![fw(), ack(), ack(), chain, ack()]
}
fn open_dev(replies: Vec<Vec<u8>>) -> (Result<Box<dyn NativeRgb>>, Sent) {
    let (mock, sent, _) = MockHid::new(replies);
    let model = probe(VID, PID).unwrap();
    (open(Box::new(mock), &model), sent)
}
fn acks(n: usize) -> Vec<Vec<u8>> {
    vec![ack(); n]
}

#[test]
fn probe_claims_only_the_link_hub() {
    assert!(probe_all(VID, PID).is_some_and(|(f, _)| f.id == "corsair_link"));
    for pid in [0x0C1C, 0x0C32, 0x0C2A, 0x0C10, 0x0C3E, 0x0C40] {
        assert!(probe_all(VID, pid).is_none_or(|(f, _)| f.id != "corsair_link"), "{pid:04X}");
    }
    assert!(probe(0x1234, PID).is_none());
    let m = probe(VID, PID).unwrap();
    assert_eq!((m.usage_page, m.fans, m.zones.len(), m.led_count()), (Some(0xFF42), 0, 1, 476));
    assert!(m.zones[0].resizable());
    assert_eq!((m.zones[0].leds_min, m.zones[0].leds_max), (1, 476));
}

#[test]
fn open_only_reads_firmware_and_enumerates_the_chain() {
    // QX (34) + canal vide + LX (18) + RX sans LED : 52 LED.
    let c = chain(&[(0x01, 0x00, "AB12"), (0x00, 0x00, ""), (0x02, 0x00, "CD"), (0x13, 0x00, "E")]);
    let (dev, sent) = open_dev(open_replies(c));
    assert!(dev.is_ok());
    let want = vec![
        frame(&[0x02, 0x13]),
        frame(&[0x05, 0x01, 0x01, 0x36]),
        frame(&[0x0D, 0x01, 0x36]),
        frame(&[0x08, 0x01]),
        frame(&[0x05, 0x01, 0x01, 0x36]),
    ];
    assert_eq!(*sent.lock(), want);
}

#[test]
fn enumeration_waits_for_the_right_data_type() {
    let mut r = open_replies(chain(&[(0x01, 0x00, "A")]));
    r.insert(3, vec![0, 0, 0, 0, 0x25, 0x00, 1, 0]); // rapport d'un autre type (vitesses), à ignorer
    let (dev, sent) = open_dev(r);
    assert!(dev.is_ok());
    assert_eq!(sent.lock().len(), 5); // aucune écriture de plus : seule une relecture
}

#[test]
fn colors_use_the_documented_direct_sequence_with_padding() {
    // QX 34 + LX 18 = 52 LED.
    let mut r = open_replies(chain(&[(0x01, 0x00, "A"), (0x02, 0x00, "B")]));
    r.extend(acks(5)); // fermer, ouvrir (couleur), 1 morceau, fermer = 4, +1 marge
    let (dev, sent) = open_dev(r);
    let mut dev = dev.unwrap();
    let mut colors = vec![Color::BLACK; 40]; // moins que la chaîne : complété en noir
    colors[0] = Color::new(1, 2, 3);
    colors[39] = Color::new(7, 8, 9);
    dev.set_colors(&colors).unwrap();
    let log = sent.lock();
    let f = &log[5..];
    assert_eq!(f.len(), 4);
    assert_eq!(f[0], frame(&[0x05, 0x01, 0x01, 0x22]));
    assert_eq!(f[1], frame(&[0x0D, 0x00, 0x22]));
    // 52 x 3 = 156 octets ; longueur = 156 + 2 = 158 = 0x9E ; type 12 00.
    let mut want = vec![0x06, 0x00, 0x9E, 0x00, 0x00, 0x00, 0x12, 0x00, 1, 2, 3];
    want.resize(8 + 39 * 3, 0);
    want.extend_from_slice(&[7, 8, 9]);
    assert_eq!(f[2], frame(&want));
    assert_eq!(f[3], frame(&[0x05, 0x01, 0x01, 0x22]));
}

#[test]
fn full_chain_is_split_in_508_byte_chunks() {
    // 14 QX = 476 LED = 1428 octets ; charge = 6 + 1428 = 1434 = 508 + 508 + 418.
    let cards: Vec<(u8, u8, &str)> = (0..14).map(|_| (0x01, 0x00, "Q")).collect();
    let mut r = open_replies(chain(&cards));
    r.extend(acks(6));
    let (dev, sent) = open_dev(r);
    let mut dev = dev.unwrap();
    let mut colors = vec![Color::BLACK; 600]; // excédent ignoré
    colors[167] = Color::new(1, 2, 3); // octets 501, 502, 503 du tampon
    dev.set_colors(&colors).unwrap();
    let log = sent.lock();
    let f = &log[5..];
    assert_eq!(f.len(), 6);
    // 1430 = 0x0596.
    assert_eq!(f[2][..11], [0, 0, 1, 0x06, 0x00, 0x96, 0x05, 0x00, 0x00, 0x12, 0x00]);
    assert_eq!(f[2][3 + 2 + 507], 1); // dernier octet du 1er morceau = octet 501
    assert_eq!(f[3][..8], [0, 0, 1, 0x07, 0x00, 2, 3, 0]);
    assert_eq!(f[4][..5], [0, 0, 1, 0x07, 0x00]);
    assert!(f.iter().all(|x| x.len() == 513));
    // Dernier morceau : 418 octets de données, puis du zéro.
    assert!(f[4][5 + 418..].iter().all(|b| *b == 0));
}

#[test]
fn nothing_persistent_is_ever_written() {
    let mut r = open_replies(chain(&[(0x0D, 0x00, "G")]));
    r.extend(acks(4));
    let (dev, sent) = open_dev(r);
    let mut dev = dev.unwrap();
    dev.set_colors(&[Color::new(5, 5, 5); 16]).unwrap();
    assert!(dev.set_fan_duty(0, 50).unwrap_err().to_string().contains("refusée"));
    dev.release();
    for f in sent.lock().iter() {
        assert_ne!(f[3..5], [0x01, 0x03], "changement de mode interdit");
        let resource = match f[3] {
            0x05 => Some(f[6]),
            0x0D => Some(f[5]),
            _ => None,
        };
        assert!(resource.is_none_or(|r| r == 0x36 || r == 0x22), "ressource {resource:02X?} touchée");
    }
}

#[test]
fn unknown_or_conflicting_devices_are_refused_without_any_color_write() {
    for cards in [
        vec![(0x01, 0x00, "A"), (0x99, 0x00, "X")],  // type inconnu
        vec![(0x14, 0x00, "O")],                     // ORIGIN OA : 0 LED (OpenRGB) contre 32 (iCUE)
        vec![(0x0A, 0x00, "G")],                     // XG3 Hybrid : 0 contre 18
        vec![(0x01, 0x07, "A")],                     // modèle inconnu
        vec![(0x13, 0x00, "R")],                     // aucune LED dans la chaîne
    ] {
        let (dev, sent) = open_dev(open_replies(chain(&cards)));
        assert!(dev.is_err(), "{cards:?}");
        let log = sent.lock();
        assert_eq!(log.len(), 5, "uniquement firmware + énumération");
        assert!(log.iter().all(|f| f[3] != 0x06 && f[3] != 0x07));
    }
}

#[test]
fn bad_enumeration_replies_are_refused() {
    let ok = chain(&[(0x01, 0x00, "ABCD")]);
    // Tronquée au milieu de l'identifiant, au milieu de la fiche, avant le nombre de canaux.
    for cut in [ok.len() - 1, 12, 6] {
        assert!(open_dev(open_replies(ok[..cut].to_vec())).0.is_err(), "coupée à {cut}");
    }
    // Plus de canaux annoncés que de fiches présentes.
    let mut lying = ok.clone();
    lying[6] = 3;
    assert!(open_dev(open_replies(lying)).0.is_err());
    // 25 canaux annoncés (max 24).
    let mut many = vec![0, 0, 0, 0, 0x21, 0x00, 25];
    many.resize(512, 0);
    assert!(open_dev(open_replies(many)).0.is_err());
    // Plus de 476 LED : 15 QX = 510.
    let cards: Vec<(u8, u8, &str)> = (0..15).map(|_| (0x01, 0x00, "Q")).collect();
    assert!(open_dev(open_replies(chain(&cards))).0.is_err());
}

#[test]
fn silent_or_wrong_device_is_refused() {
    assert!(open_dev(vec![]).0.is_err()); // pas de réponse
    assert!(open_dev(vec![vec![0, 0, 0, 0, 1]]).0.is_err()); // firmware trop court
    // Le type 21 00 n'arrive jamais : 1 réponse + 5 relectures, puis refus.
    let mut r = vec![fw(), ack(), ack()];
    r.extend(std::iter::repeat(vec![0, 0, 0, 0, 0x25, 0x00, 0]).take(6));
    let (dev, sent) = open_dev(r);
    assert!(dev.is_err());
    assert_eq!(sent.lock().len(), 5); // la fermeture est tout de même tentée
}

#[test]
fn colors_report_a_hub_that_stops_answering() {
    let mut r = open_replies(chain(&[(0x01, 0x00, "A")]));
    r.push(ack()); // fermer, puis plus rien
    let (dev, _) = open_dev(r);
    assert!(dev.unwrap().set_colors(&[Color::BLACK; 34]).is_err());
}
