//! Catalogue des écrans Thermalright (TRCC) : VID/PID -> protocole, noms de
//! produits, résolutions (tables FBL / PM).
//!
//! Sources (documentation publique TRCC-Linux, GPLv3, données seulement) :
//! - `doc/REFERENCE_TECHNICAL.md` : « Supported Devices », « FBL Detection »,
//!   « PM -> FBL Mapping », « Bulk USB Protocol », « LY Bulk Protocol ».
//! - `doc/REFERENCE_DEVICES.md` : « USB Interfaces » et listes de produits.
//! - `src/trcc/core/protocol.py` (tables `FBL_PROFILES`, `_FBL_*_BY_PM`,
//!   `ENCODE_ROTATIONS`) pour les drapeaux jpeg / big-endian / rotation que la
//!   doc ne détaille pas : seules les VALEURS des tables sont reprises.
//!
//! Tout FBL / PM absent des tables donne `None` : on ne devine jamais une résolution.

use crate::screens::driver::Protocol;

/// Variante précise du protocole (un fichier de pilote par famille).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// USB Mass Storage « USBLCD » (SCSI pass-through).
    Scsi,
    /// HID type 2 (« H », magic DA DB DC DD).
    HidType2,
    /// HID type 3 (protocole F5 : 0416:5406, 0418:5303/5304).
    HidF5,
    /// Bulk USB brut 87AD:70DB (magic 12 34 56 78).
    Bulk,
    /// LY (0416:5408).
    Ly,
    /// LY1 (0416:5409).
    Ly1,
}

impl Kind {
    pub fn protocol(self) -> Protocol {
        match self {
            Kind::Scsi => Protocol::ThermalrightScsi,
            Kind::HidType2 | Kind::HidF5 => Protocol::ThermalrightHid,
            Kind::Bulk => Protocol::ThermalrightBulk,
            Kind::Ly | Kind::Ly1 => Protocol::ThermalrightLy,
        }
    }
}

/// Une ligne du catalogue.
#[derive(Debug, Clone, Copy)]
pub struct DeviceEntry {
    pub vid: u16,
    pub pid: u16,
    pub kind: Kind,
    /// Produits connus derrière cet identifiant USB (le handshake les distingue).
    pub products: &'static [&'static str],
    /// Résolution à supposer sans handshake exploitable ; `(0, 0)` = INCONNU
    /// (un même VID:PID couvre plusieurs écrans, seul le handshake tranche).
    pub default_resolution: (u32, u32),
}

/// LA table (REFERENCE_DEVICES.md « USB Interfaces » + REFERENCE_TECHNICAL.md).
pub const DEVICES: &[DeviceEntry] = &[
    DeviceEntry { vid: 0x87CD, pid: 0x70DB, kind: Kind::Scsi,
        products: &["Frozen Horizon Pro", "Frozen Magic Pro", "Frozen Vision V2"], default_resolution: (320, 320) },
    DeviceEntry { vid: 0x0402, pid: 0x3922, kind: Kind::Scsi,
        products: &["Frozen Warframe", "Frozen Warframe 240", "Elite Vision 360 ARGB"], default_resolution: (320, 320) },
    DeviceEntry { vid: 0x0416, pid: 0x5302, kind: Kind::HidType2,
        products: &["Frozen Warframe SE", "Frozen Warframe 360", "Trofeo Vision LCD", "Assassin Spirit 120 Vision ARGB"],
        default_resolution: (0, 0) },
    DeviceEntry { vid: 0x0416, pid: 0x5406, kind: Kind::HidF5,
        products: &["LC1", "LC2", "LC3", "LC5"], default_resolution: (320, 320) },
    DeviceEntry { vid: 0x0418, pid: 0x5303, kind: Kind::HidF5, products: &["TARAN ARMS"], default_resolution: (320, 320) },
    DeviceEntry { vid: 0x0418, pid: 0x5304, kind: Kind::HidF5, products: &["TARAN ARMS"], default_resolution: (320, 320) },
    DeviceEntry { vid: 0x87AD, pid: 0x70DB, kind: Kind::Bulk,
        products: &["GrandVision 360 AIO", "Mjolnir Vision 360", "Wonder Vision Pro 360", "Frozen Warframe Pro",
            "Hyper Vision 360", "Peerless Vision 360", "Phantom Spirit 120 Vision EVO", "Levita Vision"],
        default_resolution: (480, 480) },
    DeviceEntry { vid: 0x0416, pid: 0x5408, kind: Kind::Ly, products: &["Trofeo Vision 9.16 LCD"], default_resolution: (1920, 462) },
    DeviceEntry { vid: 0x0416, pid: 0x5409, kind: Kind::Ly1, products: &[], default_resolution: (0, 0) },
];

