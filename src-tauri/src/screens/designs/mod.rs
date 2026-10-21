//! Bibliothèque de designs : embarqués (`builtin.rs`) + designs de l'utilisateur
//! (`%APPDATA%\PureRGB\screens\designs\<id>.json`).

mod builtin;
mod generated;
mod themes;
#[cfg(test)]
mod quality;

use super::layout::Layout;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const MAX_DESIGN_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignMeta {
    pub id: String,
    pub name: String,
    pub category: String,
    pub width: u32,
    pub height: u32,
    pub tags: Vec<String>,
}

pub struct DesignStore {
    user_dir: PathBuf,
}

fn parse(json: &str) -> Result<Layout> {
    let layout: Layout = serde_json::from_str(json).context("design illisible")?;
    layout.validate().map_err(anyhow::Error::msg)?;
    Ok(layout)
}

/// Un id de design ne contient que `[a-z0-9-]` : pas de traversée de chemin possible.
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn slug(name: &str) -> String {
    let s: String = name
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let s = s.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    if s.is_empty() {
        "design".into()
    } else {
        s.chars().take(40).collect()
    }
}

impl DesignStore {
    pub fn new(root: PathBuf) -> Self {
        DesignStore { user_dir: root.join("designs") }
    }

    fn user_path(&self, id: &str) -> Result<PathBuf> {
        if !valid_id(id) {
            bail!("identifiant de design invalide");
        }
        Ok(self.user_dir.join(format!("{id}.json")))
    }

    pub fn list(&self) -> Vec<DesignMeta> {
        let mut out: Vec<DesignMeta> = builtin::all()
            .filter_map(|b| match parse(b.json) {
                Ok(l) => Some(DesignMeta {
                    id: b.id.into(),
                    name: l.name,
                    category: b.category.into(),
                    width: l.width,
                    height: l.height,
                    tags: b.tags.iter().map(|t| t.to_string()).collect(),
                }),
                Err(e) => {
                    log::error!("design embarqué {} invalide: {e:#}", b.id);
                    None
                }
            })
            .collect();
        if let Ok(rd) = std::fs::read_dir(&self.user_dir) {
            for e in rd.flatten() {
                let p = e.path();
                let Some(id) = p.file_stem().and_then(|s| s.to_str()).map(str::to_string) else { continue };
                if p.extension().and_then(|x| x.to_str()) != Some("json") || !valid_id(&id) {
                    continue;
                }
                if let Ok(l) = self.get(&id) {
                    out.push(DesignMeta {
                        id,
                        name: l.name,
                        category: "user".into(),
                        width: l.width,
                        height: l.height,
                        tags: vec!["user".into()],
                    });
                }
            }
        }
        out
    }

    pub fn get(&self, id: &str) -> Result<Layout> {
        if let Some(b) = builtin::all().find(|b| b.id == id) {
            return parse(b.json);
        }
        read_layout(&self.user_path(id)?)
    }

    /// Enregistre un design utilisateur ; retourne son id (slug du nom, suffixé si pris).
    pub fn save_user(&self, layout: &Layout) -> Result<String> {
        layout.validate().map_err(anyhow::Error::msg)?;
        std::fs::create_dir_all(&self.user_dir)?;
        let base = format!("u-{}", slug(&layout.name));
        let mut id = base.clone();
        let mut n = 2;
        while self.user_path(&id)?.exists() && !self.same_name(&id, &layout.name) {
            id = format!("{base}-{n}");
            n += 1;
        }
        std::fs::write(self.user_path(&id)?, serde_json::to_vec_pretty(layout)?)?;
        Ok(id)
    }

    fn same_name(&self, id: &str, name: &str) -> bool {
        self.get(id).map(|l| l.name == name).unwrap_or(false)
    }

    pub fn delete_user(&self, id: &str) -> Result<()> {
        if builtin::all().any(|b| b.id == id) {
            bail!("un design fourni avec PureRGB ne peut pas être supprimé");
        }
        std::fs::remove_file(self.user_path(id)?).context("design introuvable")
    }

    pub fn export(&self, id: &str, path: &Path) -> Result<()> {
        require_json(path)?;
        let layout = self.get(id)?;
        std::fs::write(path, serde_json::to_vec_pretty(&layout)?).context("écriture impossible")
    }

    pub fn import(&self, path: &Path) -> Result<String> {
        require_json(path)?;
        let layout = read_layout(path)?;
        self.save_user(&layout)
    }
}

fn require_json(path: &Path) -> Result<()> {
    if path.extension().and_then(|e| e.to_str()).map(str::to_lowercase).as_deref() != Some("json") {
        bail!("le fichier doit être un .json");
    }
    Ok(())
}

