//! Auto-update de l'OpenRGB embarqué : vérifie la dernière release Codeberg
//! au lancement, télécharge et bascule les copies gérées par PureRGB
//! (bundle NSIS, `%APPDATA%\PureRGB\openrgb`) avec rollback automatique si
//! la nouvelle version ne démarre pas. Ne touche jamais une installation
//! OpenRGB indépendante de l'utilisateur.

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