pub fn find(vid: u16, pid: u16) -> Option<&'static DeviceEntry> {
    DEVICES.iter().find(|d| d.vid == vid && d.pid == pid)
}

/// Nom affiché : produits connus joints par « / », sinon un libellé générique.
pub fn display_name(e: &DeviceEntry) -> String {
    if e.products.is_empty() {
        "Écran Thermalright".to_string()
    } else {
        e.products.join(" / ")
    }
}

/// Géométrie et encodage d'un écran, dérivés du FBL (ou du couple PM/SUB).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    pub width: u32,
    pub height: u32,
    pub jpeg: bool,
    pub big_endian: bool,
    /// Écran « portrait » monté en paysage : l'image est tournée avant envoi.
    pub rotate: bool,
    /// Largeur de ligne RGB565 du framebuffer si > à celle de l'image (0 = identique).
    pub wire_row_px: u32,
}

const fn p(width: u32, height: u32, jpeg: bool, big_endian: bool, rotate: bool) -> Profile {
    Profile { width, height, jpeg, big_endian, rotate, wire_row_px: 0 }
}

/// FBL -> profil (REFERENCE_TECHNICAL.md « FBL Detection » ; drapeaux : protocol.py `FBL_PROFILES`).
/// 192 et 224 sont partagés : la résolution vient de [`fbl_profile`] via le PM.
const FBL_PROFILES: &[(u8, Profile)] = &[
    (36, p(240, 240, false, false, false)),
    (37, p(240, 240, false, false, false)),
    (49, p(320, 240, false, true, true)),
    (50, p(320, 240, false, false, true)),
    (51, p(320, 240, false, false, true)),
    (52, p(320, 240, false, false, true)),
    (53, p(320, 240, false, false, true)),
    (54, p(360, 360, true, false, false)),
    (58, p(320, 240, false, false, true)),
    (59, Profile { wire_row_px: 176, ..p(640, 172, false, false, true) }),
    (60, p(320, 176, false, false, true)),
    (64, p(640, 480, false, false, true)),
    (72, p(480, 480, false, false, false)),
    (100, p(320, 320, false, true, false)),
    (101, p(320, 320, false, true, false)),
    (102, p(320, 320, false, true, false)),
    (114, p(1600, 720, true, false, true)),
    (128, p(1280, 480, true, false, true)),
    (129, p(480, 480, false, false, false)),
    (192, p(1920, 462, true, false, true)),
    (224, p(854, 480, true, false, true)),
];

/// PM -> FBL quand ils diffèrent (REFERENCE_TECHNICAL.md « PM -> FBL Mapping »).
const PM_TO_FBL: &[(u8, u8)] = &[
    (5, 50), (7, 64), (9, 224), (10, 224), (11, 224), (12, 224), (13, 224), (14, 64), (15, 224), (16, 224),
    (17, 224), (18, 224), (20, 224), (32, 100), (50, 50), (63, 114), (64, 114), (65, 192), (66, 192),
    (68, 192), (69, 192),
];

