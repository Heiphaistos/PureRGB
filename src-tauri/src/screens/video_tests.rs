use super::*;
use std::sync::atomic::AtomicUsize;

fn red_of(p: &Pixmap) -> u8 {
    p.data()[0]
}

#[test]
fn bgrx_converts_top_down_and_bottom_up() {
    // 2x2 : ligne 0 = bleu, rouge ; ligne 1 = vert, blanc (BGRX, X volontairement à 0).
    let rows: [[u8; 8]; 2] = [[255, 0, 0, 0, 0, 0, 255, 0], [0, 255, 0, 0, 255, 255, 255, 0]];
    let top_down: Vec<u8> = rows.concat();
    let pm = bgrx_to_pixmap(2, 2, 8, &top_down, (2, 2)).expect("top-down");
    assert_eq!(&pm.data()[0..4], &[0, 0, 255, 255], "bleu en (0,0), alpha forcé à 255");
    assert_eq!(&pm.data()[4..8], &[255, 0, 0, 255]);
    let bottom_up: Vec<u8> = [rows[1], rows[0]].concat();
    let pm2 = bgrx_to_pixmap(2, 2, -8, &bottom_up, (2, 2)).expect("bottom-up");
    assert_eq!(pm.data(), pm2.data());
    // Pas plus grand que la ligne (rembourrage) accepté, tampon trop court refusé.
    assert!(bgrx_to_pixmap(2, 2, 12, &[0u8; 20], (2, 2)).is_some());
    assert!(bgrx_to_pixmap(2, 2, 8, &[0u8; 15], (2, 2)).is_none());
    assert!(bgrx_to_pixmap(2, 2, 4, &[0u8; 64], (2, 2)).is_none());
    // Recadrage : coin haut-gauche (le rembourrage est en bas/à droite), aussi en bas-en-haut.
    let c = bgrx_to_pixmap(2, 2, 8, &top_down, (1, 1)).expect("crop");
    assert_eq!((c.width(), c.height(), c.data()), (1, 1, &[0u8, 0, 255, 255][..]));
    let c2 = bgrx_to_pixmap(2, 2, -8, &bottom_up, (1, 1)).expect("crop bas-en-haut");
    assert_eq!(c.data(), c2.data());
}

struct Counters {
    opened: AtomicUsize,
    dropped: AtomicUsize,
    rewinds: AtomicUsize,
}

/// `n` images de 40 ms (rouge 10, 20, 30...), puis fin de flux.
struct Fake {
    i: u64,
    n: u64,
    c: Arc<Counters>,
}

impl FrameSource for Fake {
    fn next_frame(&mut self) -> Result<Option<Decoded>> {
        if self.i >= self.n {
            return Ok(None);
        }
        self.i += 1;
        let mut pixmap = Pixmap::new(2, 2).ok_or_else(|| anyhow!("pixmap"))?;
        pixmap.fill(tiny_skia::Color::from_rgba8(self.i as u8 * 10, 0, 0, 255));
        Ok(Some(Decoded { pixmap, ts_ms: (self.i - 1) * 40 }))
    }
    fn rewind(&mut self) -> Result<()> {
        self.c.rewinds.fetch_add(1, Ordering::SeqCst);
        self.i = 0;
        Ok(())
    }
}

impl Drop for Fake {
    fn drop(&mut self) {
        self.c.dropped.fetch_add(1, Ordering::SeqCst);
    }
}

fn fake_player(n: u64, idle: Duration) -> (VideoPlayer, Arc<Counters>) {
    let c = Arc::new(Counters { opened: AtomicUsize::new(0), dropped: AtomicUsize::new(0), rewinds: AtomicUsize::new(0) });
    let c2 = c.clone();
    let factory: Factory = Arc::new(move || {
        c2.opened.fetch_add(1, Ordering::SeqCst);
        Ok(Box::new(Fake { i: 0, n, c: c2.clone() }) as Box<dyn FrameSource>)
    });
    (VideoPlayer::with_factory(factory, idle), c)
}

