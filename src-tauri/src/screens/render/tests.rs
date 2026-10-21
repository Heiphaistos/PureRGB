use super::*;
use crate::screens::layout::Layout;
use serde_json::{json, Value};
use std::path::PathBuf;

fn layout(w: u32, h: u32, bg: &str, widgets: Value) -> Layout {
    serde_json::from_value(json!({
        "version": 1, "name": "t", "width": w, "height": h,
        "background": {"kind": "solid", "color": bg},
        "widgets": widgets,
    }))
    .expect("layout de test valide")
}

fn widget(id: &str, x: f32, y: f32, w: f32, h: f32, kind: Value) -> Value {
    let mut v = json!({"id": id, "x": x, "y": y, "w": w, "h": h});
    if let (Some(o), Some(k)) = (v.as_object_mut(), kind.as_object()) {
        o.extend(k.clone());
    }
    v
}

fn media() -> MediaStore {
    MediaStore::new(std::env::temp_dir().join(format!("purergb-render-media-{}", std::process::id())))
}

fn px(f: &Frame, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * f.width + x) * 4) as usize;
    [f.rgba[i], f.rgba[i + 1], f.rgba[i + 2], f.rgba[i + 3]]
}

fn close(a: [u8; 4], rgb: [u8; 3], tol: i32) -> bool {
    (0..3).all(|i| (a[i] as i32 - rgb[i] as i32).abs() <= tol)
}

fn lit(f: &Frame) -> usize {
    f.rgba.chunks_exact(4).filter(|p| p[0] > 40 || p[1] > 40 || p[2] > 40).count()
}

fn style(size: f32) -> Value {
    json!({"font": "Inter", "size": size, "color": "#ffffff", "align": "left", "bold": true})
}

const RAMP: [(f32, &str); 3] = [(0.0, "#00ff00"), (0.7, "#ffaa00"), (1.0, "#ff0000")];

#[test]
fn text_renders_inside_its_box_and_shrinks_to_fit() {
    let l = layout(200, 100, "#000000", json!([widget("t", 20.0, 20.0, 60.0, 40.0, json!({"type": "text", "text": "Température très longue", "style": style(30.0)}))]));
    let f = render(&l, &ValueStore::empty(), &media(), 200, 100, 0);
    assert!(lit(&f) > 100);
    for y in 0..100 {
        for x in 0..200 {
            if !(19..=81).contains(&x) || !(19..=61).contains(&y) {
                assert!(px(&f, x, y)[0] < 5, "pixel hors boîte allumé en ({x},{y})");
            }
        }
    }
}

#[test]
fn value_missing_shows_dashes_like_plain_text() {
    let st = ValueStore::empty();
    let a = layout(200, 60, "#000000", json!([widget("v", 0.0, 0.0, 200.0, 60.0, json!({"type": "value", "source": "cpu.temp", "format": "{:.0}", "style": style(40.0)}))]));
    let b = layout(200, 60, "#000000", json!([widget("t", 0.0, 0.0, 200.0, 60.0, json!({"type": "text", "text": "--", "style": style(40.0)}))]));
    let (fa, fb) = (render(&a, &st, &media(), 200, 60, 0), render(&b, &st, &media(), 200, 60, 0));
    assert!(lit(&fa) > 20);
    assert_eq!(fa.rgba, fb.rgba);
    st.set_for_test("cpu.temp", 64.0);
    assert_ne!(render(&a, &st, &media(), 200, 60, 0).rgba, fa.rgba);
}