/// FBL 224 : le PM tranche la résolution.
const RES_224_BY_PM: &[(u8, (u32, u32))] = &[
    (9, (854, 480)), (10, (960, 540)), (11, (854, 480)), (12, (800, 480)), (13, (960, 320)),
    (15, (640, 172)), (16, (960, 540)), (17, (960, 320)), (18, (960, 320)), (20, (854, 480)),
];

/// FBL 192 : le PM tranche la résolution (PM 1 : cas SUB=49 de [`pm_to_fbl`]).
const RES_192_BY_PM: &[(u8, (u32, u32))] = &[
    (1, (1920, 462)), (65, (1920, 462)), (66, (1920, 462)), (68, (1280, 480)), (69, (1920, 440)),
];

/// PM (+ SUB) -> FBL. Défaut : PM = FBL.
pub fn pm_to_fbl(pm: u8, sub: u8) -> u8 {
    match (pm, sub) {
        (1, 48) => 114,
        (1, 49) => 192,
        _ => PM_TO_FBL.iter().find(|(k, _)| *k == pm).map_or(pm, |(_, f)| *f),
    }
}

/// Le couple (PM, SUB) est-il nommé par les tables ? (sinon, bulk/LY restent sur FBL 72).
fn in_pm_ladder(pm: u8, sub: u8) -> bool {
    matches!((pm, sub), (1, 48) | (1, 49)) || PM_TO_FBL.iter().any(|(k, _)| *k == pm)
}

/// Profil d'un FBL. `None` si FBL inconnu, ou FBL 192/224 avec un PM non catalogué.
/// `sub` : sur FBL 54 (360x360), SUB 4 passe en RGB565 (protocol.py `rgb565_subs`).
pub fn fbl_profile(fbl: u8, pm: u8, sub: u8) -> Option<Profile> {
    let mut prof = FBL_PROFILES.iter().find(|(k, _)| *k == fbl).map(|(_, v)| *v)?;
    let shared = match fbl {
        192 => Some(RES_192_BY_PM),
        224 => Some(RES_224_BY_PM),
        _ => None,
    };
    if let Some(table) = shared {
        let (w, h) = table.iter().find(|(k, _)| *k == pm).map(|(_, v)| *v)?;
        prof.width = w;
        prof.height = h;
    }
    if fbl == 54 && sub == 4 {
        prof.jpeg = false;
    }
    Some(prof)
}

/// Profil d'un périphérique « mode 2 » (bulk et LY) d'après PM/SUB du handshake
/// (REFERENCE_TECHNICAL.md : « a PM outside that table stays on FBL 72 (480x480) »).
/// JPEG pour tous les PM sauf 32 et 50 (RGB565 ; PM 50 en big-endian).
pub fn bulk_profile(pm: u8, sub: u8) -> Option<Profile> {
    let fbl = if in_pm_ladder(pm, sub) { pm_to_fbl(pm, sub) } else { 72 };
    let mut prof = fbl_profile(fbl, pm, sub)?;
    prof.jpeg = pm != 32 && pm != 50;
    prof.big_endian = prof.big_endian || pm == 50;
    Some(prof)
}

/// Rotation de montage de l'écran (base, inversé) avant rotation utilisateur.
/// Valeurs : protocole `ENCODE_ROTATIONS`, bras principal seulement.
// INCONNU : les bras dépendant du SUB (1280x480 SUB 2 -> 90, 1920x462/440 SUB 2-4 -> 0,
// 1600x720 SUB 3 -> 0, 960x540 SUB 5/7 -> 180, 960x320 SUB 5+ -> 180, 640x172 SUB 5 -> 90)
// ne sont PAS gérés : l'image peut être retournée sur ces variantes, à corriger sur matériel.
fn mount_rotation(w: u32, h: u32, jpeg: bool) -> (u16, bool) {
    match (w, h, jpeg) {
        (1600, 720, true) | (1920, 462, true) | (1920, 440, true) | (960, 320, true) => (180, true),
        (854, 480, true) | (800, 480, true) => (0, false),
        (320, 320, true) | (480, 480, true) | (320, 240, true) | (1280, 480, true) | (960, 540, true)
        | (640, 480, true) | (360, 360, true) | (640, 172, true) => (0, true),
        (240, 240, false) | (320, 320, false) | (360, 360, false) | (480, 480, false) => (0, true),
        (640, 172, false) => (270, true),
        _ => (90, true), // bras par défaut (320x240 RGB565, etc.)
    }
}

