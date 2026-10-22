//! Claviers G815 (rapports fixes) et G915 / G915 TKL filaires (0x8071 + 0x8081) : couleur par touche en rapports longs 0x11 (20 octets).
//! Source : docs/rgb-protocols/analyse-logitech-ghub.md §3.2 A (codes de LED = usage HID - 3), §3.3 (trames), §3.4 (prise et restitution du contrôle), §7 (sécurité).
//!
//! Trames (device 0xFF) : commit `11 FF <8081> 7F`, « little » `1F` (4 x [code R G B], fin = FF), « big » `6F` (R G B puis jusqu'à 13 codes, fin = FF).
//! G815 : index fixes 0x08 (4522) / 0x0F (8071) / 0x10 (8081) ; G915 : index lus par IRoot (toujours interroger, jamais codés en dur).
//!
//! Écarts volontaires avec la note (sécurité, §7) : l'octet persist de SET_EFFECT reste à 0 (la note met 1 dans l'amorçage OpenRGB = écriture non volatile) ;
//! aucune commande 0x8071 NV_CONFIG / PWR_CONFIG / SHUTDOWN. SetRGBPowerMode (0x8071 fn 0x80, octet `mode` INCONNU) n'est pas envoyé.
//! G915 TKL : on garde le mode embarqué (aucune bascule host mode 0x8100/0x8101), comme l'impose la note.
//! INCONNU : table des touches hors bloc principal (G-keys, logo, multimédia : numérotation propre à la carte), présence des touches ISO 47 / 97
//! sur un modèle donné (une touche absente est envoyée telle quelle). Le G815 n'a pas de retour documenté par SW_CONTROL : `11 FF 11 1A 01`
//! (mode embarqué, d'après la note) est envoyé à la fermeture.
//! À VALIDER sur matériel : tout (aucune de ces séquences n'a tourné sur du vrai matériel), surtout l'amorçage sans persist et le retour au mode matériel.

use super::{exchange, frame, lookup, Hid, NativeRgb, REPORT_LONG};
use crate::core::Color;
use anyhow::{bail, Result};

const FEATURE_ONBOARD_KEYS: u16 = 0x4522;
const FEATURE_RGB_EFFECTS: u16 = 0x8071;
const FEATURE_PER_KEY_V2: u16 = 0x8081;
const FEATURE_PROFILE_MGMT: u16 = 0x8101;
const FEATURE_ONBOARD: u16 = 0x8100;

const FN_HANDSHAKE_1: u8 = 0x3E;
const FN_HANDSHAKE_2: u8 = 0x1E;
const FN_SET_EFFECT: u8 = 0x1E;
const FN_SW_CONTROL: u8 = 0x5E;
const FN_LITTLE: u8 = 0x1F;
const FN_BIG: u8 = 0x6F;
const FN_COMMIT: u8 = 0x7F;
const FN_PROFILE_MODE: u8 = 0x6E;
const FN_ONBOARD_MODE: u8 = 0x1E;
/// SW_CONTROL `[1, mode, flags]` : G815 / G915 utilisent mode 3, flags 7 (note §3.4) ; libération `[1, 0, 0]`.
const SW_CLAIM: [u8; 3] = [0x01, 0x03, 0x07];
const SW_RELEASE: [u8; 3] = [0x01, 0x00, 0x00];

const G815_IDX_4522: u8 = 0x08;
const G815_IDX_EFFECTS: u8 = 0x0F;
const G815_IDX_PER_KEY: u8 = 0x10;
/// g810-led, G815 : `11 FF 11 1A <1|2>` = mode embarqué (1) / logiciel (2).
const G815_IDX_ONBOARD_MODE: u8 = 0x11;
const G815_FN_ONBOARD_MODE: u8 = 0x1A;

/// Touche amorce : code brut 0x29 envoyé tel quel, couleur noire (note §3.3).
const PRIMER: [u8; 4] = [0x29, 0, 0, 0];
const LITTLE_KEYS: usize = 4;
const BIG_KEYS: usize = 13;
const END: u8 = 0xFF;

/// Codes de LED du bloc principal (usage HID - 3, 1..=98) puis modificateurs (usage 0xE0..0xE7 - 0x78 = 104..=111).
/// Le TKL n'a pas le pavé numérique (80..=96). Les positions 99..=103 ne sont équipées sur aucune carte connue : exclues.
pub(super) fn codes(tkl: bool) -> Vec<u8> {
    (1u8..=98).filter(|c| !(tkl && (80..=96).contains(c))).chain(104..=111).collect()
}

