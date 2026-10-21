//! Médias intégrés à l'application (vidéo des thèmes « Forge ») : copiés dans la médiathèque au
//! démarrage s'ils n'y sont pas, SANS les sonder (le PC peut ne pas savoir décoder le MP4 avant
//! l'installation du décodeur de secours : le thème doit quand même apparaître dans la médiathèque).

use super::media::{MediaItem, MediaKind, MediaStore};
use sha2::{Digest, Sha256};

struct Builtin {
    name: &'static str,
    ext: &'static str,
    bytes: &'static [u8],
    width: u32,
    height: u32,
    frames: u32,
    duration_ms: u32,
}

const FORGE: Builtin = Builtin {
    name: "Forge (vidéo intégrée)",
    ext: "mp4",
    bytes: include_bytes!("../../assets/media/forge-video.mp4"),
    width: 1920,
    height: 1080,
    frames: 192,
    duration_ms: 8000,
};

fn id_of(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().take(8).map(|b| format!("{b:02x}")).collect()
}

/// Installe les médias intégrés absents de la médiathèque. Les erreurs sont journalisées, jamais fatales.
pub fn install(media: &MediaStore) {
    for b in [&FORGE] {
        let id = id_of(b.bytes);
        if media.file_of(&id).is_some() {
            continue;
        }
        let item = MediaItem { id, name: b.name.into(), kind: MediaKind::Video, width: b.width, height: b.height, frames: b.frames, duration_ms: b.duration_ms, backend: Default::default() };
        if let Err(e) = media.install_known(&item, b.ext, b.bytes) {
            log::warn!("média intégré {} : {e:#}", b.name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forge_video_id_matches_the_theme_designs() {
        let id = id_of(FORGE.bytes);
        let json = include_str!("designs/themes/theme-forge-droite-u1920.json");
        assert!(json.contains(&id), "l'id du média {id} doit apparaître dans les thèmes (relancer tools/themes/forge_themes.py)");
    }

    #[test]
    fn install_is_idempotent() {
        let dir = std::env::temp_dir().join(format!("purergb-builtin-media-{}", std::process::id()));
        let store = MediaStore::new(dir.clone());
        install(&store);
        install(&store);
        assert_eq!(store.list().iter().filter(|m| m.name.starts_with("Forge")).count(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }
}