#[test]
fn gauge_full_uses_ramp_end_and_empty_shows_track() {
    let kind = |src: &str| json!({"type": "gauge_arc", "source": src, "min": 0, "max": 100, "start_deg": 135, "sweep_deg": 270, "thickness": 20, "track": "#333333", "colors": RAMP});
    let l = layout(200, 200, "#000000", json!([widget("g", 0.0, 0.0, 200.0, 200.0, kind("gpu.load"))]));
    let st = ValueStore::empty();
    st.set_for_test("gpu.load", 100.0);
    let f = render(&l, &st, &media(), 200, 200, 0);
    assert!(close(px(&f, 100, 10), [255, 0, 0], 6), "sommet = fin de rampe : {:?}", px(&f, 100, 10));
    assert_eq!(px(&f, 100, 190), [0, 0, 0, 255], "le trou du bas reste vide");
    st.set_for_test("gpu.load", 0.0);
    let f = render(&l, &st, &media(), 200, 200, 0);
    assert!(close(px(&f, 100, 10), [0x33, 0x33, 0x33], 6));
    // 35 % : couleur interpolée entre vert et orange, tracé jusqu'à l'angle attendu (point à 135+94.5 deg).
    st.set_for_test("gpu.load", 35.0);
    let f = render(&l, &st, &media(), 200, 200, 0);
    let a = (135.0f32 + 270.0 * 0.35 - 5.0).to_radians();
    let p = px(&f, (100.0 + 90.0 * a.cos()) as u32, (100.0 + 90.0 * a.sin()) as u32);
    assert!(p[1] > 200 && p[0] > 100 && p[0] < 160, "{p:?}");
}

#[test]
fn bar_fills_proportionally_horizontal_and_vertical() {
    let kind = |v: bool| json!({"type": "bar", "source": "ram.used_pct", "min": 0, "max": 100, "vertical": v, "radius": 0, "track": "#222222", "colors": [[0.0, "#00ff00"], [1.0, "#00ff00"]]});
    let l = layout(100, 100, "#000000", json!([widget("b", 10.0, 10.0, 80.0, 20.0, kind(false)), widget("c", 10.0, 40.0, 20.0, 50.0, kind(true))]));
    let st = ValueStore::empty();
    st.set_for_test("ram.used_pct", 50.0);
    let f = render(&l, &st, &media(), 100, 100, 0);
    assert!(close(px(&f, 20, 20), [0, 255, 0], 2));
    assert!(close(px(&f, 80, 20), [0x22, 0x22, 0x22], 2));
    assert!(close(px(&f, 20, 80), [0, 255, 0], 2), "vertical : rempli par le bas");
    assert!(close(px(&f, 20, 45), [0x22, 0x22, 0x22], 2));
}

#[test]
fn graph_draws_line_and_fill() {
    let st = ValueStore::empty();
    for i in 0..30 {
        st.set_for_test("cpu.load", 10.0 + i as f64 * 3.0);
    }
    let kind = json!({"type": "graph", "source": "cpu.load", "min": 0, "max": 100, "seconds": 60, "line": "#00ffff", "fill": "#00ffff80", "thickness": 3});
    let l = layout(200, 100, "#000000", json!([widget("g", 0.0, 0.0, 200.0, 100.0, kind)]));
    let f = render(&l, &st, &media(), 200, 100, 0);
    // dernier point : 10 + 29*3 = 97 % -> près du haut, à droite.
    assert!(close(px(&f, 198, 4), [0, 255, 255], 60), "{:?}", px(&f, 198, 4));
    // sous la courbe : remplissage translucide non nul ; au-dessus : noir.
    assert!(px(&f, 190, 90)[1] > 10);
    assert_eq!(px(&f, 190, 1)[0..3], [0, 0, 0]);
}