struct Keyboard {
    hid: Box<dyn Hid>,
    per_key: u8,
    codes: Vec<u8>,
    /// Trames écrites à la fermeture pour rendre la main au mode matériel.
    release: Vec<Vec<u8>>,
}

fn fn_frame(feature: u8, function: u8, params: &[u8]) -> Vec<u8> {
    let mut f = frame(REPORT_LONG, feature, function);
    f[4..4 + params.len()].copy_from_slice(params);
    f
}

/// Échange chaque trame dans l'ordre (écriture puis lecture de l'accusé).
fn run(hid: &dyn Hid, frames: &[Vec<u8>]) -> Result<()> {
    frames.iter().try_for_each(|f| exchange(hid, f).map(drop))
}

/// Amorçage commun : touche amorce noire puis commit.
fn prime(hid: &dyn Hid, per_key: u8) -> Result<()> {
    run(hid, &[fn_frame(per_key, FN_LITTLE, &PRIMER), frame(REPORT_LONG, per_key, FN_COMMIT)])
}

pub(super) fn open_g815(hid: Box<dyn Hid>) -> Result<Box<dyn NativeRgb>> {
    // Étapes 1 à 4 de la note ; persist (octet 0x10 de la trame) laissé à 0 : clusters 0 puis 1 à l'effet 0 (off), volatils.
    run(
        hid.as_ref(),
        &[
            frame(REPORT_LONG, G815_IDX_4522, FN_HANDSHAKE_1),
            frame(REPORT_LONG, G815_IDX_4522, FN_HANDSHAKE_2),
            fn_frame(G815_IDX_EFFECTS, FN_SET_EFFECT, &[0x00]),
            fn_frame(G815_IDX_EFFECTS, FN_SET_EFFECT, &[0x01]),
        ],
    )?;
    prime(hid.as_ref(), G815_IDX_PER_KEY)?;
    Ok(Box::new(Keyboard {
        hid,
        per_key: G815_IDX_PER_KEY,
        codes: codes(false),
        release: vec![fn_frame(G815_IDX_ONBOARD_MODE, G815_FN_ONBOARD_MODE, &[0x01])],
    }))
}

pub(super) fn open_g915(hid: Box<dyn Hid>, tkl: bool) -> Result<Box<dyn NativeRgb>> {
    let h = hid.as_ref();
    let find = |id: u16| lookup(h, REPORT_LONG, id);
    let onboard_keys = find(FEATURE_ONBOARD_KEYS)?;
    let Some(effects) = find(FEATURE_RGB_EFFECTS)? else { bail!("Logitech : feature {FEATURE_RGB_EFFECTS:#06x} absente de cet appareil") };
    let Some(per_key) = find(FEATURE_PER_KEY_V2)? else { bail!("Logitech : feature {FEATURE_PER_KEY_V2:#06x} absente de cet appareil") };
    // Bascule host mode / retour embarqué : (trame de bascule, trame de retour). Jamais sur le TKL.
    let host = if tkl {
        None
    } else if let Some(i) = find(FEATURE_PROFILE_MGMT)? {
        Some((fn_frame(i, FN_PROFILE_MODE, &[0x05]), fn_frame(i, FN_PROFILE_MODE, &[0x03])))
    } else {
        find(FEATURE_ONBOARD)?.map(|i| (fn_frame(i, FN_ONBOARD_MODE, &[0x02]), fn_frame(i, FN_ONBOARD_MODE, &[0x01])))
    };

    run(h, &[fn_frame(effects, FN_SW_CONTROL, &SW_CLAIM)])?;
    // Libération prête dès que le contrôle est pris : elle doit partir même si la suite de l'ouverture échoue.
    let mut release = vec![fn_frame(effects, FN_SW_CONTROL, &SW_RELEASE)];
    release.extend(host.iter().map(|(_, back)| back.clone()));
    let mut kb = Keyboard { hid, per_key, codes: codes(tkl), release };

    let mut init = Vec::new();
    if let Some(i) = onboard_keys {
        init.push(frame(REPORT_LONG, i, FN_HANDSHAKE_1));
        init.push(frame(REPORT_LONG, i, FN_HANDSHAKE_2));
    }
    init.extend(host.map(|(on, _)| on));
    init.push(fn_frame(effects, FN_SET_EFFECT, &[0x00]));
    init.push(fn_frame(effects, FN_SET_EFFECT, &[0x01]));
    let started = run(kb.hid.as_ref(), &init).and_then(|()| prime(kb.hid.as_ref(), per_key));
    if let Err(e) = started {
        kb.release();
        return Err(e);
    }
    Ok(Box::new(kb))
}

