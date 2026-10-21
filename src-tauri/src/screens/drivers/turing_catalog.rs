//! Catalogue de détection des écrans USB-C « série » (Turing, XuanFang, UsbPCMonitor, Kipye).
//!
//! Source : wiki turing-smart-screen-python « Hardware revisions » (identifiants série
//! USB, ex. `USB35INCHIPS`) et ses pages de protocole par révision. Documentation
//! seulement, aucun code repris. Un port hors de ce catalogue n'est JAMAIS ouvert.

/// Révision de protocole (nommage du wiki).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Revision {
    /// Turing 3.5", UsbMonitor 3.5/5/7" : commandes de 6 octets, RGB565 petit-boutiste.
    A,
    /// XuanFang 3.5" (B et flagship) : paquets de 10 octets, RGB565 gros-boutiste.
    B,
    /// Turing 2.1" / 5" / 8.8" : blocs de 250 octets, BGRA.
    C,
    /// Kipye Qiye 3.5".
    D,
}

/// Résultat de la classification d'un port USB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Detection {
    /// Écran reconnu sans ambiguïté.
    Known(Entry),
    /// Écran de révision C en veille : il faut le « réveiller » (ouvrir le port) avant usage.
    Sleeping(Entry),
    /// VID/PID partagé par plusieurs révisions : une surcharge explicite est requise.
    Ambiguous(&'static [Revision]),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub revision: Revision,
    pub vendor: &'static str,
    pub model: &'static str,
    /// Résolution portrait (largeur, hauteur) si le catalogue la donne ; `None` = à fournir.
    pub portrait: Option<(u32, u32)>,
}

const fn e(
    revision: Revision,
    vendor: &'static str,
    model: &'static str,
    portrait: Option<(u32, u32)>,
) -> Entry {
    Entry { revision, vendor, model, portrait }
}

/// Résolutions portrait (wiki + constantes de révision C : 2.1" 480x480, 5" 480x800, 8.8" 480x1920).
pub const SIZE_35: (u32, u32) = (320, 480);
pub const SIZE_21: (u32, u32) = (480, 480);
pub const SIZE_5: (u32, u32) = (480, 800);
pub const SIZE_88: (u32, u32) = (480, 1920);

/// Classe un port par numéro de série USB puis VID/PID. `None` = hors catalogue (à ignorer).
pub fn classify(vid: u16, pid: u16, serial: Option<&str>) -> Option<Detection> {
    use Detection::{Ambiguous, Known, Sleeping};
    use Revision::{A, B, C, D};
    // Le numéro de série est prioritaire (il lève l'ambiguïté 1a86:5722 entre A et B).
    match serial {
        Some("USB35INCHIPS") | Some("USB35INCHIPSV2") => {
            return Some(Known(e(A, "Turing", "Turing Smart Screen 3.5\"", Some(SIZE_35))));
        }
        Some("2017-2-25") => return Some(Known(e(B, "XuanFang", "XuanFang 3.5\"", Some(SIZE_35)))),
        Some("20080411") => return Some(Known(e(C, "Turing", "Turing Smart Screen (taille à confirmer)", None))),
        Some("USB7INCH") => return Some(Sleeping(e(C, "Turing", "Turing Smart Screen 5\" (veille)", Some(SIZE_5)))),
        Some("CT21INCH") => return Some(Sleeping(e(C, "Turing", "Turing Smart Screen 2.1\" (veille)", Some(SIZE_21)))),
        Some("CT88INCH") => return Some(Sleeping(e(C, "Turing", "Turing Smart Screen 8.8\" (veille)", Some(SIZE_88)))),
        _ => {}
    }
    match (vid, pid) {
        (0x1a86, 0x5722) => Some(Ambiguous(&[A, B])),
        (0x1a86, 0xca21) => Some(Sleeping(e(C, "Turing", "Turing Smart Screen (veille)", None))),
        (0x0525, 0xa4a7) | (0x1d6b, 0x0121) | (0x1d6b, 0x0106) => {
            Some(Known(e(C, "Turing", "Turing Smart Screen (taille à confirmer)", None)))
        }
        (0x454d, 0x4e41) => Some(Known(e(D, "Kipye", "Kipye Qiye 3.5\"", Some(SIZE_35)))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial_ids_from_wiki() {
        let Some(Detection::Known(a)) = classify(0x1a86, 0x5722, Some("USB35INCHIPS")) else { panic!() };
        assert_eq!(a.revision, Revision::A);
        let Some(Detection::Known(b)) = classify(0x1a86, 0x5722, Some("2017-2-25")) else { panic!() };
        assert_eq!(b.revision, Revision::B);
    }

    #[test]
    fn shared_vid_pid_is_ambiguous_and_unknown_is_ignored() {
        assert_eq!(classify(0x1a86, 0x5722, None), Some(Detection::Ambiguous(&[Revision::A, Revision::B])));
        assert_eq!(classify(0x1234, 0x5678, Some("x")), None);
        assert_eq!(classify(0x0403, 0x6001, None), None);
    }

    #[test]
    fn rev_c_sleeping_vs_awake_and_rev_d() {
        assert!(matches!(classify(0, 0, Some("CT88INCH")), Some(Detection::Sleeping(e)) if e.portrait == Some(SIZE_88)));
        assert!(matches!(classify(0x1a86, 0xca21, None), Some(Detection::Sleeping(_))));
        assert!(matches!(classify(0x0525, 0xa4a7, None), Some(Detection::Known(e)) if e.revision == Revision::C));
        assert!(matches!(classify(0x454d, 0x4e41, None), Some(Detection::Known(e)) if e.revision == Revision::D));
    }
}
