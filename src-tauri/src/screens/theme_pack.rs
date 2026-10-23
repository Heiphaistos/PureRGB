//! Thème = design + médias qu'il utilise, dans une archive `.purergb-theme` (zip sans compression) :
//! `manifest.json` (versions + SHA-256 de chaque média), `design.json`, `media/<id>.<ext>`.
//! L'archive est une entrée non fiable : noms stricts (liste blanche, aucun sous-dossier ni `..`),
//! plafonds de taille/nombre vérifiés sur les tailles déclarées ET à la lecture, SHA-256 contrôlé,
//! médias réimportés par `MediaStore::import` (qui revalide le format) avant d'enregistrer le design.

use super::designs::DesignStore;
use super::layout::{Background, Layout, WidgetKind};
use super::media::{MediaStore, EXTS, MAX_FILE_BYTES};
use super::video::VIDEO_EXTS;
use anyhow::{ensure, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

pub const THEME_EXT: &str = "purergb-theme";
const FORMAT: u32 = 1;
const MAX_JSON_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub archive_bytes: u64,
    pub files: usize,
    pub media_bytes: u64,
}

pub const LIMITS: Limits = Limits { archive_bytes: 200 * 1024 * 1024, files: 50, media_bytes: MAX_FILE_BYTES };

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    format: u32,
    app_version: String,
    layout_version: u32,
    name: String,
    media: Vec<MediaEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MediaEntry {
    id: String,
    name: String,
    ext: String,
    bytes: u64,
    sha256: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn ext_ok(ext: &str) -> bool {
    EXTS.contains(&ext) || VIDEO_EXTS.contains(&ext)
}

fn id_ok(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn require_theme_ext(path: &Path) -> Result<()> {
    let ok = path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case(THEME_EXT));
    ensure!(ok, "le fichier doit être un .{THEME_EXT}");
    Ok(())
}

/// Ids de médias utilisés par le design (fond + widgets Image), sans doublon.
pub fn media_ids(layout: &Layout) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    let mut add = |id: &str| {
        if !id.is_empty() && !ids.iter().any(|i| i == id) {
            ids.push(id.to_string());
        }
    };
    if let Background::Media { media_id, .. } = &layout.background {
        add(media_id);
    }
    for w in &layout.widgets {
        if let WidgetKind::Image { media_id, .. } = &w.kind {
            add(media_id);
        }
    }
    ids
}

pub fn export(layout: &Layout, media: &MediaStore, path: &Path) -> Result<()> {
    require_theme_ext(path)?;
    layout.validate().map_err(anyhow::Error::msg)?;
    let items = media.list();
    let mut entries = Vec::new();
    let mut blobs: Vec<(String, Vec<u8>)> = Vec::new();
    for id in media_ids(layout) {
        let file = media.file_of(&id).with_context(|| format!("média {id} introuvable dans la médiathèque"))?;
        let bytes = std::fs::read(&file).context("lecture d'un média")?;
        let ext = file.extension().and_then(|e| e.to_str()).unwrap_or_default().to_string();
        let name = items.iter().find(|m| m.id == id).map(|m| m.name.clone()).unwrap_or_else(|| id.clone());
        entries.push(MediaEntry { id: id.clone(), name, ext: ext.clone(), bytes: bytes.len() as u64, sha256: hex(&Sha256::digest(&bytes)) });
        blobs.push((format!("media/{id}.{ext}"), bytes));
    }
    let manifest = Manifest {
        format: FORMAT,
        app_version: env!("CARGO_PKG_VERSION").into(),
        layout_version: layout.version,
        name: layout.name.clone(),
        media: entries,
    };
    // Écriture dans un fichier voisin puis renommage : jamais d'archive tronquée à l'emplacement final.
    let tmp = path.with_extension("tmp");
    let res = write_zip(&tmp, &manifest, layout, &blobs);
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res?;
    std::fs::rename(&tmp, path).context("écriture de l'archive")
}

fn write_zip(path: &Path, manifest: &Manifest, layout: &Layout, blobs: &[(String, Vec<u8>)]) -> Result<()> {
    let file = std::fs::File::create(path).context("création de l'archive")?;
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    let mut put = |name: &str, data: &[u8]| -> Result<()> {
        zip.start_file(name, opts).context("écriture de l'archive")?;
        zip.write_all(data).context("écriture de l'archive")
    };
    put("manifest.json", &serde_json::to_vec_pretty(manifest)?)?;
    put("design.json", &serde_json::to_vec_pretty(layout)?)?;
    for (name, data) in blobs {
        put(name, data)?;
    }
    zip.finish().context("fin de l'archive")?;
    Ok(())
}