/// Trames de couleur : couleurs distinctes avec ≥ 2 touches en « big » (13 codes max), touches seules en « little » (4 max).
fn color_frames(per_key: u8, codes: &[u8], colors: &[Color]) -> Vec<Vec<u8>> {
    let mut groups: Vec<(Color, Vec<u8>)> = Vec::new();
    for (i, &code) in codes.iter().enumerate() {
        let c = colors.get(i).copied().unwrap_or(Color::BLACK);
        match groups.iter_mut().find(|(g, _)| *g == c) {
            Some((_, keys)) => keys.push(code),
            None => groups.push((c, vec![code])),
        }
    }
    let mut frames = Vec::new();
    let mut singles: Vec<(u8, Color)> = Vec::new();
    for (c, keys) in &groups {
        if let [code] = keys[..] {
            singles.push((code, *c));
            continue;
        }
        for chunk in keys.chunks(BIG_KEYS) {
            let mut f = fn_frame(per_key, FN_BIG, &[c.r, c.g, c.b]);
            f[7..7 + chunk.len()].copy_from_slice(chunk);
            if chunk.len() < BIG_KEYS {
                f[7 + chunk.len()] = END;
            }
            frames.push(f);
        }
    }
    for chunk in singles.chunks(LITTLE_KEYS) {
        let mut f = frame(REPORT_LONG, per_key, FN_LITTLE);
        for (i, (code, c)) in chunk.iter().enumerate() {
            f[4 + 4 * i..8 + 4 * i].copy_from_slice(&[*code, c.r, c.g, c.b]);
        }
        if chunk.len() < LITTLE_KEYS {
            f[4 + 4 * chunk.len()] = END;
        }
        frames.push(f);
    }
    frames
}

impl NativeRgb for Keyboard {
    fn set_colors(&mut self, colors: &[Color]) -> Result<()> {
        if colors.len() > self.codes.len() {
            bail!("Logitech : {} couleurs pour {} touches", colors.len(), self.codes.len());
        }
        for f in color_frames(self.per_key, &self.codes, colors) {
            self.hid.write(&f)?;
        }
        exchange(self.hid.as_ref(), &frame(REPORT_LONG, self.per_key, FN_COMMIT))?;
        Ok(())
    }

