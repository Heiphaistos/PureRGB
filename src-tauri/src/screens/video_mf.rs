//! Décodage vidéo par le Media Foundation de Windows (`IMFSourceReader`, mode synchrone).
//! Aucun binaire tiers : les codecs sont ceux installés dans Windows (H.264/HEVC/VP9/AV1
//! selon les « Extensions vidéo » du Microsoft Store). Sortie RGB32 (BGRX) grâce à
//! `MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING`, convertie en Pixmap par `bgrx_to_pixmap`.
//! COM et MF sont initialisés par thread (`MfGuard`) : une `MfSource` vit et meurt sur
//! le thread qui l'a ouverte.

use super::video::{bgrx_to_pixmap, Decoded, FrameSource, VideoError, VideoInfo, MAX_OUT, MAX_SOURCE_SIDE};
use anyhow::{anyhow, bail, Result};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::core::{GUID, PCWSTR};
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

const VIDEO: u32 = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;
const ALL: u32 = MF_SOURCE_READER_ALL_STREAMS.0 as u32;
const MEDIA_SOURCE: u32 = MF_SOURCE_READER_MEDIASOURCE.0 as u32;
/// `MF_VERSION` du SDK (MF_SDK_VERSION << 16 | MF_API_VERSION).
const MF_VERSION_NUM: u32 = (MF_SDK_VERSION << 16) | MF_API_VERSION;
/// Échantillons vides (trous, ticks) tolérés d'affilée avant d'abandonner.
const MAX_EMPTY: u32 = 200;

/// COM + Media Foundation pour la durée de vie du thread courant.
struct MfGuard {
    com: bool,
}

impl MfGuard {
    fn new() -> Result<MfGuard> {
        // SAFETY: appels d'initialisation sans pointeur ; balancés dans Drop.
        let com = unsafe {
            let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
            if hr.is_ok() {
                true
            } else if hr == windows::Win32::Foundation::RPC_E_CHANGED_MODE {
                false // thread déjà initialisé dans un autre mode : on l'utilise tel quel
            } else {
                bail!("initialisation COM impossible : {hr:?}");
            }
        };
        // SAFETY: voir plus haut.
        if let Err(e) = unsafe { MFStartup(MF_VERSION_NUM, MFSTARTUP_LITE) } {
            if com {
                // SAFETY: CoInitializeEx a réussi juste au-dessus.
                unsafe { CoUninitialize() };
            }
            return Err(VideoError::Codec(format!("Media Foundation indisponible : {e}")).into());
        }
        Ok(MfGuard { com })
    }
}

impl Drop for MfGuard {
    fn drop(&mut self) {
        // SAFETY: balance MFStartup / CoInitializeEx de `new`.
        unsafe {
            let _ = MFShutdown();
            if self.com {
                CoUninitialize();
            }
        }
    }
}

pub struct MfSource {
    // Ordre des champs = ordre de libération : le lecteur avant l'arrêt de MF/COM.
    reader: IMFSourceReader,
    w: u32,
    h: u32,
    stride: i32,
    /// Taille d'affichage (la sortie décodée peut être arrondie au macrobloc : 1088 pour 1080).
    show: (u32, u32),
    _guard: MfGuard,
}

fn pack(hi: u32, lo: u32) -> u64 {
    (u64::from(hi) << 32) | u64::from(lo)
}

fn unpack(v: u64) -> (u32, u32) {
    ((v >> 32) as u32, v as u32)
}

fn codec_err<E: std::fmt::Display>(e: E) -> anyhow::Error {
    VideoError::Codec(e.to_string()).into()
}

/// Taille de sortie : celle de la source, réduite (côtés pairs) pour tenir dans `MAX_OUT`.
fn target_size(w: u32, h: u32) -> (u32, u32) {
    if w <= MAX_OUT.0 && h <= MAX_OUT.1 {
        return (w, h);
    }
    let k = (MAX_OUT.0 as f64 / w as f64).min(MAX_OUT.1 as f64 / h as f64);
    let even = |v: u32| (((v as f64 * k) as u32) & !1).max(2);
    (even(w), even(h))
}

fn create_reader(path: &Path, advanced: bool) -> Result<IMFSourceReader> {
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    // SAFETY: `wide` est terminée par NUL et vit pendant l'appel ; les interfaces COM sont gérées par `windows`.
    unsafe {
        let mut attrs: Option<IMFAttributes> = None;
        MFCreateAttributes(&mut attrs, 1).map_err(codec_err)?;
        let attrs = attrs.ok_or_else(|| anyhow!("attributs Media Foundation indisponibles"))?;
        let key = if advanced { MF_SOURCE_READER_ENABLE_ADVANCED_VIDEO_PROCESSING } else { MF_SOURCE_READER_ENABLE_VIDEO_PROCESSING };
        attrs.SetUINT32(&key, 1).map_err(codec_err)?;
        let reader = MFCreateSourceReaderFromURL(PCWSTR(wide.as_ptr()), &attrs).map_err(codec_err)?;
        reader.SetStreamSelection(ALL, false).map_err(codec_err)?;
        reader.SetStreamSelection(VIDEO, true).map_err(|_| anyhow::Error::from(VideoError::NoTrack))?;
        Ok(reader)
    }
}

/// Taille et cadence natives de la première piste vidéo.
fn native_info(reader: &IMFSourceReader) -> Result<(u32, u32, f32)> {
    // SAFETY: appels COM sur un lecteur valide.
    unsafe {
        let t = reader.GetNativeMediaType(VIDEO, 0).map_err(|_| anyhow::Error::from(VideoError::NoTrack))?;
        let (w, h) = unpack(t.GetUINT64(&MF_MT_FRAME_SIZE).map_err(codec_err)?);
        let fps = t.GetUINT64(&MF_MT_FRAME_RATE).map(unpack).ok().filter(|r| r.1 != 0).map_or(0.0, |(n, d)| n as f32 / d as f32);
        Ok((w, h, fps))
    }
}