/// Lit une entrée en plafonnant ce qui est réellement décompressé (la taille déclarée peut mentir).
fn read_entry<R: Read>(entry: &mut R, declared: u64, max: u64) -> Result<Vec<u8>> {
    ensure!(declared <= max, "fichier trop volumineux dans l'archive");
    let mut buf = Vec::new();
    entry.take(max + 1).read_to_end(&mut buf).context("lecture de l'archive")?;
    ensure!(buf.len() as u64 <= max && buf.len() as u64 == declared, "taille incohérente dans l'archive");
    Ok(buf)
}

/// Nom de fichier affichable sûr pour la fiche média (le nom n'est jamais utilisé comme chemin).
fn safe_name(name: &str, fallback: &str) -> String {
    let s: String = name.chars().filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.' | '(' | ')')).take(60).collect();
    let s = s.trim().trim_matches('.').to_string();
    if s.is_empty() {
        fallback.to_string()
    } else {
        s
    }
}

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Result<Scratch> {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("purergb-theme-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).context("dossier temporaire")?;
        Ok(Scratch(dir))
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Valide l'archive, importe les médias puis enregistre le design. Retourne l'id du design.
pub fn import(path: &Path, designs: &DesignStore, media: &MediaStore, limits: Limits) -> Result<String> {
    require_theme_ext(path)?;
    let meta = std::fs::metadata(path).context("fichier introuvable")?;
    ensure!(meta.is_file(), "ce n'est pas un fichier");
    ensure!(meta.len() <= limits.archive_bytes, "archive trop volumineuse (max {} Mo)", limits.archive_bytes / 1024 / 1024);
    let mut zip = ZipArchive::new(std::fs::File::open(path).context("lecture de l'archive")?).context("archive illisible")?;
    ensure!(zip.len() <= limits.files, "trop de fichiers dans l'archive (max {})", limits.files);

    let (mut manifest_raw, mut design_raw, mut blobs) = (None, None, Vec::<(String, Vec<u8>)>::new());
    let mut total: u64 = 0;
    for i in 0..zip.len() {
        let mut e = zip.by_index(i).context("archive illisible")?;
        let name = e.name().to_string();
        let size = e.size();
        ensure!(!e.is_dir() && !e.is_symlink(), "entrée interdite dans l'archive : {name}");
        total = total.saturating_add(size);
        ensure!(total <= limits.archive_bytes, "contenu de l'archive trop volumineux");
        match name.as_str() {
            "manifest.json" => manifest_raw = Some(read_entry(&mut e, size, MAX_JSON_BYTES)?),
            "design.json" => design_raw = Some(read_entry(&mut e, size, MAX_JSON_BYTES)?),
            _ => {
                let file = name.strip_prefix("media/").with_context(|| format!("nom interdit dans l'archive : {name}"))?;
                let (id, ext) = file.rsplit_once('.').with_context(|| format!("nom de média invalide : {name}"))?;
                ensure!(id_ok(id) && ext_ok(ext), "nom ou extension de média refusé : {name}");
                blobs.push((file.to_string(), read_entry(&mut e, size, limits.media_bytes)?));
            }
        }
    }
    let manifest: Manifest = serde_json::from_slice(&manifest_raw.context("manifest.json absent")?).context("manifest illisible")?;
    ensure!(manifest.format == FORMAT, "format de thème non supporté : {}", manifest.format);
    let layout: Layout = serde_json::from_slice(&design_raw.context("design.json absent")?).context("design illisible")?;
    layout.validate().map_err(anyhow::Error::msg)?;
    ensure!(manifest.media.len() == blobs.len(), "médias de l'archive et du manifeste différents");

    // Tout est contrôlé avant la moindre écriture dans la médiathèque.
    let mut verified: Vec<(&MediaEntry, &Vec<u8>)> = Vec::new();
    for m in &manifest.media {
        ensure!(id_ok(&m.id) && ext_ok(&m.ext), "entrée de manifeste invalide : {}", m.id);
        let want = format!("{}.{}", m.id, m.ext);
        let (_, data) = blobs.iter().find(|(n, _)| *n == want).with_context(|| format!("média {want} absent de l'archive"))?;
        ensure!(data.len() as u64 == m.bytes && hex(&Sha256::digest(data)) == m.sha256.to_lowercase(), "somme SHA-256 incorrecte : {want}");
        verified.push((m, data));
    }
    for id in media_ids(&layout) {
        ensure!(manifest.media.iter().any(|m| m.id == id), "le design utilise le média {id} absent du thème");
    }

    let scratch = Scratch::new()?;
    for (i, (m, data)) in verified.iter().enumerate() {
        let file = scratch.0.join(format!("{}.{}", safe_name(&m.name, &format!("media{i}")), m.ext));
        std::fs::write(&file, data).context("fichier temporaire")?;
        let item = media.import(&file).with_context(|| format!("média « {} » refusé", m.name))?;
        ensure!(item.id == m.id, "l'identifiant du média {} ne correspond pas à son contenu", m.id);
    }
    designs.save_user(&layout)
}

#[cfg(test)]
mod tests;
