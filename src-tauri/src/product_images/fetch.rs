//! Téléchargement de la photo d'un produit : liste blanche d'hôtes (https seulement, jamais d'IP ni
//! de localhost, chaque redirection revérifiée), image décodée puis ré-encodée, cache disque 30 j,
//! échec mémorisé 24 h. Client : curl.exe (arguments, aucun shell), comme `trust.rs`.

use super::catalog::Product;
use super::imgproc;
use crate::trust::https_host;
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// CDN d'images connus, autorisés EN PLUS du domaine du produit (hôte exact ou sous-domaine).
/// Un redirect vers autre chose est refusé. Ajouter ici uniquement un CDN public de confiance.
const KNOWN_CDNS: &[&str] = &[
    "cdn.shopify.com",
    "images.ctfassets.net",
    "cloudfront.net",
    "cdn.sanity.io",
    "res.cloudinary.com",
    "images.prismic.io",
    "assets.corsair.com",
    "cdn.nzxt.com",
];

const TTL_IMAGE_SECS: u64 = 30 * 24 * 3600;
const TTL_FAIL_SECS: u64 = 24 * 3600;
const MAX_REDIRECTS: usize = 3;
const TIMEOUT_SECS: u32 = 20;

/// Hôte nominatif : jamais une IP (v4 y compris formes courtes), `localhost` ni hôte sans point.
pub(super) fn host_is_name(host: &str) -> bool {
    let h = host.to_ascii_lowercase();
    let last = h.rsplit('.').next().unwrap_or("");
    h.contains('.')
        && h != "localhost"
        && !h.ends_with(".localhost")
        && !h.starts_with(['.', '-'])
        && !h.ends_with(['.', '-'])
        && !h.contains("..")
        && h.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
        && last.chars().any(|c| c.is_ascii_alphabetic())
}

fn covers(domain: &str, host: &str) -> bool {
    host == domain || host.ends_with(&format!(".{domain}"))
}

/// Domaines autorisés pour ce produit : ceux de son catalogue + CDN connus.
fn allowed_domains(p: &Product) -> Vec<String> {
    let mut v: Vec<String> = KNOWN_CDNS.iter().map(|s| s.to_string()).collect();
    if let Some(i) = &p.image {
        for u in [&i.url, &i.source_page] {
            if let Some(h) = https_host(u) {
                v.push(h.to_ascii_lowercase().trim_start_matches("www.").to_string());
            }
        }
    }
    v
}

fn url_allowed(url: &str, domains: &[String]) -> Result<()> {
    let host = https_host(url).map(str::to_ascii_lowercase);
    match host {
        Some(h) if host_is_name(&h) && domains.iter().any(|d| covers(d, &h)) => Ok(()),
        Some(h) => bail!("hôte non autorisé pour la photo : {h}"),
        None => bail!("URL de photo refusée (https avec nom d'hôte requis)"),
    }
}

pub(super) struct HopResp {
    pub code: u16,
    pub content_type: String,
    pub redirect: Option<String>,
}

/// Une requête HTTP SANS suivre les redirections ; le corps est écrit dans `dest`.
pub(super) trait Hop {
    fn hop(&self, url: &str, dest: &Path) -> Result<HopResp>;
}

struct CurlHop;

impl Hop for CurlHop {
    fn hop(&self, url: &str, dest: &Path) -> Result<HopResp> {
        let mut cmd = Command::new("curl.exe");
        cmd.args(["-sS", "--proto", "=https", "--max-redirs", "0", "-A", "PureRGB", "--max-time"])
            .arg(TIMEOUT_SECS.to_string())
            .arg("--max-filesize")
            .arg(imgproc::MAX_BYTES.to_string())
            .args(["-w", "%{http_code}|%{content_type}|%{redirect_url}", "-o"])
            .arg(dest)
            .arg(url)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        let out = cmd.output().context("lancement de curl.exe")?;
        if !out.status.success() {
            bail!("téléchargement échoué : {}", String::from_utf8_lossy(&out.stderr).trim());
        }
        let text = String::from_utf8_lossy(&out.stdout);
        let mut it = text.splitn(3, '|');
        let code = it.next().and_then(|c| c.trim().parse().ok()).context("réponse HTTP illisible")?;
        let content_type = it.next().unwrap_or("").trim().to_ascii_lowercase();
        let redirect = it.next().map(str::trim).filter(|r| !r.is_empty()).map(String::from);
        Ok(HopResp { code, content_type, redirect })
    }
}

