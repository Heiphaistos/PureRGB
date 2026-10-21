use super::*;
use crate::screens::layout::{Fit, Widget};

fn tmp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("purergb-theme-test-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn png_file(dir: &Path, name: &str, shade: u8) -> PathBuf {
    let p = dir.join(name);
    image::RgbaImage::from_pixel(8, 8, image::Rgba([shade, 10, 200, 255])).save(&p).unwrap();
    p
}

fn stores(dir: &Path) -> (DesignStore, MediaStore) {
    (DesignStore::new(dir.to_path_buf()), MediaStore::new(dir.join("media")))
}

fn themed_layout(media_id: &str) -> Layout {
    let mut l = DesignStore::new(tmp("seed")).get("gen-analogique-carte-mere-cyber-u1920").unwrap();
    l.name = "Thème test".into();
    l.background = Background::Media { media_id: media_id.into(), fit: Fit::Cover };
    let w: Widget = serde_json::from_value(serde_json::json!({
        "id": "logo", "x": 0, "y": 0, "w": 10, "h": 10, "type": "image", "media_id": media_id, "fit": "contain"
    }))
    .unwrap();
    l.widgets.push(w);
    l
}

/// Archive construite à la main, sans passer par `export` (pour fabriquer des archives piégées).
fn write(path: &Path, entries: &[(String, Vec<u8>)]) {
    let mut z = ZipWriter::new(std::fs::File::create(path).unwrap());
    let o = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for (n, d) in entries {
        z.start_file(n.as_str(), o).unwrap();
        z.write_all(d).unwrap();
    }
    z.finish().unwrap();
}

fn manifest(id: &str, ext: &str, data: &[u8], sha: Option<&str>) -> Vec<u8> {
    serde_json::to_vec(&Manifest {
        format: 1,
        app_version: "0".into(),
        layout_version: 1,
        name: "t".into(),
        media: vec![MediaEntry {
            id: id.into(),
            name: "img".into(),
            ext: ext.into(),
            bytes: data.len() as u64,
            sha256: sha.map(str::to_string).unwrap_or_else(|| hex(&Sha256::digest(data))),
        }],
    })
    .unwrap()
}

fn good_entries(id: &str, data: &[u8]) -> Vec<(String, Vec<u8>)> {
    vec![
        ("manifest.json".into(), manifest(id, "png", data, None)),
        ("design.json".into(), serde_json::to_vec(&themed_layout(id)).unwrap()),
        (format!("media/{id}.png"), data.to_vec()),
    ]
}

#[test]
fn export_then_import_roundtrip_keeps_media_ids() {
    let (a, b) = (tmp("a"), tmp("b"));
    let (_, ma) = stores(&a);
    let item = ma.import(&png_file(&a, "fond.png", 5)).unwrap();
    let layout = themed_layout(&item.id);
    let out = a.join("t.purergb-theme");
    export(&layout, &ma, &out).unwrap();
    assert_eq!(media_ids(&layout), vec![item.id.clone()]);

    let (db, mb) = stores(&b);
    let id = import(&out, &db, &mb, LIMITS).unwrap();
    assert_eq!(db.get(&id).unwrap().name, "Thème test");
    assert!(mb.list().iter().any(|m| m.id == item.id && m.name == "fond"));
    // Réimport : médias dédupliqués.
    import(&out, &db, &mb, LIMITS).unwrap();
    assert_eq!(mb.list().len(), 1);
}

#[test]
fn export_refuses_missing_media_and_wrong_extension() {
    let a = tmp("exp");
    let (_, m) = stores(&a);
    assert!(export(&themed_layout("deadbeef"), &m, &a.join("t.purergb-theme")).is_err());
    assert!(export(&themed_layout(""), &m, &a.join("t.zip")).is_err());
}

#[test]
fn hostile_entry_names_are_refused_and_nothing_is_imported() {
    let d = tmp("evil");
    let data = std::fs::read(png_file(&d, "x.png", 9)).unwrap();
    for evil in ["..\\evil.png", "../evil.png", "/etc/evil.png", "C:\\evil.png", "media/../evil.png", "media/sub/a.png", "media\\a.png", "evil.png"] {
        let mut e = good_entries("abc123", &data);
        e.push((evil.into(), data.clone()));
        let p = d.join("evil.purergb-theme");
        write(&p, &e);
        let (ds, ms) = stores(&d.join("dest"));
        assert!(import(&p, &ds, &ms, LIMITS).is_err(), "{evil} accepté");
        assert!(ms.list().is_empty());
    }
}

#[test]
fn executable_and_unknown_extensions_are_refused() {
    let d = tmp("exe");
    let data = b"MZ not an image".to_vec();
    for ext in ["exe", "dll", "bat", "svg", "json"] {
        let e = vec![
            ("manifest.json".to_string(), manifest("abc", ext, &data, None)),
            ("design.json".to_string(), serde_json::to_vec(&themed_layout("abc")).unwrap()),
            (format!("media/abc.{ext}"), data.clone()),
        ];
        let p = d.join("x.purergb-theme");
        write(&p, &e);
        let (ds, ms) = stores(&d.join("dest"));
        assert!(import(&p, &ds, &ms, LIMITS).is_err(), ".{ext} accepté");
    }
}

