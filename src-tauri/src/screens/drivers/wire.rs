//! Image -> charge utile de fil, commune aux protocoles Thermalright :
//! rotation (montage + utilisateur), puis JPEG ou RGB565 selon le [`Profile`].

use super::catalog::Profile;
use super::encode;
use crate::screens::driver::Frame;
use anyhow::{bail, Result};

/// Plafond de taille d'un JPEG accepté par le firmware (protocol.py `max_frame_bytes` :
/// TRCC n'envoie jamais 450000 octets ou plus). On baisse la qualité jusqu'à passer sous le plafond.
pub const MAX_JPEG_BYTES: usize = 450_000;
const MIN_JPEG_QUALITY: u8 = 10;

/// Image encodée prête à être placée dans un en-tête de protocole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WireImage {
    pub data: Vec<u8>,
    /// Dimensions réelles de l'image encodée (après rotation).
    pub width: u32,
    pub height: u32,
    pub jpeg: bool,
}

/// Encode `frame` (taille exacte du profil) : rotation `profile.wire_angle(user_rotation)`, puis
/// JPEG (`quality` de départ) ou RGB565 (+ recentrage de ligne si `wire_row_px`).
pub fn encode_frame(profile: &Profile, frame: &Frame, user_rotation: u16, quality: u8) -> Result<WireImage> {
    if !frame.is_valid() {
        bail!("Image invalide : {} octets pour {}x{}.", frame.rgba.len(), frame.width, frame.height);
    }
    if (frame.width, frame.height) != (profile.width, profile.height) {
        bail!(
            "L'image fait {}x{} mais l'écran attend {}x{}.",
            frame.width, frame.height, profile.width, profile.height
        );
    }
    let (rgba, w, h) = encode::rotate_rgba(&frame.rgba, frame.width, frame.height, profile.wire_angle(user_rotation))?;
    if profile.jpeg {
        let mut q = quality.clamp(1, 100);
        loop {
            let data = encode::rgba_to_jpeg(&rgba, w, h, q)?;
            if data.len() < MAX_JPEG_BYTES {
                return Ok(WireImage { data, width: w, height: h, jpeg: true });
            }
            if q <= MIN_JPEG_QUALITY {
                bail!("Image JPEG trop lourde ({} octets, plafond {MAX_JPEG_BYTES}) même à qualité minimale.", data.len());
            }
            q = q.saturating_sub(5).max(MIN_JPEG_QUALITY);
        }
    }
    let mut data = encode::rgba_to_rgb565(&rgba, profile.big_endian);
    if profile.wire_row_px > w {
        data = repack_rows(&data, w, h, profile.wire_row_px);
    }
    Ok(WireImage { data, width: w, height: h, jpeg: false })
}

/// Recopie chaque ligne RGB565 de `w` pixels dans une ligne de `row_px` pixels, décalée de 4 octets
/// (REFERENCE : écran 640x172, tampon de 176 pixels par ligne, `protocol.py` `wire_row_px`).
fn repack_rows(data: &[u8], w: u32, h: u32, row_px: u32) -> Vec<u8> {
    let (src_row, dst_row) = (w as usize * 2, row_px as usize * 2);
    let offset = 4.min(dst_row - src_row);
    let mut out = vec![0u8; dst_row * h as usize];
    for y in 0..h as usize {
        out[y * dst_row + offset..y * dst_row + offset + src_row].copy_from_slice(&data[y * src_row..(y + 1) * src_row]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screens::drivers::catalog::{bulk_profile, fbl_profile};

    fn solid(w: u32, h: u32, rgba: [u8; 4]) -> Frame {
        Frame { width: w, height: h, rgba: rgba.iter().copied().cycle().take((w * h * 4) as usize).collect() }
    }

    #[test]
    fn rgb565_square_big_endian() {
        let p = fbl_profile(100, 100, 0).unwrap();
        let img = encode_frame(&p, &solid(320, 320, [255, 0, 0, 255]), 0, 80).unwrap();
        assert_eq!(img.data.len(), 204_800);
        assert_eq!(&img.data[..2], &[0xF8, 0x00]); // rouge, big-endian
        assert!(!img.jpeg);
    }

    #[test]
    fn rotate_panel_320x240_is_turned_to_240x320() {
        let p = fbl_profile(50, 50, 0).unwrap(); // base 90
        let img = encode_frame(&p, &solid(320, 240, [0, 0, 255, 255]), 0, 80).unwrap();
        assert_eq!((img.width, img.height, img.data.len()), (240, 320, 153_600));
        assert_eq!(&img.data[..2], &[0x1F, 0x00]); // bleu, little-endian
    }

    #[test]
    fn jpeg_profile_gives_jpeg_with_real_dims() {
        let p = bulk_profile(3, 0).unwrap(); // 480x480 JPEG
        let img = encode_frame(&p, &solid(480, 480, [10, 20, 30, 255]), 0, 80).unwrap();
        assert!(img.jpeg && img.data.starts_with(&[0xFF, 0xD8]) && img.data.len() < MAX_JPEG_BYTES);
        assert_eq!((img.width, img.height), (480, 480));
    }

    #[test]
    fn wrong_size_is_refused() {
        let p = fbl_profile(100, 100, 0).unwrap();
        assert!(encode_frame(&p, &solid(240, 240, [0; 4]), 0, 80).is_err());
        let bad = Frame { width: 320, height: 320, rgba: vec![0; 10] };
        assert!(encode_frame(&p, &bad, 0, 80).is_err());
    }

    #[test]
    fn row_repack_640x172() {
        // FBL 59 : 640x172 tourné de 270 -> 172x640, lignes de 172 px recopiées dans 176 px, 4 octets en retrait.
        let p = fbl_profile(59, 59, 0).unwrap();
        let img = encode_frame(&p, &solid(640, 172, [255, 255, 255, 255]), 0, 80).unwrap();
        assert_eq!((img.width, img.height), (172, 640));
        assert_eq!(img.data.len(), 225_280);
        assert_eq!(&img.data[..4], &[0, 0, 0, 0]); // marge gauche
        assert_eq!(&img.data[4..6], &[0xFF, 0xFF]);
        assert_eq!(&img.data[352 - 2..352], &[0, 0]); // marge droite
    }
}
