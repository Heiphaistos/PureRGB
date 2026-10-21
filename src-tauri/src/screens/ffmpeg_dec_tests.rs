use super::*;
use parking_lot::Mutex;
use std::io::Cursor;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;

// Sorties réelles de `ffmpeg -hide_banner -i` (ffmpeg 9.0.2), chemins retirés.
const MP4: &str = "Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'a.mp4':
  Metadata:
    major_brand     : isom
    encoder         : Lavf63.1.102
  Duration: 00:00:08.00, start: 0.000000, bitrate: 4808 kb/s
  Stream #0:0[0x1](und): Video: h264 (High) (avc1 / 0x31637661), yuv420p(tv, smpte170m/bt709/iec61966-2-1, progressive), 1920x1080, 4804 kb/s, 24 fps, 24 tbr, 12288 tbn (default)
    Metadata:
      handler_name    : VideoHandler
At least one output file must be specified";
const ROTATED: &str = "Input #0, mov,mp4,m4a,3gp,3g2,mj2, from 'rot.mp4':
  Duration: 00:00:01.08, start: 0.000000, bitrate: 6899 kb/s
  Stream #0:0[0x1](und): Video: h264 (High) (avc1 / 0x31637661), yuv420p(tv, progressive), 1920x1080, 6890 kb/s, 24 fps, 24 tbr, 12288 tbn (default)
    Metadata:
      handler_name    : VideoHandler
    Side data:
      Display Matrix: rotation of 90.00 degrees";
const RAW_NA: &str = "Input #0, h264, from 'raw.h264':
  Duration: N/A, bitrate: N/A
  Stream #0:0: Video: h264 (High), yuv420p(tv, smpte170m/bt709/iec61966-2-1, progressive), 1920x1080, 25 fps, 48 tbr, 1200k tbn";
// Formats d'autres versions de ffmpeg (échantillons de la documentation / sorties classiques).
const AVI_DIV: &str = "Input #0, avi, from 'v.avi':
  Duration: 01:02:03.50, start: 0.000000, bitrate: 1200 kb/s
  Stream #0:0: Video: mpeg4 (Simple Profile) (XVID / 0x44495658), yuv420p, 640x272 [SAR 1:1 DAR 40:17], 29.97 fps, 29.97 tbr, 29.97 tbn
  Stream #0:1: Audio: mp3 (U[0][0][0] / 0x0055), 48000 Hz, stereo, fltp, 128 kb/s";
const OLD_ROTATE: &str = "  Duration: 00:00:10.00, start: 0.000000, bitrate: 900 kb/s
    Stream #0:0(und): Video: h264 (Main), yuv420p, 1280x720, 800 kb/s, 30 tbr, 90k tbn, 180k tbc (default)
    Metadata:
      rotate          : 270
    Stream #0:1(und): Audio: aac, 44100 Hz, stereo, fltp, 96 kb/s (default)";
const COVER_THEN_VIDEO: &str = "  Duration: 00:03:00.00, start: 0.000000, bitrate: 320 kb/s
  Stream #0:0: Audio: mp3, 44100 Hz, stereo, fltp, 320 kb/s
  Stream #0:1: Video: mjpeg (Baseline), yuvj420p(pc), 500x500, 90k tbr, 90k tbn (attached pic)
  Stream #0:2: Video: h264 (High), yuv420p, 320x240, 15 fps, 15 tbr, 1k tbn";

#[test]
fn parses_real_ffmpeg_output() {
    let i = parse_probe(MP4).unwrap();
    assert_eq!((i.width, i.height, i.duration_ms, i.fps), (1920, 1080, 8000, 24.0));
    let i = parse_probe(RAW_NA).unwrap();
    assert_eq!((i.width, i.height, i.duration_ms, i.fps), (1920, 1080, 0, 25.0), "durée N/A -> 0");
    let i = parse_probe(AVI_DIV).unwrap();
    assert_eq!((i.width, i.height, i.duration_ms), (640, 272, 3_723_500));
    assert!((i.fps - 29.97).abs() < 1e-3);
}