    fn release(&mut self) {
        for f in &self.release {
            if let Err(e) = self.hid.write(f) {
                log::warn!("Logitech : retour au mode matériel impossible : {e}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::MockHid;
    use super::*;

    type Sent = std::sync::Arc<parking_lot::Mutex<Vec<Vec<u8>>>>;

    /// Rapport 20 octets `11 FF idx fn`.
    fn ack(idx: u8, f: u8) -> Vec<u8> {
        let mut r = vec![0u8; 20];
        r[..4].copy_from_slice(&[0x11, 0xFF, idx, f]);
        r
    }
    fn iroot(index: u8) -> Vec<u8> {
        let mut r = ack(0x00, 0x07);
        r[4] = index;
        r
    }
    /// Trame 20 octets attendue : `bytes` puis zéros.
    fn want(bytes: &[u8]) -> Vec<u8> {
        let mut v = bytes.to_vec();
        v.resize(20, 0);
        v
    }

    fn g815() -> (Box<dyn NativeRgb>, Sent) {
        let replies = vec![ack(8, 0x3E), ack(8, 0x1E), ack(0x0F, 0x1E), ack(0x0F, 0x1E), ack(0x10, 0x1F), ack(0x10, 0x7F)];
        let (mock, sent, _) = MockHid::new(replies);
        (open_g815(Box::new(mock)).expect("open"), sent)
    }

    #[test]
    fn g815_init_frames_match_the_note_with_persist_left_at_zero() {
        let (_d, sent) = g815();
        let s = sent.lock();
        let expected: [Vec<u8>; 6] = [
            want(&[0x11, 0xFF, 0x08, 0x3E]),
            want(&[0x11, 0xFF, 0x08, 0x1E]),
            want(&[0x11, 0xFF, 0x0F, 0x1E]),
            want(&[0x11, 0xFF, 0x0F, 0x1E, 0x01]),
            want(&[0x11, 0xFF, 0x10, 0x1F, 0x29, 0x00, 0x00, 0x00]),
            want(&[0x11, 0xFF, 0x10, 0x7F]),
        ];
        assert_eq!(*s, expected);
        assert!(s.iter().all(|f| f[16] == 0), "persist (octet 0x10) doit rester à 0");
    }

    #[test]
    fn g815_release_returns_to_onboard_mode() {
        let (mut d, sent) = g815();
        sent.lock().clear();
        d.release();
        assert_eq!(*sent.lock(), [want(&[0x11, 0xFF, 0x11, 0x1A, 0x01])]);
    }

    #[test]
    fn set_colors_groups_big_and_little_frames_by_hand() {
        let (mock, sent, _) = MockHid::new(vec![ack(0x10, 0x7F)]);
        let mut d = Keyboard { hid: Box::new(mock), per_key: 0x10, codes: codes(false), release: vec![] };
        let mut c = vec![Color::BLACK; 106];
        c[0] = Color::new(255, 0, 0); // code 1 (A)
        c[1] = Color::new(255, 0, 0); // code 2 (B)
        c[37] = Color::new(0, 255, 0); // code 38 (Échap = usage 0x29 - 3), seule de sa couleur
        d.set_colors(&c).expect("set");
        let s = sent.lock();
        // 1 big rouge + 8 big noirs (103 touches = 7 x 13 + 12) + 1 little + 1 commit.
        assert_eq!(s.len(), 1 + 8 + 1 + 1);
        assert_eq!(s[0], want(&[0x11, 0xFF, 0x10, 0x6F, 0xFF, 0x00, 0x00, 0x01, 0x02, 0xFF]));
        assert_eq!(s[1], want(&[0x11, 0xFF, 0x10, 0x6F, 0, 0, 0, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]));
        assert_eq!(s[8], want(&[0x11, 0xFF, 0x10, 0x6F, 0, 0, 0, 95, 96, 97, 98, 104, 105, 106, 107, 108, 109, 110, 111, 0xFF]));
        assert_eq!(s[9], want(&[0x11, 0xFF, 0x10, 0x1F, 0x26, 0x00, 0xFF, 0x00, 0xFF]));
        assert_eq!(s[10], want(&[0x11, 0xFF, 0x10, 0x7F]));
    }

    #[test]
    fn five_single_keys_use_two_little_frames() {
        let colors: Vec<Color> = (0..5).map(|i| Color::new(10 + i, 20 + i, 30 + i)).collect();
        let f = color_frames(0x10, &codes(false)[..5], &colors);
        assert_eq!(f.len(), 2);
        assert_eq!(f[0], want(&[0x11, 0xFF, 0x10, 0x1F, 1, 10, 20, 30, 2, 11, 21, 31, 3, 12, 22, 32, 4, 13, 23, 33]));
        assert_eq!(f[1], want(&[0x11, 0xFF, 0x10, 0x1F, 5, 14, 24, 34, 0xFF]));
    }

    #[test]
    fn key_codes_follow_usage_minus_three() {
        let c = codes(false);
        assert_eq!(c.len(), 106);
        assert_eq!(c[0], 1); // A = usage 0x04 - 3
        assert_eq!(c[37], 38); // Échap = 0x29 - 3
        assert!(c.contains(&80) && c.contains(&96)); // pavé numérique
        assert_eq!(&c[98..], &[104, 105, 106, 107, 108, 109, 110, 111]); // 0xE0..0xE7 - 0x78
        let t = codes(true);
        assert_eq!(t.len(), 89);
        assert!(!t.contains(&80) && !t.contains(&96) && t.contains(&79) && t.contains(&97));
    }

    #[test]
    fn too_many_colors_is_rejected_and_nothing_more_is_sent() {
        let (mut d, sent) = g815();
        sent.lock().clear();
        assert!(d.set_colors(&vec![Color::BLACK; 107]).is_err());
        assert!(sent.lock().is_empty());
    }

    fn g915_replies(tail: Vec<Vec<u8>>) -> Vec<Vec<u8>> {
        let mut r = vec![iroot(0x0E), iroot(0x09), iroot(0x0A)];
        r.extend(tail);
        r
    }

    #[test]
    fn g915_claims_with_profile_management_and_releases() {
        let mut replies = g915_replies(vec![iroot(0x0D)]); // 8101 présente
        replies.extend([ack(0x09, 0x5E), ack(0x0E, 0x3E), ack(0x0E, 0x1E), ack(0x0D, 0x6E), ack(0x09, 0x1E), ack(0x09, 0x1E), ack(0x0A, 0x1F), ack(0x0A, 0x7F)]);
        let (mock, sent, _) = MockHid::new(replies);
        let mut d = open_g915(Box::new(mock), false).expect("open");
        let expected: [Vec<u8>; 12] = [
            want(&[0x11, 0xFF, 0x00, 0x07, 0x45, 0x22]),
            want(&[0x11, 0xFF, 0x00, 0x07, 0x80, 0x71]),
            want(&[0x11, 0xFF, 0x00, 0x07, 0x80, 0x81]),
            want(&[0x11, 0xFF, 0x00, 0x07, 0x81, 0x01]),
            want(&[0x11, 0xFF, 0x09, 0x5E, 0x01, 0x03, 0x07]),
            want(&[0x11, 0xFF, 0x0E, 0x3E]),
            want(&[0x11, 0xFF, 0x0E, 0x1E]),
            want(&[0x11, 0xFF, 0x0D, 0x6E, 0x05]),
            want(&[0x11, 0xFF, 0x09, 0x1E]),
            want(&[0x11, 0xFF, 0x09, 0x1E, 0x01]),
            want(&[0x11, 0xFF, 0x0A, 0x1F, 0x29, 0x00, 0x00, 0x00]),
            want(&[0x11, 0xFF, 0x0A, 0x7F]),
        ];
        assert_eq!(*sent.lock(), expected);
        sent.lock().clear();
        d.release();
        assert_eq!(*sent.lock(), [want(&[0x11, 0xFF, 0x09, 0x5E, 0x01, 0x00, 0x00]), want(&[0x11, 0xFF, 0x0D, 0x6E, 0x03])]);
    }

    #[test]
    fn g915_falls_back_to_onboard_feature() {
        let mut replies = g915_replies(vec![iroot(0), iroot(0x0C)]); // 8101 absente, 8100 présente
        replies.extend([ack(0x09, 0x5E), ack(0x0E, 0x3E), ack(0x0E, 0x1E), ack(0x0C, 0x1E), ack(0x09, 0x1E), ack(0x09, 0x1E), ack(0x0A, 0x1F), ack(0x0A, 0x7F)]);
        let (mock, sent, _) = MockHid::new(replies);
        let mut d = open_g915(Box::new(mock), false).expect("open");
        assert_eq!(sent.lock()[3 + 2 + 1 + 2], want(&[0x11, 0xFF, 0x0C, 0x1E, 0x02]));
        sent.lock().clear();
        d.release();
        assert_eq!(sent.lock()[1], want(&[0x11, 0xFF, 0x0C, 0x1E, 0x01]));
    }

    #[test]
    fn g915_tkl_stays_in_onboard_mode() {
        let mut replies = g915_replies(vec![]);
        replies.extend([ack(0x09, 0x5E), ack(0x0E, 0x3E), ack(0x0E, 0x1E), ack(0x09, 0x1E), ack(0x09, 0x1E), ack(0x0A, 0x1F), ack(0x0A, 0x7F)]);
        let (mock, sent, _) = MockHid::new(replies);
        let mut d = open_g915(Box::new(mock), true).expect("open");
        // 3 IRoot + SW_CONTROL + 2 poignées + 2 clusters + amorce + commit : aucune recherche 8101 / 8100, aucune bascule.
        assert_eq!(sent.lock().len(), 3 + 1 + 2 + 2 + 2);
        d.release();
        assert_eq!(sent.lock().len(), 3 + 1 + 2 + 2 + 2 + 1);
        assert_eq!(sent.lock()[10], want(&[0x11, 0xFF, 0x09, 0x5E, 0x01, 0x00, 0x00]));
    }

    #[test]
    fn g915_refuses_missing_features_error_and_silence() {
        let (m, _, _) = MockHid::new(vec![iroot(0x0E), iroot(0)]);
        assert!(open_g915(Box::new(m), false).is_err()); // 8071 absente
        let (m, _, _) = MockHid::new(vec![iroot(0x0E), iroot(0x09), iroot(0)]);
        assert!(open_g915(Box::new(m), false).is_err()); // 8081 absente
        let (m, _, _) = MockHid::new(vec![]);
        assert!(open_g915(Box::new(m), false).is_err());
        let mut err = iroot(0);
        err[2] = 0xFF;
        let (m, _, _) = MockHid::new(vec![err]);
        assert!(open_g915(Box::new(m), true).is_err());
    }

    #[test]
    fn g915_failed_init_gives_the_control_back() {
        // SW_CONTROL accusé, puis silence : l'ouverture échoue et la libération part quand même.
        let mut replies = g915_replies(vec![]);
        replies.push(ack(0x09, 0x5E));
        let (mock, sent, _) = MockHid::new(replies);
        assert!(open_g915(Box::new(mock), true).is_err());
        assert_eq!(*sent.lock().last().expect("trame"), want(&[0x11, 0xFF, 0x09, 0x5E, 0x01, 0x00, 0x00]));
    }
}