impl Profile {
    /// Angle horaire appliqué à l'image avant encodage, pour la rotation utilisateur `user` (0/90/180/270).
    /// Écran « rotate » : (base ∓ utilisateur) ; sinon rotation utilisateur seule (360 - user).
    pub fn wire_angle(&self, user: u16) -> u16 {
        let user = user % 360;
        if self.rotate {
            let (base, invert) = mount_rotation(self.width, self.height, self.jpeg);
            let signed = if invert { (360 - user) % 360 } else { user };
            (base + signed) % 360
        } else {
            (360 - user) % 360
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_has_unique_ids_and_all_nine_usb_ids() {
        assert_eq!(DEVICES.len(), 9);
        for (i, a) in DEVICES.iter().enumerate() {
            assert!(DEVICES[i + 1..].iter().all(|b| (a.vid, a.pid) != (b.vid, b.pid)), "doublon {:04x}:{:04x}", a.vid, a.pid);
        }
    }

    #[test]
    fn usb_ids_match_doc() {
        // REFERENCE_DEVICES.md « USB Interfaces ».
        let k = |v, p| find(v, p).map(|d| d.kind);
        assert_eq!(k(0x87CD, 0x70DB), Some(Kind::Scsi));
        assert_eq!(k(0x0402, 0x3922), Some(Kind::Scsi));
        assert_eq!(k(0x87AD, 0x70DB), Some(Kind::Bulk));
        assert_eq!(k(0x0416, 0x5302), Some(Kind::HidType2));
        assert_eq!(k(0x0416, 0x5406), Some(Kind::HidF5));
        assert_eq!(k(0x0418, 0x5303), Some(Kind::HidF5));
        assert_eq!(k(0x0418, 0x5304), Some(Kind::HidF5));
        assert_eq!(k(0x0416, 0x5408), Some(Kind::Ly));
        assert_eq!(k(0x0416, 0x5409), Some(Kind::Ly1));
        assert_eq!(k(0x0416, 0x8001), None); // LED : hors périmètre
        assert_eq!(Kind::Ly1.protocol(), Protocol::ThermalrightLy);
    }

    #[test]
    fn fbl_resolutions_match_doc() {
        // REFERENCE_TECHNICAL.md « FBL Detection ».
        let res = |f, pm| fbl_profile(f, pm, 0).map(|p| (p.width, p.height));
        assert_eq!(res(36, 36), Some((240, 240)));
        assert_eq!(res(50, 50), Some((320, 240)));
        assert_eq!(res(54, 54), Some((360, 360)));
        assert_eq!(res(59, 59), Some((640, 172)));
        assert_eq!(res(60, 60), Some((320, 176)));
        assert_eq!(res(64, 64), Some((640, 480)));
        assert_eq!(res(72, 72), Some((480, 480)));
        assert_eq!(res(100, 100), Some((320, 320)));
        assert_eq!(res(102, 102), Some((320, 320)));
        assert_eq!(res(114, 114), Some((1600, 720)));
        assert_eq!(res(128, 128), Some((1280, 480)));
        assert_eq!(res(129, 129), Some((480, 480)));
        assert_eq!(res(55, 55), None); // jamais deviné
    }

    #[test]
    fn shared_fbl_192_224_use_pm() {
        // « PM disambiguates: 68->1280x480, 69->1920x440 » ; 224 : 9/11/20 854x480, 10/16 960x540,
        // 12 800x480, 13/17/18 960x320, 15 640x172.
        let res = |f, pm| fbl_profile(f, pm, 0).map(|p| (p.width, p.height));
        assert_eq!(res(192, 68), Some((1280, 480)));
        assert_eq!(res(192, 69), Some((1920, 440)));
        assert_eq!(res(192, 65), Some((1920, 462)));
        for pm in [9, 11, 20] {
            assert_eq!(res(224, pm), Some((854, 480)));
        }
        for pm in [10, 16] {
            assert_eq!(res(224, pm), Some((960, 540)));
        }
        assert_eq!(res(224, 12), Some((800, 480)));
        for pm in [13, 17, 18] {
            assert_eq!(res(224, pm), Some((960, 320)));
        }
        assert_eq!(res(224, 15), Some((640, 172)));
        assert_eq!(res(224, 99), None);
        assert_eq!(res(192, 99), None);
    }

    #[test]
    fn pm_to_fbl_matches_doc_table() {
        // REFERENCE_TECHNICAL.md « PM -> FBL Mapping ».
        for (pm, fbl) in [(5, 50), (7, 64), (9, 224), (14, 64), (32, 100), (50, 50), (63, 114), (64, 114), (65, 192), (66, 192), (68, 192), (69, 192)] {
            assert_eq!(pm_to_fbl(pm, 0), fbl, "PM {pm}");
        }
        assert_eq!(pm_to_fbl(1, 48), 114);
        assert_eq!(pm_to_fbl(1, 49), 192);
        assert_eq!(pm_to_fbl(3, 0), 3); // hors table : PM = FBL
    }

    #[test]
    fn bulk_profile_rules() {
        let b = |pm, sub| bulk_profile(pm, sub).map(|p| (p.width, p.height, p.jpeg, p.big_endian));
        assert_eq!(b(3, 0), Some((480, 480, true, false))); // hors table -> FBL 72, JPEG
        assert_eq!(b(32, 0), Some((320, 320, false, true))); // PM 32 : RGB565
        assert_eq!(b(50, 0), Some((320, 240, false, true))); // PM 50 : RGB565 big-endian
        assert_eq!(b(5, 0), Some((320, 240, true, false))); // PM 5 : JPEG
        assert_eq!(b(64, 0), Some((1600, 720, true, false)));
        assert_eq!(b(65, 4), Some((1920, 462, true, false)));
        assert_eq!(b(14, 0), Some((640, 480, true, false)));
        assert_eq!(b(15, 0), Some((640, 172, true, false)));
    }

    #[test]
    fn wire_angles() {
        let sq = fbl_profile(100, 100, 0).unwrap();
        assert_eq!([0, 90, 180, 270].map(|u| sq.wire_angle(u)), [0, 270, 180, 90]);
        let wf = fbl_profile(50, 50, 0).unwrap(); // 320x240 RGB565 : base 90
        assert_eq!([0, 90, 180, 270].map(|u| wf.wire_angle(u)), [90, 0, 270, 180]);
        let w854 = bulk_profile(9, 0).unwrap(); // 854x480 JPEG : base 0, non inversé
        assert_eq!([0, 90, 180, 270].map(|u| w854.wire_angle(u)), [0, 90, 180, 270]);
        let trofeo = bulk_profile(65, 0).unwrap(); // 1920x462 JPEG : base 180
        assert_eq!(trofeo.wire_angle(0), 180);
    }

    #[test]
    fn profile_flags() {
        assert_eq!(fbl_profile(54, 54, 4).map(|p| p.jpeg), Some(false));
        assert_eq!(fbl_profile(54, 54, 1).map(|p| p.jpeg), Some(true));
        assert_eq!(fbl_profile(59, 59, 0).map(|p| p.wire_row_px), Some(176));
        assert!(fbl_profile(49, 49, 0).is_some_and(|p| p.big_endian));
    }
}