#[test]
fn rotation_swaps_display_size() {
    let i = parse_probe(ROTATED).unwrap();
    assert_eq!((i.width, i.height), (1080, 1920));
    let i = parse_probe(OLD_ROTATE).unwrap();
    assert_eq!((i.width, i.height, i.fps), (720, 1280, 30.0), "ancien format `rotate : 270`, fps = tbr");
    // Une rotation de la piste suivante ne s'applique pas à la première.
    let two = format!("{MP4}\n  Stream #0:1: Video: h264, 100x100, 10 fps\n    Side data:\n      Display Matrix: rotation of 90.00 degrees");
    assert_eq!(parse_probe(&two).unwrap().width, 1920);
}

#[test]
fn skips_cover_art_and_refuses_non_video() {
    let i = parse_probe(COVER_THEN_VIDEO).unwrap();
    assert_eq!((i.width, i.height, i.fps), (320, 240, 15.0));
    assert!(parse_probe("  Duration: 00:03:00.00\n  Stream #0:0: Audio: mp3, 44100 Hz").unwrap_err().to_string().contains("aucune piste vidéo"));
    assert!(parse_probe("x.mp4: Invalid data found when processing input").unwrap_err().to_string().contains("ne reconnaît pas"));
    assert!(parse_probe("").is_err());
    // Un titre de métadonnées piégé n'est pas pris pour une piste ou une durée.
    let evil = "    title : Stream #0:0: Video: h264, 99x99, 5 fps\n    comment : Duration: 99:00:00.00,\n  Duration: 00:00:01.00, x\n  Stream #0:0: Video: h264, 64x48, 30 fps";
    let i = parse_probe(evil).unwrap();
    assert_eq!((i.width, i.height, i.duration_ms), (64, 48, 1000));
}

#[test]
fn refuses_oversized_and_hex_tag_is_not_a_size() {
    let big = "  Stream #0:0: Video: hevc, yuv420p, 8192x4320, 30 fps";
    assert!(parse_probe(big).unwrap_err().to_string().contains("trop grande"));
    let tag_only = "  Stream #0:0: Video: h264 (avc1 / 0x31637661), yuv420p, 30 fps";
    assert!(parse_probe(tag_only).is_err(), "0x31637661 n'est pas une taille");
}

#[test]
fn output_size_and_fps_are_bounded() {
    assert_eq!(out_size(1280, 720), (1280, 720));
    assert_eq!(out_size(3840, 2160), (1920, 1080));
    assert_eq!(out_size(1080, 3840), (303, 1080), "ratio conservé");
    assert_eq!(out_fps(0.0), 30.0);
    assert_eq!(out_fps(240.0), 60.0);
    assert_eq!(out_fps(29.97), 29.97);
}

