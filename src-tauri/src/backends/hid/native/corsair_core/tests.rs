//! Trames écrites à la main d'après docs/rgb-protocols/corsair.md (famille A).

use super::*;
use crate::backends::hid::native::{probe as probe_all, MockHid};

type Sent = std::sync::Arc<parking_lot::Mutex<Vec<Vec<u8>>>>;

fn pad(head: &[u8], n: usize) -> Vec<u8> {
    let mut v = head.to_vec();
    v.resize(n, 0);
    v
}
fn ack(cmd0: u8) -> Vec<u8> {
    vec![0x00, cmd0]
}
fn fw(major: u8) -> Vec<u8> {
    vec![0x00, 0x02, 0x00, major, 0x05, 0x07]
}
/// Réponses de l'ouverture : réveil, firmware, ouvrir, écrire, fermer, réveil.
fn open_replies(major: u8) -> Vec<Vec<u8>> {
    vec![ack(0x01), fw(major), ack(0x0D), ack(0x06), ack(0x05), ack(0x01)]
}
fn open_dev(pid: u16, replies: Vec<Vec<u8>>) -> (Result<Box<dyn NativeRgb>>, Sent) {
    let (mock, sent, _) = MockHid::new(replies);
    let model = probe(VID, pid).unwrap();
    (open(Box::new(mock), &model), sent)
}

#[test]
fn probe_claims_only_documented_pids() {
    for pid in [0x0C1C, 0x0C32, 0x0C2A] {
        assert!(probe_all(VID, pid).is_some_and(|(f, _)| f.id == "corsair_core"));
    }
    for pid in [0x0C10, 0x0C0B, 0x0C1A, 0x1D00, 0x0C3F, 0x0C17, 0x0C1D, 0x0C3C] {
        assert!(probe_all(VID, pid).is_none_or(|(f, _)| f.id != "corsair_core"), "{pid:04X}");
    }
    assert!(probe(0x1234, 0x0C1C).is_none());
    let core = probe(VID, 0x0C1C).unwrap();
    assert_eq!((core.led_count(), core.zones.len(), core.usage_page, core.fans), (233, 7, Some(0xFF42), 0));
    assert_eq!(probe(VID, 0x0C2A).unwrap().led_count(), 204);
}

#[test]
fn open_core_sends_documented_init_sequence() {
    let (dev, sent) = open_dev(0x0C1C, open_replies(2));
    assert!(dev.is_ok());
    let want = vec![
        pad(&[0, 8, 0x01, 0x03, 0x00, 0x02], 97),
        pad(&[0, 8, 0x02, 0x13], 97),
        pad(&[0, 8, 0x0D, 0x00, 0x1E, 0x00], 97),
        pad(&[0, 8, 0x06, 0x00, 17, 0, 0, 0, 0x0D, 0x00, 0x07, 0x01, 0x08, 1, 6, 1, 6, 1, 6, 1, 6, 1, 6, 1, 6], 97),
        pad(&[0, 8, 0x05, 0x01, 0x00], 97),
        pad(&[0, 8, 0x01, 0x03, 0x00, 0x02], 97),
    ];
    assert_eq!(*sent.lock(), want);
}

#[test]
fn open_xt_uses_no_aio_port_mode() {
    let (dev, sent) = open_dev(0x0C2A, open_replies(2));
    assert!(dev.is_ok());
    let log = sent.lock();
    assert_eq!(log[3], pad(&[0, 8, 0x06, 0x00, 17, 0, 0, 0, 0x0D, 0x00, 0x07, 0x00, 0x00, 1, 6, 1, 6, 1, 6, 1, 6, 1, 6, 1, 6], 385));
}

#[test]
fn core_firmware_major_1_switches_to_1025_byte_reports() {
    let (dev, sent) = open_dev(0x0C1C, open_replies(1));
    assert!(dev.is_ok());
    let lens: Vec<usize> = sent.lock().iter().map(Vec::len).collect();
    assert_eq!(lens, vec![97, 97, 1025, 1025, 1025, 1025]);
    // Le ST (0C32) ne change jamais de taille.
    let (_, sent) = open_dev(0x0C32, open_replies(1));
    assert!(sent.lock().iter().all(|f| f.len() == 65));
}

#[test]
fn open_rejects_bad_replies() {
    assert!(open_dev(0x0C1C, vec![]).0.is_err()); // pas de réponse
    assert!(open_dev(0x0C1C, vec![ack(0x09)]).0.is_err()); // écho incorrect
    assert!(open_dev(0x0C1C, vec![ack(0x01), vec![0, 2, 0]]).0.is_err()); // firmware trop court
    // Un rapport d'octet 0 non nul est ignoré, le suivant est lu.
    let mut r = open_replies(2);
    r.insert(0, vec![0x05, 0x01]);
    assert!(open_dev(0x0C1C, r).0.is_ok());
}