/// Télécharge `url` en revérifiant la liste blanche à chaque redirection ; renvoie les octets
/// (≤ 4 Mo, Content-Type image/*) — non encore validés comme image.
fn download(hop: &dyn Hop, url: &str, domains: &[String], tmp: &Path) -> Result<Vec<u8>> {
    let mut cur = url.to_string();
    for _ in 0..=MAX_REDIRECTS {
        url_allowed(&cur, domains)?;
        let r = hop.hop(&cur, tmp)?;
        if (300..400).contains(&r.code) {
            cur = r.redirect.context("redirection sans destination")?;
            continue;
        }
        if r.code != 200 {
            bail!("le site a répondu HTTP {}", r.code);
        }
        if !r.content_type.starts_with("image/") {
            bail!("le contenu reçu n'est pas une image ({})", r.content_type);
        }
        let len = std::fs::metadata(tmp).context("fichier reçu introuvable")?.len();
        if len > imgproc::MAX_BYTES {
            bail!("image trop volumineuse (plus de 4 Mo)");
        }
        return std::fs::read(tmp).context("lecture du fichier reçu");
    }
    bail!("trop de redirections")
}

fn sha256_hex(s: &str) -> String {
    Sha256::digest(s.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn mtime_secs(p: &Path) -> Option<u64> {
    let t = std::fs::metadata(p).ok()?.modified().ok()?;
    t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs())
}

fn fresh(stamp: u64, now: u64, ttl: u64) -> bool {
    now.saturating_sub(stamp) < ttl
}

/// Cœur testable : dossier de cache, horloge (`now`, secondes Unix) et client injectés.
/// Le chemin de sortie dépend uniquement du SHA-256 de l'URL du catalogue.
pub(super) fn fetch_in(dir: &Path, now: u64, p: &Product, hop: &dyn Hop) -> Result<PathBuf> {
    let img = p.image.as_ref().context("aucune photo connue pour ce produit")?;
    let hash = sha256_hex(&img.url);
    let png = dir.join(format!("{hash}.png"));
    let fail = dir.join(format!("{hash}.fail"));
    let stale = png.is_file();
    if stale && mtime_secs(&png).is_some_and(|t| fresh(t, now, TTL_IMAGE_SECS)) {
        return Ok(png);
    }
    let failed_recently = std::fs::read_to_string(&fail)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .is_some_and(|t| fresh(t, now, TTL_FAIL_SECS));
    if failed_recently {
        return if stale { Ok(png) } else { bail!("photo indisponible (nouvel essai dans 24 h au plus)") };
    }
    std::fs::create_dir_all(dir).context("création du dossier des photos")?;
    let tmp = dir.join(format!("{hash}.tmp"));
    let result = download(hop, &img.url, &allowed_domains(p), &tmp)
        .and_then(|bytes| imgproc::to_png(&bytes))
        .and_then(|out| {
            std::fs::write(&tmp, out).context("écriture de la photo")?;
            std::fs::rename(&tmp, &png).context("installation de la photo")
        });
    let _ = std::fs::remove_file(&tmp);
    match result {
        Ok(()) => {
            let _ = std::fs::remove_file(&fail);
            Ok(png)
        }
        Err(e) => {
            let _ = std::fs::write(&fail, now.to_string());
            log::warn!("photo produit {} : {e:#}", p.id);
            if stale {
                Ok(png)
            } else {
                Err(e)
            }
        }
    }
}

/// Photo PNG (≤ 640 px) du produit dans `%APPDATA%\PureRGB\product-images\`, téléchargée si besoin.
pub fn fetch_image(p: &Product) -> Result<PathBuf> {
    let dir = crate::settings::dirs_dir().context("dossier de données introuvable")?.join("product-images");
    fetch_in(&dir, now_secs(), p, &CurlHop)
}

#[cfg(test)]
mod tests {
    use super::super::catalog::{Image, Kind, Match};
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::io::Cursor;

    /// Client factice : url -> (code, type, redirect, corps) ; compte les appels.
    struct Fake {
        routes: HashMap<String, (u16, &'static str, Option<String>, Vec<u8>)>,
        calls: RefCell<Vec<String>>,
    }

    impl Hop for Fake {
        fn hop(&self, url: &str, dest: &Path) -> Result<HopResp> {
            self.calls.borrow_mut().push(url.to_string());
            let (code, ct, red, body) = self.routes.get(url).context("hors ligne")?.clone();
            std::fs::write(dest, body)?;
            Ok(HopResp { code, content_type: ct.into(), redirect: red })
        }
    }

    const URL: &str = "https://www.maker.com/img/a.png";

    fn product() -> Product {
        Product {
            id: "p".into(),
            kind: Kind::Fan,
            vendor: "Maker".into(),
            name: "Maker Fan".into(),
            matching: Match { usb: vec![], names: vec!["maker fan".into()] },
            image: Some(Image { url: URL.into(), source_page: "https://www.maker.com/p".into() }),
        }
    }

    fn png_bytes() -> Vec<u8> {
        let mut o = Vec::new();
        image::RgbaImage::from_pixel(900, 300, image::Rgba([1, 2, 3, 255]))
            .write_to(&mut Cursor::new(&mut o), image::ImageFormat::Png)
            .unwrap();
        o
    }

    fn fake(routes: Vec<(&str, (u16, &'static str, Option<&str>, Vec<u8>))>) -> Fake {
        Fake {
            routes: routes.into_iter().map(|(u, (c, t, r, b))| (u.to_string(), (c, t, r.map(String::from), b))).collect(),
            calls: RefCell::new(vec![]),
        }
    }

    fn tmpdir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("purergb_pi_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn host_rules() {
        for ok in ["maker.com", "cdn.maker.com", "a-b.co.uk"] {
            assert!(host_is_name(ok), "{ok}");
        }
        for bad in ["127.0.0.1", "localhost", "foo.localhost", "10.0.0.1", "127.1", "2130706433", "a..b.com", "-a.com", "a.com.", "exa_mple.com"] {
            assert!(!host_is_name(bad), "{bad}");
        }
    }

    #[test]
    fn allowlist_urls() {
        let d = allowed_domains(&product());
        assert!(url_allowed("https://www.maker.com/x.png", &d).is_ok());
        assert!(url_allowed("https://cdn.maker.com/x.png", &d).is_ok());
        assert!(url_allowed("https://cdn.shopify.com/x.png", &d).is_ok());
        for bad in [
            "http://www.maker.com/x.png",
            "https://evilmaker.com/x.png",
            "https://maker.com.evil.com/x.png",
            "https://127.0.0.1/x.png",
            "https://localhost/x.png",
            "https://maker.com@evil.com/x.png",
            "https://[::1]/x.png",
            "https://autre.org/x.png",
            "file:///C:/x.png",
        ] {
            assert!(url_allowed(bad, &d).is_err(), "{bad}");
        }
    }

    #[test]
    fn downloads_and_reencodes_into_hash_path() {
        let dir = tmpdir("ok");
        let f = fake(vec![(URL, (200, "image/png", None, png_bytes()))]);
        let out = fetch_in(&dir, 1_000, &product(), &f).unwrap();
        assert_eq!(out, dir.join(format!("{}.png", sha256_hex(URL))));
        assert_eq!(out.parent(), Some(dir.as_path()));
        let img = image::open(&out).unwrap();
        assert_eq!((img.width(), img.height()), (640, 213));
        assert!(!dir.join(format!("{}.tmp", sha256_hex(URL))).exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn redirect_to_unlisted_host_is_refused_and_not_followed() {
        let dir = tmpdir("redir");
        let f = fake(vec![
            (URL, (302, "", Some("https://evil.example.org/a.png"), vec![])),
            ("https://evil.example.org/a.png", (200, "image/png", None, png_bytes())),
        ]);
        assert!(fetch_in(&dir, 1_000, &product(), &f).is_err());
        assert_eq!(*f.calls.borrow(), vec![URL.to_string()], "la cible n'est jamais contactée");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn redirect_to_listed_cdn_is_followed_but_http_or_ip_is_not() {
        let dir = tmpdir("redir2");
        let cdn = "https://cdn.shopify.com/a.png";
        let f = fake(vec![(URL, (301, "", Some(cdn), vec![])), (cdn, (200, "image/png", None, png_bytes()))]);
        assert!(fetch_in(&dir, 1_000, &product(), &f).is_ok());
        for bad in ["http://www.maker.com/a.png", "https://127.0.0.1/a.png"] {
            let d2 = tmpdir("redir3");
            let f = fake(vec![(URL, (302, "", Some(bad), vec![]))]);
            assert!(fetch_in(&d2, 1_000, &product(), &f).is_err(), "{bad}");
            let _ = std::fs::remove_dir_all(&d2);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_wrong_content_type_status_and_non_image() {
        for (code, ct, body) in [
            (200u16, "text/html", png_bytes()),
            (404, "image/png", png_bytes()),
            (200, "image/png", b"MZ not an image".to_vec()),
            (200, "image/png", vec![0u8; (imgproc::MAX_BYTES + 1) as usize]),
        ] {
            let dir = tmpdir("bad");
            let f = fake(vec![(URL, (code, ct, None, body))]);
            assert!(fetch_in(&dir, 1_000, &product(), &f).is_err(), "{code} {ct}");
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn failure_is_remembered_24h_then_retried() {
        let dir = tmpdir("fail");
        let f = fake(vec![]);
        assert!(fetch_in(&dir, 1_000, &product(), &f).is_err());
        assert!(fetch_in(&dir, 1_000 + 3_600, &product(), &f).is_err());
        assert_eq!(f.calls.borrow().len(), 1, "pas de martelage pendant 24 h");
        assert!(fetch_in(&dir, 1_000 + TTL_FAIL_SECS + 1, &product(), &f).is_err());
        assert_eq!(f.calls.borrow().len(), 2, "nouvel essai après 24 h");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cache_hit_then_expiry_refreshes_and_keeps_stale_on_failure() {
        let dir = tmpdir("cache");
        let ok = fake(vec![(URL, (200, "image/png", None, png_bytes()))]);
        let now = now_secs();
        let p1 = fetch_in(&dir, now, &product(), &ok).unwrap();
        fetch_in(&dir, now + 10 * 86_400, &product(), &ok).unwrap();
        assert_eq!(ok.calls.borrow().len(), 1, "cache valide 30 j");
        fetch_in(&dir, now + 31 * 86_400, &product(), &ok).unwrap();
        assert_eq!(ok.calls.borrow().len(), 2, "expiré : retéléchargé");
        let down = fake(vec![]);
        let p2 = fetch_in(&dir, now + 90 * 86_400, &product(), &down).unwrap();
        assert_eq!(p1, p2, "échec réseau : l'ancienne photo reste servie");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn product_without_image_errors() {
        let mut p = product();
        p.image = None;
        assert!(fetch_in(&tmpdir("noimg"), 0, &p, &fake(vec![])).is_err());
    }

    #[test]
    fn output_path_never_depends_on_remote_data() {
        // Même avec un id ou une URL piégés, le nom de fichier est un hash hexadécimal.
        let h = sha256_hex("https://x.com/../../Windows/System32/evil.png");
        assert_eq!(h.len(), 64);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// Réseau réel (ignoré par défaut) : télécharge la photo de chaque produit du catalogue embarqué
    /// avec le vrai `curl.exe` et la validation complète ; `cargo test real_download -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_download_of_the_whole_catalog() {
        let trofeo = super::super::catalog::identify("Thermalright", "Trofeo Vision 9.16 LCD", &["0416:5408"]).expect("Trofeo identifié");
        let dir = std::env::temp_dir().join(format!("purergb-photo-real-{}", std::process::id()));
        let path = fetch_in(&dir, now_secs(), trofeo, &CurlHop).expect("photo du Trofeo");
        let img = image::open(&path).expect("PNG valide");
        eprintln!("Trofeo : {}x{} -> {}", img.width(), img.height(), path.display());
        assert!(img.width() <= 640 && img.height() <= 640 && img.width() >= 100);
        let mut failed = Vec::new();
        let all = super::super::catalog::all();
        for p in all {
            if p.image.is_none() {
                continue;
            }
            if let Err(e) = fetch_in(&dir, now_secs(), p, &CurlHop) {
                failed.push(format!("{} : {e:#}", p.id));
            }
        }
        eprintln!("{} échecs sur {} produits", failed.len(), all.len());
        for f in &failed {
            eprintln!("  {f}");
        }
        let _ = std::fs::remove_dir_all(&dir);
        assert!(failed.len() * 10 < all.len(), "trop d'échecs : {}", failed.len());
    }
}
