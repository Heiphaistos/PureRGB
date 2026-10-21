//! Conversions image -> format de fil des écrans : RGB565 (LE/BE), JPEG,
//! rotation par quarts de tour, luminosité logicielle.
//!
//! Formule RGB565 : REFERENCE_TECHNICAL.md (TRCC-Linux), section « Pixel Format » :
//! `((r & 0xF8) << 8) | ((g & 0xFC) << 3) | (b >> 3)`. L'alpha est ignoré
//! (le moteur de rendu fournit des images opaques).

use anyhow::{bail, Result};

/// Borne de sécurité : aucun écran connu ne dépasse 1920 px de côté.
const MAX_DIM: u32 = 8192;

fn check_len(rgba: &[u8], width: u32, height: u32) -> Result<()> {
    if width == 0 || height == 0 || width > MAX_DIM || height > MAX_DIM {
        bail!("Dimensions d'image invalides : {width}x{height}.");
    }
    let expected = (width as usize) * (height as usize) * 4;
    if rgba.len() != expected {
        bail!("Image RGBA incohérente : {} octets pour {width}x{height} (attendu {expected}).", rgba.len());
    }
    Ok(())
}

/// RGBA -> RGB565, 2 octets par pixel. `big_endian` : octet de poids fort d'abord.
/// Entrée non multiple de 4 : les octets en trop sont ignorés (usage interne, les
/// appelants valident déjà la taille de la `Frame`).
pub fn rgba_to_rgb565(rgba: &[u8], big_endian: bool) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgba.len() / 2);
    for px in rgba.chunks_exact(4) {
        let v = ((u16::from(px[0]) & 0xF8) << 8) | ((u16::from(px[1]) & 0xFC) << 3) | (u16::from(px[2]) >> 3);
        out.extend_from_slice(&if big_endian { v.to_be_bytes() } else { v.to_le_bytes() });
    }
    out
}

/// RGBA -> JPEG (baseline), qualité 1..=100.
pub fn rgba_to_jpeg(rgba: &[u8], width: u32, height: u32, quality: u8) -> Result<Vec<u8>> {
    check_len(rgba, width, height)?;
    if quality == 0 || quality > 100 {
        bail!("Qualité JPEG hors bornes : {quality} (1 à 100).");
    }
    let mut out = Vec::new();
    // Dimensions <= MAX_DIM (8192) : tiennent dans un u16.
    let enc = jpeg_encoder::Encoder::new(&mut out, quality);
    enc.encode(rgba, width as u16, height as u16, jpeg_encoder::ColorType::Rgba)
        .map_err(|e| anyhow::anyhow!("Encodage JPEG impossible : {e}"))?;
    Ok(out)
}

/// Rotation horaire de 0/90/180/270 degrés. Renvoie (pixels, largeur, hauteur) après rotation.
pub fn rotate_rgba(rgba: &[u8], width: u32, height: u32, angle: u16) -> Result<(Vec<u8>, u32, u32)> {
    check_len(rgba, width, height)?;
    let (w, h) = (width as usize, height as usize);
    let (nw, nh) = match angle {
        0 | 180 => (w, h),
        90 | 270 => (h, w),
        other => bail!("Rotation {other}° non supportée (0, 90, 180 ou 270)."),
    };
    if angle == 0 {
        return Ok((rgba.to_vec(), width, height));
    }
    let mut out = vec![0u8; rgba.len()];
    for y in 0..h {
        for x in 0..w {
            // Destination du pixel (x, y) pour une rotation horaire.
            let (dx, dy) = match angle {
                90 => (h - 1 - y, x),
                180 => (w - 1 - x, h - 1 - y),
                _ => (y, w - 1 - x), // 270
            };
            let (s, d) = ((y * w + x) * 4, (dy * nw + dx) * 4);
            out[d..d + 4].copy_from_slice(&rgba[s..s + 4]);
        }
    }
    Ok((out, nw as u32, nh as u32))
}