#[test]
fn rect_fill_stroke_radius_rotation_opacity_scale() {
    let rect = |fill: &str, stroke: Value, r: f32| json!({"type": "rect", "fill": fill, "stroke": stroke, "stroke_width": 4, "radius": r});
    let l = layout(100, 100, "#000000", json!([widget("r", 10.0, 10.0, 80.0, 80.0, rect("#0000ff", json!("#ff0000"), 20.0))]));
    let f = render(&l, &ValueStore::empty(), &media(), 100, 100, 0);
    assert!(close(px(&f, 50, 50), [0, 0, 255], 1));
    assert!(close(px(&f, 50, 11), [255, 0, 0], 20), "contour");
    assert_eq!(px(&f, 11, 11)[0..3], [0, 0, 0], "coin arrondi");
    // Échelle x2.
    let f2 = render(&l, &ValueStore::empty(), &media(), 200, 200, 0);
    assert!(close(px(&f2, 100, 100), [0, 0, 255], 1) && close(px(&f2, 100, 22), [255, 0, 0], 20));
    // Rotation 90 deg : 80x20 devient vertical.
    let mut rot = widget("r", 10.0, 40.0, 80.0, 20.0, rect("#ffffff", Value::Null, 0.0));
    rot["rotation"] = json!(90);
    let f = render(&layout(100, 100, "#000000", json!([rot])), &ValueStore::empty(), &media(), 100, 100, 0);
    assert!(close(px(&f, 50, 15), [255; 3], 1) && px(&f, 15, 50)[0] < 5);
    // Opacité 50 %.
    let mut half = widget("r", 0.0, 0.0, 100.0, 100.0, rect("#ffffff", Value::Null, 0.0));
    half["opacity"] = json!(0.5);
    let f = render(&layout(100, 100, "#000000", json!([half])), &ValueStore::empty(), &media(), 100, 100, 0);
    assert!(close(px(&f, 50, 50), [128; 3], 2));
}