#[test]
fn wrong_sha256_is_refused() {
    let d = tmp("sha");
    let data = std::fs::read(png_file(&d, "x.png", 3)).unwrap();
    let mut e = good_entries("abc", &data);
    e[0].1 = manifest("abc", "png", &data, Some(&"0".repeat(64)));
    let p = d.join("x.purergb-theme");
    write(&p, &e);
    let (ds, ms) = stores(&d.join("dest"));
    let err = import(&p, &ds, &ms, LIMITS).unwrap_err().to_string();
    assert!(err.contains("SHA-256"), "{err}");
    assert!(ms.list().is_empty());
}

#[test]
fn bomb_like_archives_hit_the_limits() {
    let d = tmp("bomb");
    let data = std::fs::read(png_file(&d, "x.png", 3)).unwrap();
    let mut big = data.clone();
    big.resize(300_000, 0);
    let p = d.join("x.purergb-theme");
    let (ds, ms) = stores(&d.join("dest"));
    // Média au-delà du plafond par fichier.
    write(&p, &good_entries("abc", &big));
    let per_file = Limits { archive_bytes: 10_000_000, files: 50, media_bytes: 100_000 };
    assert!(import(&p, &ds, &ms, per_file).is_err());
    // Total au-delà du plafond de l'archive.
    let total = Limits { archive_bytes: 200_000, files: 50, media_bytes: 100_000_000 };
    assert!(import(&p, &ds, &ms, total).is_err());
    // Trop de fichiers.
    let mut e = good_entries("abc", &data);
    for i in 0..5 {
        e.push((format!("media/f{i}.png"), data.clone()));
    }
    write(&p, &e);
    let few = Limits { archive_bytes: 10_000_000, files: 4, media_bytes: 1_000_000 };
    assert!(import(&p, &ds, &ms, few).is_err());
    assert!(ms.list().is_empty());
}

#[test]
fn media_used_by_design_must_be_in_manifest() {
    let d = tmp("missing");
    let data = std::fs::read(png_file(&d, "x.png", 3)).unwrap();
    let mut e = good_entries("abc", &data);
    e[1].1 = serde_json::to_vec(&themed_layout("zzz")).unwrap();
    let p = d.join("x.purergb-theme");
    write(&p, &e);
    let (ds, ms) = stores(&d.join("dest"));
    assert!(import(&p, &ds, &ms, LIMITS).is_err());
}

#[test]
fn id_must_match_content_hash() {
    let d = tmp("idmatch");
    let data = std::fs::read(png_file(&d, "x.png", 3)).unwrap();
    let p = d.join("x.purergb-theme");
    write(&p, &good_entries("abc123", &data)); // sha correct mais id inventé
    let (ds, ms) = stores(&d.join("dest"));
    assert!(import(&p, &ds, &ms, LIMITS).is_err());
}

/// Fabrique un thème `.purergb-theme` à partir d'une vidéo/GIF/image et d'un modèle de design
/// (outil manuel, ignoré par défaut) :
/// `PURERGB_THEME_MEDIA=<fichier> PURERGB_THEME_TEMPLATE=<design.json avec MEDIA_ID> PURERGB_THEME_OUT=<x.purergb-theme>
///  cargo test make_theme_from_env -- --ignored --nocapture`
#[test]
#[ignore]
fn make_theme_from_env() {
    let var = |k: &str| std::env::var(k).unwrap_or_else(|_| panic!("variable {k} manquante"));
    let (media_path, template, out) = (var("PURERGB_THEME_MEDIA"), var("PURERGB_THEME_TEMPLATE"), var("PURERGB_THEME_OUT"));
    let dir = tmp("make");
    let (_designs, media) = stores(&dir);
    let item = media.import(Path::new(&media_path)).expect("import du média");
    eprintln!("média importé : {item:?}");
    let json = std::fs::read_to_string(&template).expect("modèle").replace("MEDIA_ID", &item.id);
    let layout: Layout = serde_json::from_str(&json).expect("design valide");
    export(&layout, &media, Path::new(&out)).expect("export du thème");
    // Aperçu PNG (valeurs plausibles) à côté du thème, pour contrôle visuel.
    let store = crate::screens::bindings::ValueStore::empty();
    for (k, v) in [("cpu.temp", 58.0), ("cpu.load", 42.0), ("gpu.temp", 71.0), ("gpu.load", 63.0), ("ram.used_pct", 61.0), ("ram.used_gb", 19.5)] {
        store.set_for_test(k, v);
    }
    let frame = crate::screens::render::render(&layout, &store, &media, layout.width, layout.height, 0);
    if let Some(img) = image::RgbaImage::from_raw(frame.width, frame.height, frame.rgba) {
        let _ = img.save(format!("{out}.png"));
    }
    eprintln!("thème écrit : {out} ({} octets)", std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0));
    let _ = std::fs::remove_dir_all(&dir);
}
