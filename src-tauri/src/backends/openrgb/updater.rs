//! Auto-update de l'OpenRGB embarqué : vérifie la dernière release Codeberg
//! au lancement, télécharge et bascule les copies gérées par PureRGB
//! (bundle NSIS, `%APPDATA%\PureRGB\openrgb`) avec rollback automatique si
//! la nouvelle version ne démarre pas. Ne touche jamais une installation
//! OpenRGB indépendante de l'utilisateur.

use anyhow::{bail, Context, Result};

const RELEASES_API: &str =
    "https://codeberg.org/api/v1/repos/OpenRGB/OpenRGB/releases?limit=1";

/// Sélectionne l'asset Windows 64-bit dans la liste des assets d'une
/// release Codeberg. Le hash de commit dans le nom change à chaque
/// release (`OpenRGB_1.0rc3_Windows_64_6fbcf62.zip`) — sélection par motif,
/// exclut explicitement le `.msi`.
pub fn pick_windows_asset(assets: &[(String, String)]) -> Option<(String, String)> {
    assets
        .iter()
        .find(|(name, _)| {
            name.starts_with("OpenRGB_") && name.contains("_Windows_64_") && name.ends_with(".zip")
        })
        .cloned()
}

/// true si la version distante diffère de la version enregistrée (ou si
/// aucune version n'a jamais été enregistrée).
pub fn needs_update(latest_tag: &str, saved_version: &Option<String>) -> bool {
    saved_version.as_deref() != Some(latest_tag)
}

/// Interroge l'API Codeberg pour la dernière release OpenRGB publiée.
/// Retourne `(tag_name, download_url)` de l'asset Windows 64-bit.
/// Aucun fichier de checksums n'est publié par OpenRGB : seule garantie,
/// HTTPS vers `codeberg.org` (décision validée dans la spec).
pub fn check_latest_version() -> Result<(String, String)> {
    let body = crate::netdev::curl(&[RELEASES_API]).context("requête releases Codeberg")?;
    let releases: serde_json::Value =
        serde_json::from_str(&body).context("parsing réponse Codeberg")?;
    let release = releases
        .get(0)
        .context("aucune release Codeberg trouvée")?;
    let tag_name = release
        .get("tag_name")
        .and_then(|v| v.as_str())
        .context("tag_name absent de la réponse Codeberg")?
        .to_string();
    let assets: Vec<(String, String)> = release
        .get("assets")
        .and_then(|v| v.as_array())
        .context("assets absents de la réponse Codeberg")?
        .iter()
        .filter_map(|a| {
            let name = a.get("name")?.as_str()?.to_string();
            let url = a.get("browser_download_url")?.as_str()?.to_string();
            Some((name, url))
        })
        .collect();
    let (_, download_url) = pick_windows_asset(&assets)
        .context("aucun asset Windows 64-bit trouvé dans la release")?;
    Ok((tag_name, download_url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_windows_asset_selects_zip_by_pattern() {
        let assets = vec![
            ("OpenRGB_1.0rc3_Windows_64_6fbcf62.zip".to_string(), "https://a/zip".to_string()),
            ("OpenRGB_1.0rc3_Windows_64_6fbcf62.msi".to_string(), "https://a/msi".to_string()),
            ("OpenRGB_1.0rc3_Source_Code.zip".to_string(), "https://a/src".to_string()),
        ];
        let picked = pick_windows_asset(&assets);
        assert_eq!(
            picked,
            Some((
                "OpenRGB_1.0rc3_Windows_64_6fbcf62.zip".to_string(),
                "https://a/zip".to_string()
            ))
        );
    }

    #[test]
    fn pick_windows_asset_handles_varying_commit_hash() {
        let assets = vec![(
            "OpenRGB_1.0rc4_Windows_64_a1b2c3d.zip".to_string(),
            "https://a/other".to_string(),
        )];
        assert!(pick_windows_asset(&assets).is_some());
    }

    #[test]
    fn pick_windows_asset_none_when_missing() {
        let assets = vec![("OpenRGB_1.0rc3_Linux_x64.tar.gz".to_string(), "https://a/lin".to_string())];
        assert_eq!(pick_windows_asset(&assets), None);
    }

    #[test]
    fn needs_update_true_when_different() {
        assert!(needs_update("release_candidate_1.0rc4", &Some("release_candidate_1.0rc3".to_string())));
    }

    #[test]
    fn needs_update_false_when_same() {
        assert!(!needs_update("release_candidate_1.0rc3", &Some("release_candidate_1.0rc3".to_string())));
    }

    #[test]
    fn needs_update_true_when_never_checked() {
        assert!(needs_update("release_candidate_1.0rc3", &None));
    }
}