#[test]
fn core_colors_are_split_in_packets_with_fixed_34_led_stride() {
    let mut replies = open_replies(2);
    replies.extend([ack(0x0D), ack(0x06)]);
    replies.extend(std::iter::repeat(ack(0x07)).take(7));
    replies.push(ack(0x05));
    let (dev, sent) = open_dev(0x0C1C, replies);
    let mut dev = dev.unwrap();
    let mut colors = vec![Color::BLACK; 233];
    colors[0] = Color::new(1, 2, 3); // pompe, LED 0
    colors[29] = Color::new(9, 8, 7); // ventilateur 1, LED 0 = octet 87
    colors[232] = Color::new(4, 5, 6); // ventilateur 6, dernière LED = octets 696..698
    dev.set_colors(&colors).unwrap();
    let log = sent.lock();
    let frames = &log[6..];
    assert_eq!(frames.len(), 10);
    assert_eq!(frames[0], pad(&[0, 8, 0x0D, 0x00, 0x22, 0x00], 97));
    // 699 + 2 (type) = 701 = 0x02BD ; 87 octets de données dans le 1er paquet.
    assert_eq!(frames[1], pad(&[0, 8, 0x06, 0x00, 0xBD, 0x02, 0, 0, 0x12, 0x00, 1, 2, 3], 97));
    assert_eq!(frames[2], pad(&[0, 8, 0x07, 0x00, 9, 8, 7], 97));
    for f in &frames[3..9] {
        assert_eq!(f.len(), 97);
    }
    for f in &frames[3..8] {
        assert_eq!(*f, pad(&[0, 8, 0x07, 0x00], 97));
    }
    // 7e paquet de suite : octets 645..698, les 3 derniers à l'index 4 + 51.
    let mut last = pad(&[0, 8, 0x07, 0x00], 97);
    last[55..58].copy_from_slice(&[4, 5, 6]);
    assert_eq!(frames[8], last);
    assert_eq!(frames[9], pad(&[0, 8, 0x05, 0x01, 0x00], 97));
}

#[test]
fn xt_colors_fill_1224_bytes_in_four_packets() {
    let mut replies = open_replies(2);
    replies.extend([ack(0x0D), ack(0x06), ack(0x07), ack(0x07), ack(0x07), ack(0x05)]);
    let (dev, sent) = open_dev(0x0C2A, replies);
    let mut dev = dev.unwrap();
    dev.set_colors(&[Color::new(0xAA, 0xBB, 0xCC); 204]).unwrap();
    let log = sent.lock();
    let frames = &log[6..];
    assert_eq!(frames.len(), 6);
    assert!(frames.iter().all(|f| f.len() == 385));
    // 1224 + 2 = 1226 = 0x04CA ; 375 octets de données dans le 1er paquet (index 10..384).
    assert_eq!(frames[1][..13], [0, 8, 0x06, 0x00, 0xCA, 0x04, 0, 0, 0x12, 0x00, 0xAA, 0xBB, 0xCC]);
    assert_eq!(frames[1][384], 0xCC); // octet de données 374 = dernier octet du 1er paquet
    // Suite 1 : octets 375..755 ; les couleurs s'arrêtent à l'octet 611 (index 4 + 236), le reste est à zéro.
    assert_eq!(frames[2][..7], [0, 8, 0x07, 0x00, 0xAA, 0xBB, 0xCC]);
    assert_eq!(frames[2][240], 0xCC);
    assert_eq!(frames[2][241], 0x00);
    assert_eq!(frames[3][4..], vec![0u8; 381]);
    assert_eq!(frames[4][..5], [0, 8, 0x07, 0x00, 0x00]);
    assert_eq!(frames[5], pad(&[0, 8, 0x05, 0x01, 0x00], 385));
}

#[test]
fn set_colors_validates_length_and_fan_duty_is_refused() {
    let (dev, sent) = open_dev(0x0C1C, open_replies(2));
    let mut dev = dev.unwrap();
    assert!(dev.set_colors(&[Color::BLACK; 10]).is_err());
    assert!(dev.set_colors(&[Color::BLACK; 234]).is_err());
    let e = dev.set_fan_duty(0, 50).unwrap_err().to_string();
    assert!(e.contains("INCONNU"));
    assert_eq!(sent.lock().len(), 6); // rien d'envoyé après l'ouverture
}

#[test]
fn colors_report_a_device_that_stops_answering() {
    let mut replies = open_replies(2);
    replies.push(ack(0x0D)); // puis plus rien
    let (dev, _) = open_dev(0x0C1C, replies);
    assert!(dev.unwrap().set_colors(&[Color::BLACK; 233]).is_err());
}

#[test]
fn release_returns_to_hardware_mode() {
    let mut replies = open_replies(2);
    replies.push(ack(0x01));
    let (dev, sent) = open_dev(0x0C1C, replies);
    dev.unwrap().release();
    assert_eq!(sent.lock()[6], pad(&[0, 8, 0x01, 0x03, 0x00, 0x01], 97));
}