/// Luminosité logicielle : multiplie R, G, B par `percent`/100 (plafonné à 100). L'alpha est conservé.
pub fn apply_brightness(rgba: &mut [u8], percent: u8) {
    let p = u32::from(percent.min(100));
    if p == 100 {
        return;
    }
    for px in rgba.chunks_exact_mut(4) {
        for c in &mut px[..3] {
            *c = (u32::from(*c) * p / 100) as u8;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb565_known_values() {
        // Rouge pur : (0xF8)<<8 = 0xF800 ; vert pur : 0xFC<<3 = 0x07E0 ; bleu pur : 0xFF>>3 = 0x001F.
        let px = [255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255];
        assert_eq!(rgba_to_rgb565(&px, false), [0x00, 0xF8, 0xE0, 0x07, 0x1F, 0x00, 0xFF, 0xFF]);
        assert_eq!(rgba_to_rgb565(&px, true), [0xF8, 0x00, 0x07, 0xE0, 0x00, 0x1F, 0xFF, 0xFF]);
    }

    #[test]
    fn rgb565_truncates_low_bits() {
        // (0x07,0x03,0x07) -> tous les bits utiles à 0.
        assert_eq!(rgba_to_rgb565(&[7, 3, 7, 255], true), [0, 0]);
    }

    #[test]
    fn jpeg_has_markers_and_validates() {
        let img = vec![200u8; 16 * 16 * 4];
        let j = rgba_to_jpeg(&img, 16, 16, 80).unwrap();
        assert_eq!(&j[..2], &[0xFF, 0xD8]);
        assert_eq!(&j[j.len() - 2..], &[0xFF, 0xD9]);
        assert!(rgba_to_jpeg(&img, 16, 16, 0).is_err());
        assert!(rgba_to_jpeg(&img, 16, 15, 80).is_err());
        assert!(rgba_to_jpeg(&[], 0, 0, 80).is_err());
    }

    // Image 2x1 : A=(1,1,1,1) B=(2,2,2,2).
    const AB: [u8; 8] = [1, 1, 1, 1, 2, 2, 2, 2];

    #[test]
    fn rotations() {
        assert_eq!(rotate_rgba(&AB, 2, 1, 0).unwrap(), (AB.to_vec(), 2, 1));
        // 180 : B puis A.
        assert_eq!(rotate_rgba(&AB, 2, 1, 180).unwrap(), (vec![2, 2, 2, 2, 1, 1, 1, 1], 2, 1));
        // 90 horaire : image 1x2, A en haut, B en bas.
        assert_eq!(rotate_rgba(&AB, 2, 1, 90).unwrap(), (vec![1, 1, 1, 1, 2, 2, 2, 2], 1, 2));
        // 270 : B en haut, A en bas.
        assert_eq!(rotate_rgba(&AB, 2, 1, 270).unwrap(), (vec![2, 2, 2, 2, 1, 1, 1, 1], 1, 2));
        assert!(rotate_rgba(&AB, 2, 1, 45).is_err());
        assert!(rotate_rgba(&AB, 3, 1, 90).is_err());
    }

    #[test]
    fn rotate_90_four_times_is_identity() {
        let img: Vec<u8> = (0..(3 * 2 * 4) as u8).collect();
        let (mut cur, mut w, mut h) = (img.clone(), 3, 2);
        for _ in 0..4 {
            (cur, w, h) = rotate_rgba(&cur, w, h, 90).unwrap();
        }
        assert_eq!((cur, w, h), (img, 3, 2));
    }

    #[test]
    fn brightness() {
        let mut px = [200, 100, 50, 77];
        apply_brightness(&mut px, 50);
        assert_eq!(px, [100, 50, 25, 77]);
        apply_brightness(&mut px, 0);
        assert_eq!(px, [0, 0, 0, 77]);
        let mut full = [10, 20, 30, 40];
        apply_brightness(&mut full, 250);
        assert_eq!(full, [10, 20, 30, 40]);
    }
}