fn wait_for(cond: impl Fn() -> bool, ms: u64) -> bool {
    let end = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < end {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    cond()
}

#[test]
fn lazy_start_paced_and_looping() {
    let (p, c) = fake_player(3, Duration::from_secs(5));
    assert_eq!(c.opened.load(Ordering::SeqCst), 0, "rien ne démarre avant la première demande");
    let t0 = Instant::now();
    let first = p.frame().expect("première image");
    assert_eq!(red_of(&first), 10);
    let mut seen = vec![10u8];
    while t0.elapsed() < Duration::from_millis(500) {
        if let Some(f) = p.frame() {
            if seen.last() != Some(&red_of(&f)) {
                seen.push(red_of(&f));
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    // 500 ms / 120 ms par boucle : au moins une boucle complète 10,20,30,10 ; ordre respecté.
    assert!(seen.len() >= 4, "pas de boucle : {seen:?}");
    assert!(seen.windows(2).all(|w| w[1] == w[0] + 10 || (w[0] == 30 && w[1] == 10)), "ordre : {seen:?}");
    assert!(c.rewinds.load(Ordering::SeqCst) >= 1);
    assert_eq!(c.opened.load(Ordering::SeqCst), 1, "un seul lecteur ouvert");
    // Cadence : en 500 ms avec 40 ms/image, ~12 changements, pas des centaines.
    assert!(seen.len() <= 20, "cadence trop rapide : {seen:?}");
}

#[test]
fn stops_when_idle_and_restarts_from_the_beginning() {
    let (p, c) = fake_player(3, Duration::from_millis(150));
    assert!(p.frame().is_some());
    assert!(wait_for(|| !p.shared.running.load(Ordering::SeqCst), 1500), "doit s'arrêter seul");
    assert_eq!(c.dropped.load(Ordering::SeqCst), 1, "source libérée");
    assert!(p.shared.slot.lock().frame.is_none(), "image libérée");
    let again = p.frame().expect("reprise");
    assert_eq!(red_of(&again), 10, "la reprise repart de la première image");
    assert_eq!(c.opened.load(Ordering::SeqCst), 2);
}

#[test]
fn drop_joins_the_thread_and_releases_the_source() {
    let (p, c) = fake_player(3, Duration::from_secs(5));
    assert!(p.frame().is_some());
    drop(p);
    assert_eq!(c.dropped.load(Ordering::SeqCst), 1);
}

#[test]
fn empty_stream_and_open_error_are_reported_without_spinning() {
    let (p, _) = fake_player(0, Duration::from_secs(5));
    assert!(p.frame().is_none());
    assert!(wait_for(|| p.last_error().is_some(), 1000));
    assert!(p.last_error().is_some_and(|e| e.contains("aucune image")));

    let factory: Factory = Arc::new(|| Err(anyhow!("boom")));
    let q = VideoPlayer::with_factory(factory, Duration::from_secs(5));
    assert!(q.frame().is_none());
    assert!(wait_for(|| q.last_error().is_some(), 1000));
}

#[test]
fn fake_file_is_rejected_in_french() {
    let dir = std::env::temp_dir().join(format!("purergb-video-bad-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let f = dir.join("faux.mp4");
    std::fs::write(&f, b"ceci n'est pas une video").expect("écriture");
    let e = VideoPlayer::open(&f).err().map(|e| format!("{e:#}")).unwrap_or_default();
    assert!(e.contains("non pris en charge par Windows") || e.contains("aucune piste"), "message : {e}");
    assert!(VideoPlayer::open(&dir.join("absent.mp4")).is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

/// AVI non compressé (RGB24) fait main : 5 images 32x24 dont le rouge vaut 0, 50, 100, 150, 200.
pub(crate) fn tiny_avi() -> Vec<u8> {
    fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut v = id.to_vec();
        v.extend((body.len() as u32).to_le_bytes());
        v.extend(body);
        if body.len() % 2 == 1 {
            v.push(0);
        }
        v
    }
    fn list(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut inner = kind.to_vec();
        inner.extend(body);
        chunk(b"LIST", &inner)
    }
    let (w, h, n) = (32u32, 24u32, 5u32);
    let frame_size = w * h * 3; // 96 octets par ligne, multiple de 4
    let u = |v: u32| v.to_le_bytes();
    let mut avih = Vec::new();
    for v in [100_000u32, 0, 0, 0x10, n, 0, 1, frame_size, w, h, 0, 0, 0, 0] {
        avih.extend(u(v));
    }
    let mut strh = b"vidsDIB ".to_vec();
    for v in [0u32, 0, 0, 1, 10, 0, n, frame_size, 0xFFFF_FFFF, 0] {
        strh.extend(u(v));
    }
    strh.extend([0u8; 8]); // rcFrame
    let mut strf = Vec::new();
    for v in [40u32, w, h] {
        strf.extend(u(v));
    }
    strf.extend(1u16.to_le_bytes());
    strf.extend(24u16.to_le_bytes());
    for v in [0u32, frame_size, 0, 0, 0, 0] {
        strf.extend(u(v));
    }
    let strl = list(b"strl", &[chunk(b"strh", &strh), chunk(b"strf", &strf)].concat());
    let hdrl = list(b"hdrl", &[chunk(b"avih", &avih), strl].concat());
    let mut movi_body = Vec::new();
    let mut idx = Vec::new();
    for i in 0..n {
        let px = [0u8, 0, (i * 50) as u8]; // BGR : rouge = i*50
        let data: Vec<u8> = px.iter().copied().cycle().take(frame_size as usize).collect();
        idx.extend(b"00db");
        idx.extend(u(0x10));
        idx.extend(u(4 + movi_body.len() as u32));
        idx.extend(u(frame_size));
        movi_body.extend(chunk(b"00db", &data));
    }
    let movi = list(b"movi", &movi_body);
    let mut riff_body = b"AVI ".to_vec();
    riff_body.extend([hdrl, movi, chunk(b"idx1", &idx)].concat());
    chunk(b"RIFF", &riff_body)
}

#[cfg(windows)]
#[test]
fn real_decode_of_handmade_avi() {
    let dir = std::env::temp_dir().join(format!("purergb-video-avi-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let f = dir.join("mini.avi");
    std::fs::write(&f, tiny_avi()).expect("écriture");
    match VideoPlayer::open(&f) {
        Err(e) => eprintln!("AVI fait main refusé par Windows : {e:#}"),
        Ok(p) => {
            let info = probe(&f).expect("sonde");
            eprintln!("AVI fait main : {info:?}");
            assert_eq!((info.width, info.height), (32, 24));
            let fr = p.frame().expect("première image");
            assert_eq!((fr.width(), fr.height()), (32, 24));
            assert_eq!(fr.data()[3], 255);
            let reds: Vec<u8> = (0..8)
                .filter_map(|_| {
                    std::thread::sleep(Duration::from_millis(60));
                    p.frame().map(|f| f.data()[0])
                })
                .collect();
            eprintln!("rouges observés : {reds:?}");
            assert!(reds.iter().all(|r| r % 50 == 0), "couleurs inattendues : {reds:?}");
            p.shutdown();
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Vraie vidéo trouvée sur la machine (lecture seule) ; ignoré s'il n'y en a pas.
#[cfg(windows)]
#[test]
fn real_decode_of_a_video_found_on_this_machine() {
    // Fournir un fichier avec PURERGB_TEST_VIDEO pour exécuter ce test sur une vraie vidéo.
    let candidates = [std::env::var("PURERGB_TEST_VIDEO").unwrap_or_default()];
    let Some(path) = candidates.iter().map(PathBuf::from).find(|p| p.is_file()) else {
        eprintln!("aucune vidéo de test sur cette machine : test ignoré");
        return;
    };
    let t = Instant::now();
    let p = match VideoPlayer::open(&path) {
        Ok(p) => p,
        Err(e) => panic!("ouverture de {} : {e:#}", path.display()),
    };
    let info = probe(&path).expect("sonde");
    eprintln!("{} -> {info:?} (ouverture {:?})", path.display(), t.elapsed());
    assert!(info.width > 0 && info.height > 0 && info.duration_ms > 0);
    let t = Instant::now();
    let f1 = p.frame().expect("première image");
    eprintln!("1re image en {:?} : {}x{}", t.elapsed(), f1.width(), f1.height());
    assert!(f1.width() <= MAX_OUT.0 && f1.height() <= MAX_OUT.1);
    let sum: u64 = f1.data().iter().map(|b| u64::from(*b)).sum();
    assert!(sum > 0, "première image entièrement noire");
    let mut distinct = 1;
    let mut last = f1.data().to_vec();
    let t = Instant::now();
    while t.elapsed() < Duration::from_millis(1200) {
        if let Some(f) = p.frame() {
            if f.data() != last.as_slice() {
                distinct += 1;
                last = f.data().to_vec();
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    eprintln!("{distinct} images distinctes observées en 1,2 s (fps natif {:.1})", info.fps);
    assert!(distinct >= 2, "la vidéo ne bouge pas");
    p.shutdown();
}