fn tmp_video(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("purergb_ffd_{name}_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    let f = d.join("clip & test;x.mp4");
    std::fs::write(&f, b"x").unwrap();
    f
}

#[test]
fn path_validation_and_argument_array() {
    assert!(validate_path(Path::new("relatif.mp4")).is_err());
    assert!(validate_path(Path::new("-i")).is_err());
    let f = tmp_video("args");
    assert!(validate_path(&f.with_extension("exe")).is_err());
    assert!(validate_path(f.parent().unwrap()).is_err());
    let info = VideoInfo { width: 3840, height: 2160, duration_ms: 1000, fps: 24.0 };
    let src = FfmpegSource::open(Arc::new(Fake::new(1)), &f, info).unwrap();
    let a: Vec<String> = src.args.iter().map(|a| a.to_string_lossy().into_owned()).collect();
    assert_eq!(a.iter().filter(|x| x.contains("clip & test;x")).count(), 1, "chemin = un seul argument, intact");
    for want in ["-an", "pipe:1", "bgra", "rawvideo", "scale=1920:1080:flags=bilinear", "-nostdin", "file"] {
        assert!(a.iter().any(|x| x == want), "{want} manquant : {a:?}");
    }
    let r = a.iter().position(|x| x == "-r").unwrap();
    assert_eq!(a[r + 1], "24");
}

struct Counters {
    spawns: AtomicUsize,
    kills: AtomicUsize,
}

/// Faux ffmpeg : sert `frames` images 2x2 (rouge 10, 20, 30...) à chaque lancement, ou bloque (`stall`) jusqu'au kill.
struct Fake {
    frames: usize,
    stall: bool,
    c: Arc<Counters>,
    sizes: Mutex<Vec<usize>>,
}

impl Fake {
    fn new(frames: usize) -> Fake {
        Fake { frames, stall: false, c: Arc::new(Counters { spawns: AtomicUsize::new(0), kills: AtomicUsize::new(0) }), sizes: Mutex::new(vec![]) }
    }
}

struct FakeKill(Arc<Counters>, Option<mpsc::Sender<()>>);
impl Killer for FakeKill {
    fn kill(&mut self) {
        if self.1.take().is_some() {
            self.0.kills.fetch_add(1, Ordering::SeqCst);
        }
    }
}

/// Lit jusqu'à la disparition de l'émetteur (= kill), puis fin de flux.
struct Stall(mpsc::Receiver<()>);
impl Read for Stall {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        let _ = self.0.recv();
        Ok(0)
    }
}

impl Spawner for Fake {
    fn spawn(&self, args: &[OsString]) -> std::io::Result<Spawned> {
        self.c.spawns.fetch_add(1, Ordering::SeqCst);
        self.sizes.lock().push(args.len());
        let (tx, rx) = mpsc::channel();
        let out: Box<dyn Read + Send> = if self.stall {
            Box::new(Stall(rx))
        } else {
            let data: Vec<u8> = (1..=self.frames).flat_map(|i| [0, 0, i as u8 * 10, 255].repeat(4)).collect(); // BGRA 2x2
            Box::new(Cursor::new(data))
        };
        Ok(Spawned { out, killer: Box::new(FakeKill(self.c.clone(), Some(tx))) })
    }
}

fn source(fake: &Arc<Fake>, path: &Path) -> FfmpegSource {
    FfmpegSource::open(fake.clone(), path, VideoInfo { width: 2, height: 2, duration_ms: 100, fps: 25.0 }).unwrap()
}

#[test]
fn paces_loops_by_respawn_and_stops_cleanly() {
    let f = tmp_video("loop");
    let fake = Arc::new(Fake::new(3));
    let mut s = source(&fake, &f);
    for pass in 0..2 {
        let got: Vec<(u64, u8)> = std::iter::from_fn(|| s.next_frame().unwrap()).map(|d| (d.ts_ms, d.pixmap.data()[0])).collect();
        // BGRA -> RGBA : le rouge est l'octet 2 du fake, 10/20/30 ; 25 i/s = 40 ms.
        assert_eq!(got, [(0, 10), (40, 20), (80, 30)], "passage {pass}");
        s.rewind().unwrap();
    }
    assert_eq!(fake.c.spawns.load(Ordering::SeqCst), 2, "un processus par passage, relancé au retour au début");
    assert_eq!(fake.c.kills.load(Ordering::SeqCst), 2, "chaque processus tué");
    // Après rewind, la lecture suivante relance ; Drop tue le dernier processus.
    assert!(s.next_frame().unwrap().is_some());
    assert_eq!(fake.c.spawns.load(Ordering::SeqCst), 3);
    drop(s);
    assert_eq!(fake.c.kills.load(Ordering::SeqCst), 3, "Drop tue le processus en cours");
}