impl MfSource {
    /// Ouvre `path` sur le thread courant et renvoie le lecteur + les infos de la vidéo.
    pub fn open(path: &Path) -> Result<(MfSource, VideoInfo)> {
        if !path.is_file() {
            bail!("fichier vidéo introuvable");
        }
        let guard = MfGuard::new()?;
        let mut reader = create_reader(path, false)?;
        let (nw, nh, fps) = native_info(&reader)?;
        if nw == 0 || nh == 0 || nw > MAX_SOURCE_SIDE || nh > MAX_SOURCE_SIDE {
            bail!("vidéo trop grande ({nw}×{nh}, max {MAX_SOURCE_SIDE} px de côté)");
        }
        let (tw, th) = target_size(nw, nh);
        if (tw, th) != (nw, nh) {
            reader = create_reader(path, true)?; // le redimensionnement exige le traitement avancé
        }
        // SAFETY: appels COM sur des objets valides ; `mt` est consommé par SetCurrentMediaType.
        let (w, h, stride, duration_ms) = unsafe {
            let mt = MFCreateMediaType().map_err(codec_err)?;
            mt.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video).map_err(codec_err)?;
            mt.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32).map_err(codec_err)?;
            if (tw, th) != (nw, nh) {
                mt.SetUINT64(&MF_MT_FRAME_SIZE, pack(tw, th)).map_err(codec_err)?;
            }
            reader.SetCurrentMediaType(VIDEO, None, &mt).map_err(codec_err)?;
            let (w, h, stride) = current_geometry(&reader)?;
            let dur = reader
                .GetPresentationAttribute(MEDIA_SOURCE, &MF_PD_DURATION)
                .ok()
                .and_then(|v| u64::try_from(&v).ok())
                .map_or(0, |d| (d / 10_000).min(u64::from(u32::MAX)) as u32);
            (w, h, stride, dur)
        };
        let info = VideoInfo { width: nw, height: nh, duration_ms, fps };
        Ok((MfSource { reader, w, h, stride, show: (tw, th), _guard: guard }, info))
    }

    fn refresh_geometry(&mut self) -> Result<()> {
        let (w, h, stride) = current_geometry(&self.reader)?;
        (self.w, self.h, self.stride) = (w, h, stride);
        Ok(())
    }
}

/// Taille et pas (négatif = image de bas en haut) du type de sortie courant.
fn current_geometry(reader: &IMFSourceReader) -> Result<(u32, u32, i32)> {
    // SAFETY: appels COM sur un lecteur valide.
    unsafe {
        let t = reader.GetCurrentMediaType(VIDEO).map_err(codec_err)?;
        let (w, h) = unpack(t.GetUINT64(&MF_MT_FRAME_SIZE).map_err(codec_err)?);
        if w == 0 || h == 0 || w > MAX_SOURCE_SIDE || h > MAX_SOURCE_SIDE {
            bail!("taille de sortie vidéo invalide ({w}×{h})");
        }
        let stride = t.GetUINT32(&MF_MT_DEFAULT_STRIDE).map(|s| s as i32).unwrap_or((w * 4) as i32);
        Ok((w, h, if stride == 0 { (w * 4) as i32 } else { stride }))
    }
}

impl FrameSource for MfSource {
    fn next_frame(&mut self) -> Result<Option<Decoded>> {
        for _ in 0..MAX_EMPTY {
            let (mut flags, mut ts) = (0u32, 0i64);
            let mut sample: Option<IMFSample> = None;
            // SAFETY: les pointeurs de sortie désignent des variables locales valides.
            unsafe { self.reader.ReadSample(VIDEO, 0, None, Some(&mut flags), Some(&mut ts), Some(&mut sample)) }
                .map_err(|e| anyhow!("lecture de la vidéo : {e}"))?;
            if flags & MF_SOURCE_READERF_CURRENTMEDIATYPECHANGED.0 as u32 != 0 {
                self.refresh_geometry()?;
            }
            if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                return Ok(None);
            }
            let Some(sample) = sample else { continue };
            let pixmap = self.to_pixmap(&sample)?;
            return Ok(Some(Decoded { pixmap, ts_ms: (ts.max(0) / 10_000) as u64 }));
        }
        bail!("la vidéo ne produit plus d'images")
    }

    fn rewind(&mut self) -> Result<()> {
        // SAFETY: PROPVARIANT i64 valide, GUID_NULL = unité 100 ns.
        unsafe { self.reader.SetCurrentPosition(&GUID::zeroed(), &PROPVARIANT::from(0i64)) }
            .map_err(|e| anyhow!("retour au début de la vidéo : {e}"))
    }
}

impl MfSource {
    fn to_pixmap(&self, sample: &IMFSample) -> Result<tiny_skia::Pixmap> {
        // SAFETY: le tampon est verrouillé le temps de la copie, puis toujours déverrouillé.
        unsafe {
            let buf = sample.ConvertToContiguousBuffer().map_err(|e| anyhow!("tampon vidéo : {e}"))?;
            let (mut ptr, mut len) = (std::ptr::null_mut::<u8>(), 0u32);
            buf.Lock(&mut ptr, None, Some(&mut len)).map_err(|e| anyhow!("verrouillage du tampon vidéo : {e}"))?;
            let out = if ptr.is_null() { None } else { bgrx_to_pixmap(self.w, self.h, self.stride, std::slice::from_raw_parts(ptr, len as usize), self.show) };
            let _ = buf.Unlock();
            out.ok_or_else(|| anyhow!("image vidéo de taille inattendue"))
        }
    }
}