fn read_layout(path: &Path) -> Result<Layout> {
    let meta = std::fs::metadata(path).context("fichier introuvable")?;
    if meta.len() > MAX_DESIGN_BYTES {
        bail!("design trop volumineux (max 2 Mo)");
    }
    parse(&std::fs::read_to_string(path).context("lecture impossible")?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> DesignStore {
        DesignStore::new(std::env::temp_dir().join(format!("purergb-designs-{}", std::process::id())))
    }

    #[test]
    fn every_builtin_design_parses_and_validates() {
        for b in builtin::all() {
            parse(b.json).unwrap_or_else(|e| panic!("{} : {e:#}", b.id));
        }
        let all: Vec<_> = builtin::all().map(|b| b.id).collect();
        let ids: std::collections::HashSet<_> = all.iter().collect();
        assert_eq!(ids.len(), all.len(), "ids dupliqués");
    }

    #[test]
    fn user_designs_roundtrip_and_reject_hostile_ids() {
        let s = store();
        let l = s.get("gen-analogique-carte-mere-cyber-u1920").unwrap();
        let id = s.save_user(&l).unwrap();
        assert!(id.starts_with("u-"));
        assert_eq!(s.get(&id).unwrap().name, l.name);
        assert!(s.list().iter().any(|m| m.id == id && m.category == "user"));
        assert!(s.get("../settings").is_err());
        assert!(s.delete_user("gen-analogique-carte-mere-cyber-u1920").is_err());
        s.delete_user(&id).unwrap();
        std::fs::remove_dir_all(&s.user_dir).ok();
    }

    /// Valeurs plausibles pour toutes les clés stables + historique (courbes visibles).
    pub(super) fn preview_store() -> crate::screens::bindings::ValueStore {
        let store = crate::screens::bindings::ValueStore::empty();
        let fixed: [(&str, f64); 46] = [
            ("cpu.temp", 58.0), ("cpu.load", 42.0), ("cpu.clock", 4850.0), ("cpu.power", 65.0), ("cpu.voltage", 1.25), ("cpu.fan", 1200.0),
            ("gpu.temp", 71.0), ("gpu.hotspot", 82.0), ("gpu.load", 63.0), ("gpu.clock", 2475.0), ("gpu.mem_clock", 1750.0), ("gpu.power", 180.0),
            ("gpu.vram_used_pct", 48.0), ("gpu.vram_used_gb", 7.7), ("gpu.fan", 1500.0), ("ram.used_pct", 61.0), ("ram.used_gb", 19.5),
            ("ram.free_gb", 12.5), ("ram.total_gb", 32.0), ("mobo.temp", 37.0), ("chipset.temp", 48.0), ("vrm.temp", 55.0),
            ("ssd.1.temp", 41.0), ("ssd.1.used_pct", 55.0), ("ssd.2.temp", 38.0), ("ssd.2.used_pct", 23.0),
            ("disk.C.used_pct", 72.0), ("disk.C.free_gb", 138.0), ("disk.C.total_gb", 500.0), ("disk.D.used_pct", 41.0),
            ("disk.D.free_gb", 1180.0), ("disk.D.total_gb", 2000.0), ("disk.read", 420.0), ("disk.write", 180.0),
            ("net.up", 1.2), ("net.down", 8.4), ("liquid.temp", 33.0), ("pump.rpm", 2800.0), ("fan.1.rpm", 900.0), ("fan.2.rpm", 950.0),
            ("fan.3.rpm", 880.0), ("uptime", 93600.0), ("cpu.core1", 40.0), ("cpu.core2", 44.0), ("gpu.core", 63.0), ("net.ping", 14.0),
        ];
        for (k, v) in fixed {
            for i in 0..48 {
                let wave = ((i as f64) * 0.45 + v * 0.07).sin() * 0.12 + ((i as f64) * 0.13).cos() * 0.05;
                store.set_for_test(k, if i == 47 { v } else { v * (1.0 + wave) });
            }
        }
        store
    }

    /// Rend chaque design embarqué avec des valeurs plausibles : PNG dans
    /// `src-tauri/target/screens-preview/designs/` (indépendant de CARGO_TARGET_DIR ; surcharge
    /// `PURERGB_PREVIEW_DIR`) pour revue à l'œil, et refuse un rendu vide. Les rendus sont
    /// limités à 640 px de côté pour rester rapides.
    #[test]
    fn every_builtin_design_renders_a_non_blank_preview() {
        use crate::screens::{media::MediaStore, render};
        let store = preview_store();
        let media = MediaStore::new(std::env::temp_dir().join("purergb-design-preview"));
        crate::screens::builtin_media::install(&media);
        let out = std::env::var_os("PURERGB_PREVIEW_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/screens-preview/designs"));
        std::fs::create_dir_all(&out).unwrap();
        // Les ids générés changent à chaque régénération : on retire les anciens aperçus.
        for e in std::fs::read_dir(&out).unwrap().flatten() {
            if e.file_name().to_string_lossy().starts_with("gen-") {
                std::fs::remove_file(e.path()).ok();
            }
        }
        for b in builtin::all() {
            let l = parse(b.json).unwrap();
            let k = (640.0 / l.width.max(l.height) as f32).min(1.0);
            let (rw, rh) = (((l.width as f32 * k).round() as u32).max(1), ((l.height as f32 * k).round() as u32).max(1));
            let f = render::render(&l, &store, &media, rw, rh, 0);
            let distinct = f.rgba.chunks(4).step_by(97).map(|p| [p[0], p[1], p[2]]).collect::<std::collections::HashSet<_>>().len();
            assert!(distinct > 8, "design {} rendu quasi uni ({distinct} couleurs)", b.id);
            let img = image::RgbaImage::from_raw(f.width, f.height, f.rgba).unwrap();
            img.save(out.join(format!("{}.png", b.id))).unwrap();
        }
    }
}