#[test]
fn empty_stream_ends_without_frames_and_truncated_frame_is_dropped() {
    let f = tmp_video("empty");
    let fake = Arc::new(Fake::new(0));
    let mut s = source(&fake, &f);
    assert!(s.next_frame().unwrap().is_none());
    let fake = Arc::new(Fake::new(2));
    let mut s = source(&fake, &f);
    assert_eq!(std::iter::from_fn(|| s.next_frame().unwrap()).count(), 2);
}

#[test]
fn frame_cap_per_pass_ends_the_pass_and_kills() {
    let f = tmp_video("cap");
    let fake = Arc::new(Fake::new(10));
    let mut s = source(&fake, &f);
    s.max_frames = 4;
    assert_eq!(std::iter::from_fn(|| s.next_frame().unwrap()).count(), 4);
    assert_eq!(fake.c.kills.load(Ordering::SeqCst), 1);
}

#[test]
fn stalled_process_times_out_and_is_killed() {
    let f = tmp_video("stall");
    let mut fk = Fake::new(1);
    fk.stall = true;
    let fake = Arc::new(fk);
    let mut s = source(&fake, &f);
    s.read_timeout = Duration::from_millis(150);
    let t = std::time::Instant::now();
    let e = s.next_frame().err().expect("erreur de délai");
    assert!(e.to_string().contains("délai"), "{e}");
    assert!(t.elapsed() < Duration::from_secs(3));
    assert_eq!(fake.c.kills.load(Ordering::SeqCst), 1, "processus bloqué tué");
}

/// Test réel : `PURERGB_TEST_FFMPEG_EXE` (ffmpeg.exe extrait) + `PURERGB_TEST_VIDEO` (mp4/avi en lecture seule).
#[test]
#[ignore = "nécessite ffmpeg.exe et une vidéo réelle (variables d'environnement)"]
fn real_ffmpeg_reads_a_real_video() {
    use std::time::Instant;
    let (Some(exe), Some(video)) = (std::env::var_os("PURERGB_TEST_FFMPEG_EXE"), std::env::var_os("PURERGB_TEST_VIDEO")) else { return };
    let (exe, video) = (PathBuf::from(exe), PathBuf::from(video));
    let t = Instant::now();
    let info = probe_file(&exe, &video).expect("sonde");
    eprintln!("sonde : {info:?} en {:?}", t.elapsed());
    assert!(info.width > 0 && info.height > 0);
    let mut s = FfmpegSource::open(Arc::new(ExeSpawner::new(exe)), &video, info).expect("ouverture");
    let (w, h) = out_size(info.width, info.height);
    let t = Instant::now();
    let f1 = s.next_frame().expect("1re image").expect("image");
    eprintln!("1re image {}x{} en {:?}", f1.pixmap.width(), f1.pixmap.height(), t.elapsed());
    assert_eq!((f1.pixmap.width(), f1.pixmap.height()), (w, h));
    assert!(f1.pixmap.data().iter().any(|b| *b > 8 && *b != 255), "première image noire");
    let t = Instant::now();
    let (mut n, mut last_ts, mut changed) = (1u32, f1.ts_ms, 0u32);
    let mut prev = f1.pixmap.data().to_vec();
    while let Some(d) = s.next_frame().expect("image") {
        if d.pixmap.data() != prev.as_slice() {
            changed += 1;
            prev = d.pixmap.data().to_vec();
        }
        (n, last_ts) = (n + 1, d.ts_ms);
    }
    let el = t.elapsed();
    eprintln!("passage 1 : {n} images (dernier ts {last_ts} ms, {changed} différentes) en {el:?} = {:.0} i/s décodées", f64::from(n) / el.as_secs_f64());
    assert!(changed > 0, "la vidéo ne bouge pas");
    s.rewind().expect("rewind");
    let t = Instant::now();
    let f = s.next_frame().expect("boucle").expect("image de la boucle");
    eprintln!("boucle : 1re image du 2e passage en {:?} (ts {})", t.elapsed(), f.ts_ms);
    assert_eq!(f.ts_ms, 0);
    drop(s);
}