#[test]
fn image_widget_and_media_background() {
    let dir = std::env::temp_dir().join(format!("purergb-render-img-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("tmp");
    let src = dir.join("red.png");
    image::RgbaImage::from_pixel(20, 10, image::Rgba([255, 0, 0, 255])).save(&src).expect("png");
    let m = MediaStore::new(dir.join("media"));
    let id = m.import(&src).expect("import").id;
    let st = ValueStore::empty();
    let l = layout(100, 100, "#000000", json!([widget("i", 0.0, 0.0, 100.0, 100.0, json!({"type": "image", "media_id": id, "fit": "contain"}))]));
    let f = render(&l, &st, &m, 100, 100, 0);
    assert!(close(px(&f, 50, 50), [255, 0, 0], 2));
    assert_eq!(px(&f, 50, 10)[0..3], [0, 0, 0], "contain : bandes noires (ratio 2:1)");
    let mut l = layout(100, 100, "#000000", json!([]));
    l.background = serde_json::from_value(json!({"kind": "media", "media_id": id, "fit": "cover"})).expect("bg");
    assert!(close(px(&render(&l, &st, &m, 100, 100, 0), 50, 5), [255, 0, 0], 2), "cover remplit tout");
    // Média manquant : fond noir, pas de panique.
    let l = layout(50, 50, "#000000", json!([widget("i", 0.0, 0.0, 50.0, 50.0, json!({"type": "image", "media_id": "nope", "fit": "cover"}))]));
    assert_eq!(lit(&render(&l, &st, &m, 50, 50, 0)), 0);
}

#[test]
fn analog_clock_draws_face_and_center() {
    let kind = json!({"type": "analog_clock", "face": "#202040", "hands": "#ffffff", "accent": "#ff0000"});
    let l = layout(200, 200, "#000000", json!([widget("c", 0.0, 0.0, 200.0, 200.0, kind)]));
    let f = render(&l, &ValueStore::empty(), &media(), 200, 200, 0);
    assert!(close(px(&f, 100, 100), [255, 0, 0], 4), "pivot accentué");
    assert!(close(px(&f, 193, 125), [0x20, 0x20, 0x40], 3), "cadran : {:?}", px(&f, 193, 125));
    assert_eq!(px(&f, 2, 2)[0..3], [0, 0, 0], "hors du cadran");
}

#[test]
fn gradient_background_runs_from_to() {
    let mut l = layout(100, 100, "#000000", json!([]));
    l.background = serde_json::from_value(json!({"kind": "gradient", "from": "#ff0000", "to": "#0000ff", "angle": 90})).expect("bg");
    let f = render(&l, &ValueStore::empty(), &media(), 100, 100, 0);
    assert!(close(px(&f, 50, 1), [255, 0, 0], 8) && close(px(&f, 50, 98), [0, 0, 255], 8));
}

#[test]
fn hostile_layouts_never_panic() {
    let st = ValueStore::empty();
    let m = media();
    // Couleur invalide : seul ce widget est ignoré, les autres sont dessinés.
    let l = layout(
        100,
        100,
        "#000000",
        json!([
            widget("bad", 0.0, 0.0, 50.0, 50.0, json!({"type": "rect", "fill": "pas-une-couleur", "stroke": null, "stroke_width": 0, "radius": 0})),
            widget("ok", 50.0, 50.0, 50.0, 50.0, json!({"type": "rect", "fill": "#ffffff", "stroke": null, "stroke_width": 0, "radius": 0})),
            widget("neg", 0.0, 0.0, -5.0, 10.0, json!({"type": "rect", "fill": "#fff", "stroke": null, "stroke_width": 0, "radius": 0})),
        ]),
    );
    let f = render(&l, &st, &m, 100, 100, 0);
    assert_eq!(px(&f, 10, 10)[0..3], [0, 0, 0]);
    assert!(close(px(&f, 75, 75), [255; 3], 1));
    // Fond invalide : fond noir, widgets dessinés.
    let mut l2 = l.clone();
    l2.background = serde_json::from_value(json!({"kind": "solid", "color": "#zzz"})).expect("bg");
    assert!(close(px(&render(&l2, &st, &m, 100, 100, 0), 75, 75), [255; 3], 1));
    // Canevas énorme, 10 000 widgets, version inconnue : refusés par validate() -> frame noire.
    let big = Layout { width: 100_000, ..l.clone() };
    assert!(big.validate().is_err() && lit(&render(&big, &st, &m, 10, 10, 0)) == 0);
    let many = layout(10, 10, "#fff", Value::Array((0..10_000).map(|i| widget(&format!("w{i}"), 0.0, 0.0, 1.0, 1.0, json!({"type": "rect", "fill": "#fff", "stroke": null, "stroke_width": 0, "radius": 0}))).collect()));
    assert!(many.validate().is_err());
    assert_eq!(lit(&render(&many, &st, &m, 10, 10, 0)), 0);
    // Tailles de sortie dégénérées.
    assert_eq!(render(&l, &st, &m, 0, 0, 0).rgba.len(), 0);
    assert!(render(&l, &st, &m, 1, 1, 0).is_valid());
    assert_eq!(render(&l, &st, &m, u32::MAX, 4, 0).width, MAX_OUTPUT);
}

#[test]
fn png_base64_is_a_real_png() {
    let l = layout(8, 8, "#336699", json!([]));
    let f = render(&l, &ValueStore::empty(), &media(), 8, 8, 0);
    let b = frame_to_png_base64(&f).expect("png");
    assert!(b.starts_with("iVBORw0KGgo"));
    assert_eq!(base64(b"Ma"), "TWE=");
    assert_eq!(base64(b"M"), "TQ==");
    assert_eq!(base64(b"Man"), "TWFu");
    assert!(frame_to_png_base64(&Frame { width: 4, height: 4, rgba: vec![] }).is_err());
}

// ---------- aperçus de référence (inspectés à l'oeil) ----------

fn text(id: &str, x: f32, y: f32, w: f32, h: f32, t: &str, font: &str, size: f32, color: &str, align: &str, bold: bool) -> Value {
    widget(id, x, y, w, h, json!({"type": "text", "text": t, "style": {"font": font, "size": size, "color": color, "align": align, "bold": bold}}))
}

fn value(id: &str, x: f32, y: f32, w: f32, h: f32, src: &str, fmt: &str, font: &str, size: f32, color: &str, align: &str) -> Value {
    widget(id, x, y, w, h, json!({"type": "value", "source": src, "format": fmt, "style": {"font": font, "size": size, "color": color, "align": align, "bold": true}}))
}

fn gauge(id: &str, x: f32, y: f32, s: f32, src: &str, th: f32) -> Value {
    widget(id, x, y, s, s, json!({"type": "gauge_arc", "source": src, "min": 0, "max": 100, "start_deg": 135, "sweep_deg": 270, "thickness": th, "track": "#ffffff1a", "colors": [[0.0, "#2de2e6"], [0.6, "#7b5cff"], [1.0, "#ff2a6d"]]}))
}

fn demo_store() -> ValueStore {
    let st = ValueStore::empty();
    for i in 0..60 {
        let w = i as f64 * 0.35;
        st.set_for_test("cpu.load", 35.0 + 25.0 * w.sin() + 8.0 * (w * 2.3).cos());
        st.set_for_test("gpu.load", 55.0 + 30.0 * (w * 0.8).cos());
    }
    for (k, v) in [("cpu.temp", 58.0), ("gpu.temp", 71.0), ("ram.used_pct", 46.0), ("ram.used_gb", 14.7), ("ssd.1.temp", 39.0), ("ssd.1.used_pct", 63.0), ("cpu.clock", 4850.0), ("gpu.clock", 2475.0)] {
        st.set_for_test(k, v);
    }
    st
}

fn monitor(w: u32, h: u32) -> Layout {
    let (wf, hf) = (w as f32, h as f32);
    let k = hf / 480.0;
    let col = wf / 2.0;
    let side = |x: f32, name: &str, t: &str, l: &str, c: &str, clock: &str| {
        vec![
            text(&format!("{name}-h"), x, 22.0 * k, 190.0 * k, 28.0 * k, name, "Rajdhani", 26.0 * k, "#8b93b8", "center", true),
            gauge(&format!("{name}-g"), x + 20.0 * k, 56.0 * k, 150.0 * k, l, 14.0 * k),
            value(&format!("{name}-t"), x + 20.0 * k, 96.0 * k, 150.0 * k, 50.0 * k, t, "{:.0}°", "Rajdhani", 54.0 * k, "#ffffff", "center"),
            value(&format!("{name}-l"), x + 20.0 * k, 142.0 * k, 150.0 * k, 24.0 * k, l, "{:.0} %", "Inter", 18.0 * k, "#9aa3c7", "center"),
            value(&format!("{name}-c"), x, 214.0 * k, 190.0 * k, 22.0 * k, c, clock, "Roboto Mono", 16.0 * k, "#6e7699", "center"),
        ]
    };
    let mut ws = Vec::new();
    ws.extend(side(col / 2.0 - 95.0 * k, "CPU", "cpu.temp", "cpu.load", "cpu.clock", "{:.0} MHz"));
    ws.extend(side(col + col / 2.0 - 95.0 * k, "GPU", "gpu.temp", "gpu.load", "gpu.clock", "{:.0} MHz"));
    ws.push(widget("card", 24.0 * k, 252.0 * k, wf - 48.0 * k, 96.0 * k, json!({"type": "rect", "fill": "#ffffff0d", "stroke": "#ffffff1f", "stroke_width": 1.5, "radius": 16.0 * k})));
    ws.push(widget("graph", 34.0 * k, 262.0 * k, wf - 68.0 * k, 76.0 * k, json!({"type": "graph", "source": "cpu.load", "min": 0, "max": 100, "seconds": 60, "line": "#2de2e6", "fill": "#2de2e655", "thickness": 3.0 * k})));
    ws.push(value("time", 24.0 * k, 364.0 * k, 170.0 * k, 56.0 * k, "time", "%H:%M", "Rajdhani", 56.0 * k, "#ffffff", "left"));
    ws.push(value("date", 24.0 * k, 420.0 * k, 220.0 * k, 26.0 * k, "date", "%A %d %B", "Inter", 18.0 * k, "#8b93b8", "left"));
    ws.push(text("ram-l", wf - 250.0 * k, 368.0 * k, 226.0 * k, 22.0 * k, "MÉMOIRE", "Rajdhani", 18.0 * k, "#8b93b8", "right", true));
    ws.push(value("ram-v", wf - 250.0 * k, 388.0 * k, 226.0 * k, 28.0 * k, "ram.used_gb", "{:.1} Go", "Inter", 26.0 * k, "#ffffff", "right"));
    ws.push(widget("ram-b", wf - 250.0 * k, 424.0 * k, 226.0 * k, 10.0 * k, json!({"type": "bar", "source": "ram.used_pct", "min": 0, "max": 100, "vertical": false, "radius": 5.0 * k, "track": "#ffffff1a", "colors": [[0.0, "#2de2e6"], [1.0, "#ff2a6d"]]})));
    layout_from(w, h, ws)
}

fn layout_from(w: u32, h: u32, ws: Vec<Value>) -> Layout {
    let mut l = layout(w, h, "#000000", Value::Array(ws));
    l.background = serde_json::from_value(json!({"kind": "gradient", "from": "#0b1020", "to": "#1b0f2e", "angle": 60})).expect("bg");
    l
}

fn preview_dir() -> PathBuf {
    let d = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join("screens-preview");
    std::fs::create_dir_all(&d).expect("dossier d'aperçus");
    d
}

fn save(name: &str, f: &Frame) {
    image::save_buffer(preview_dir().join(name), &f.rgba, f.width, f.height, image::ColorType::Rgba8).expect("png");
}

#[test]
fn write_reference_previews() {
    let st = demo_store();
    let m = media();
    let sq = monitor(480, 480);
    save("monitor_480x480.png", &render(&sq, &st, &m, 480, 480, 0));
    save("monitor_240x240.png", &render(&sq, &st, &m, 240, 240, 0));

    // Variante large 1280x480 : jauges + horloge analogique + courbe.
    let mut ws = vec![
        gauge("g1", 40.0, 90.0, 300.0, "cpu.load", 26.0),
        gauge("g2", 940.0, 90.0, 300.0, "gpu.load", 26.0),
        value("t1", 40.0, 190.0, 300.0, 100.0, "cpu.temp", "{:.0}°C", "Rajdhani", 96.0, "#ffffff", "center"),
        value("t2", 940.0, 190.0, 300.0, 100.0, "gpu.temp", "{:.0}°C", "Rajdhani", 96.0, "#ffffff", "center"),
        text("l1", 40.0, 30.0, 300.0, 40.0, "PROCESSEUR", "Rajdhani", 34.0, "#8b93b8", "center", true),
        text("l2", 940.0, 30.0, 300.0, 40.0, "CARTE GRAPHIQUE", "Rajdhani", 34.0, "#8b93b8", "center", true),
        widget("clock", 440.0, 40.0, 400.0, 400.0, json!({"type": "analog_clock", "face": "#ffffff0a", "hands": "#e8ecff", "accent": "#ff2a6d"})),
    ];
    ws.push(widget("gr", 360.0, 400.0, 560.0, 60.0, json!({"type": "graph", "source": "gpu.load", "min": 0, "max": 100, "seconds": 60, "line": "#7b5cff", "fill": "#7b5cff55", "thickness": 3})));
    save("wide_1280x480.png", &render(&layout_from(1280, 480, ws), &st, &m, 1280, 480, 0));
}

#[test]
fn text_is_vertically_centred_on_its_box() {
    // « H » : la hauteur des capitales doit être centrée sur le milieu de la boîte (y = 150).
    let l = layout(300, 300, "#000000", json!([widget("t", 0.0, 100.0, 300.0, 100.0, json!({"type": "text", "text": "H", "style": style(80.0)}))]));
    let f = render(&l, &ValueStore::empty(), &media(), 300, 300, 0);
    let rows: Vec<u32> = (0..300).filter(|&y| (0..300).any(|x| px(&f, x, y)[0] > 128)).collect();
    let (top, bottom) = (*rows.first().expect("rien dessiné"), *rows.last().unwrap());
    let centre = (top + bottom) as f32 / 2.0;
    assert!((centre - 150.0).abs() < 3.0, "capitales centrées en y={centre}, attendu 150 (haut {top}, bas {bottom})");
}
