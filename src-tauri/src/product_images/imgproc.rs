//! Validation réelle d'une image reçue : décodage par la crate `image`, réduction à 640 px max,
//! ré-encodage PNG (supprime métadonnées et charges utiles). Jamais d'octets non décodés en sortie.

use anyhow::{bail, Context, Result};
use image::{ImageFormat, ImageReader, Limits};
use std::io::Cursor;

pub const MAX_BYTES: u64 = 4 << 20;
pub const MAX_SIDE_OUT: u32 = 640;
/// Au-delà : image « bombe » (décompression géante), refusée avant décodage.
const MAX_SIDE_IN: u32 = 8000;
const MAX_PIXELS_IN: u64 = 40_000_000;

fn reader(bytes: &[u8]) -> Result<ImageReader<Cursor<&[u8]>>> {
    let mut r = ImageReader::new(Cursor::new(bytes)).with_guessed_format().context("lecture de l'image")?;
    let mut lim = Limits::default();
    lim.max_image_width = Some(MAX_SIDE_IN);
    lim.max_image_height = Some(MAX_SIDE_IN);
    lim.max_alloc = Some(256 << 20);
    r.limits(lim);
    Ok(r)
}

/// Octets bruts reçus -> PNG ≤ 640 px.
pub fn to_png(bytes: &[u8]) -> Result<Vec<u8>> {
    if bytes.len() as u64 > MAX_BYTES {
        bail!("image trop volumineuse (plus de 4 Mo)");
    }
    match image::guess_format(bytes) {
        Ok(ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::Gif | ImageFormat::WebP | ImageFormat::Bmp) => {}
        _ => bail!("format d'image non reconnu"),
    }
    let (w, h) = reader(bytes)?.into_dimensions().context("dimensions illisibles")?;
    if w == 0 || h == 0 || u64::from(w) * u64::from(h) > MAX_PIXELS_IN {
        bail!("dimensions d'image refusées ({w}×{h})");
    }
    let mut img = reader(bytes)?.decode().context("image invalide ou corrompue")?;
    if img.width().max(img.height()) > MAX_SIDE_OUT {
        img = img.resize(MAX_SIDE_OUT, MAX_SIDE_OUT, image::imageops::FilterType::Lanczos3);
    }
    let mut out = Vec::new();
    img.write_to(&mut Cursor::new(&mut out), ImageFormat::Png).context("encodage PNG")?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut out = Vec::new();
        image::RgbaImage::from_pixel(w, h, image::Rgba([10, 20, 30, 255]))
            .write_to(&mut Cursor::new(&mut out), ImageFormat::Png)
            .unwrap();
        out
    }

    #[test]
    fn big_image_is_shrunk_to_640() {
        let out = to_png(&png(1600, 800)).unwrap();
        let img = image::load_from_memory(&out).unwrap();
        assert_eq!((img.width(), img.height()), (640, 320));
    }

    #[test]
    fn small_image_kept() {
        let img = image::load_from_memory(&to_png(&png(100, 50)).unwrap()).unwrap();
        assert_eq!((img.width(), img.height()), (100, 50));
    }

    #[test]
    fn non_image_refused() {
        assert!(to_png(b"MZ\x90\x00 ceci est un exe").is_err());
        assert!(to_png(b"<svg xmlns='http://www.w3.org/2000/svg'><script>1</script></svg>").is_err());
        assert!(to_png(b"").is_err());
    }

    #[test]
    fn truncated_image_refused() {
        let p = png(64, 64);
        assert!(to_png(&p[..p.len() / 2]).is_err());
    }

    #[test]
    fn too_many_bytes_refused() {
        assert!(to_png(&vec![0u8; MAX_BYTES as usize + 1]).is_err());
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut c = !0u32;
        for &b in data {
            c ^= u32::from(b);
            for _ in 0..8 {
                c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
            }
        }
        !c
    }

    #[test]
    fn giant_dimensions_refused() {
        // En-tête PNG valide (CRC recalculé) annonçant 20000 x 20000 : refusé avant tout décodage.
        let mut p = png(8, 8);
        p[16..20].copy_from_slice(&20000u32.to_be_bytes());
        p[20..24].copy_from_slice(&20000u32.to_be_bytes());
        let crc = crc32(&p[12..29]);
        p[29..33].copy_from_slice(&crc.to_be_bytes());
        let err = to_png(&p).unwrap_err().to_string();
        assert!(err.contains("dimensions") || err.contains("limit"), "{err}");
    }
}
