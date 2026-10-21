// Rendu 2D simplifié pour le mode démo (le vrai rendu est fait par le moteur Rust).
import { formatValue, isTimeSource, sampleRamp, strftime } from "./format.ts";
import type { Layout, LiveValues, TextStyle, Widget } from "./types.ts";

/** Images de la médiathèque de démo (id -> image chargée). */
export const mockImages = new Map<string, HTMLImageElement>();

export function mockValues(tMs: number): LiveValues {
  const s = (period: number, lo: number, hi: number, phase = 0): number =>
    lo + ((Math.sin(tMs / period + phase) + 1) / 2) * (hi - lo);
  return {
    "cpu.temp": s(3000, 38, 82),
    "cpu.load": s(2200, 5, 95, 1),
    "cpu.clock": s(4000, 3200, 5100),
    "cpu.power": s(2600, 25, 140),
    "cpu.fan": s(5000, 900, 1800),
    "gpu.temp": s(3500, 40, 78, 2),
    "gpu.load": s(1800, 2, 99, 3),
    "gpu.clock": s(4200, 1400, 2700),
    "gpu.power": s(2400, 30, 320),
    "gpu.vram_used_pct": s(9000, 20, 80),
    "gpu.fan": s(5000, 0, 1900),
    "ram.used_pct": s(11000, 35, 70),
    "ram.used_gb": s(11000, 11, 22),
    "ram.total_gb": 32,
    "liquid.temp": s(7000, 28, 38),
    "pump.rpm": s(8000, 2600, 2900),
    "net.up": s(2500, 0, 3),
    "net.down": s(1700, 0, 12),
  };
}

function ctxFont(s: TextStyle): string {
  return `${s.bold ? "700" : "400"} ${s.size}px "${s.font}", "Segoe UI", sans-serif`;
}

function norm(v: number, min: number, max: number): number {
  return max === min ? 0 : Math.min(1, Math.max(0, (v - min) / (max - min)));
}

function liveText(source: string, format: string, vals: LiveValues, now: Date): string {
  if (isTimeSource(source)) {
    if (source === "uptime") return "3 h 12";
    return strftime(format.includes("%") ? format : source === "time" ? "%H:%M" : "%d/%m/%Y", now);
  }
  return formatValue(format, vals[source] ?? 0);
}

function drawText(c: CanvasRenderingContext2D, text: string, s: TextStyle, w: number): void {
  c.font = ctxFont(s);
  c.fillStyle = s.color;
  c.textBaseline = "middle";
  c.textAlign = s.align;
  const x = s.align === "left" ? 0 : s.align === "right" ? w : w / 2;
  c.fillText(text, x - w / 2, 0, Math.max(w, 1));
}

function roundRect(c: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number): void {
  const rr = Math.max(0, Math.min(r, w / 2, h / 2));
  c.beginPath();
  c.roundRect(x, y, w, h, rr);
}

function drawImageFit(
  c: CanvasRenderingContext2D,
  img: HTMLImageElement | undefined,
  label: string,
  x: number,
  y: number,
  w: number,
  h: number,
  fit: "cover" | "contain" | "stretch",
): void {
  if (!img || !img.naturalWidth) {
    c.fillStyle = "#33334a";
    c.fillRect(x, y, w, h);
    c.strokeStyle = "#6a6a88";
    c.strokeRect(x + 1, y + 1, w - 2, h - 2);
    c.fillStyle = "#b8b8d0";
    c.font = `${Math.max(10, Math.min(w, h) / 6)}px sans-serif`;
    c.textAlign = "center";
    c.textBaseline = "middle";
    c.fillText(label || "image", x + w / 2, y + h / 2, w);
    return;
  }
  const iw = img.naturalWidth;
  const ih = img.naturalHeight;
  if (fit === "stretch") return c.drawImage(img, x, y, w, h);
  const k = fit === "cover" ? Math.max(w / iw, h / ih) : Math.min(w / iw, h / ih);
  c.save();
  c.beginPath();
  c.rect(x, y, w, h);
  c.clip();
  c.drawImage(img, x + (w - iw * k) / 2, y + (h - ih * k) / 2, iw * k, ih * k);
  c.restore();
}

function drawWidget(c: CanvasRenderingContext2D, wd: Widget, vals: LiveValues, now: Date, tMs: number): void {
  const { w, h } = wd;
  switch (wd.type) {
    case "text":
      drawText(c, wd.text, wd.style, w);
      break;
    case "value":
      drawText(c, liveText(wd.source, wd.format, vals, now), wd.style, w);
      break;
    case "gauge_arc": {
      const n = norm(vals[wd.source] ?? 0, wd.min, wd.max);
      const r = Math.min(w, h) / 2 - wd.thickness / 2;
      const a0 = (wd.start_deg * Math.PI) / 180;
      const sweep = (wd.sweep_deg * Math.PI) / 180;
      c.lineWidth = wd.thickness;
      c.lineCap = "round";
      c.strokeStyle = wd.track;
      c.beginPath();
      c.arc(0, 0, r, a0, a0 + sweep);
      c.stroke();
      if (n > 0.001) {
        c.strokeStyle = sampleRamp(wd.colors, n);
        c.beginPath();
        c.arc(0, 0, r, a0, a0 + sweep * n);
        c.stroke();
      }
      break;
    }
    case "bar": {
      const n = norm(vals[wd.source] ?? 0, wd.min, wd.max);
      c.fillStyle = wd.track;
      roundRect(c, -w / 2, -h / 2, w, h, wd.radius);
      c.fill();
      c.fillStyle = sampleRamp(wd.colors, n);
      if (wd.vertical) roundRect(c, -w / 2, h / 2 - h * n, w, h * n, wd.radius);
      else roundRect(c, -w / 2, -h / 2, w * n, h, wd.radius);
      c.fill();
      break;
    }
    case "graph": {
      const pts = 48;
      const ys = Array.from({ length: pts }, (_, i) => {
        const base = vals[wd.source] ?? (wd.min + wd.max) / 2;
        const t = tMs - (pts - i) * 800;
        return norm(base + Math.sin(t / 1500) * (wd.max - wd.min) * 0.12, wd.min, wd.max);
      });
      c.beginPath();
      ys.forEach((n, i) => {
        const px = -w / 2 + (i / (pts - 1)) * w;
        const py = h / 2 - n * h;
        if (i === 0) c.moveTo(px, py);
        else c.lineTo(px, py);
      });
      if (wd.fill) {
        c.save();
        c.lineTo(w / 2, h / 2);
        c.lineTo(-w / 2, h / 2);
        c.closePath();
        c.fillStyle = wd.fill;
        c.fill();
        c.restore();
        c.beginPath();
        ys.forEach((n, i) => {
          const px = -w / 2 + (i / (pts - 1)) * w;
          const py = h / 2 - n * h;
          if (i === 0) c.moveTo(px, py);
          else c.lineTo(px, py);
        });
      }
      c.strokeStyle = wd.line;
      c.lineWidth = wd.thickness;
      c.lineJoin = "round";
      c.stroke();
      break;
    }
    case "rect":
      roundRect(c, -w / 2, -h / 2, w, h, wd.radius);
      c.fillStyle = wd.fill;
      c.fill();
      if (wd.stroke) {
        c.strokeStyle = wd.stroke;
        c.lineWidth = wd.stroke_width;
        c.stroke();
      }
      break;
    case "image":
      drawImageFit(c, mockImages.get(wd.media_id), wd.media_id ? "image" : "(aucune image)", -w / 2, -h / 2, w, h, wd.fit);
      break;
    case "analog_clock": {
      const r = Math.min(w, h) / 2;
      c.beginPath();
      c.arc(0, 0, r, 0, Math.PI * 2);
      c.fillStyle = wd.face;
      c.fill();
      c.strokeStyle = wd.accent;
      c.lineWidth = Math.max(2, r / 20);
      c.stroke();
      const hand = (frac: number, len: number, width: number, color: string): void => {
        const a = frac * Math.PI * 2 - Math.PI / 2;
        c.beginPath();
        c.moveTo(0, 0);
        c.lineTo(Math.cos(a) * r * len, Math.sin(a) * r * len);
        c.strokeStyle = color;
        c.lineWidth = width;
        c.lineCap = "round";
        c.stroke();
      };
      const sec = now.getSeconds();
      const min = now.getMinutes() + sec / 60;
      hand(((now.getHours() % 12) + min / 60) / 12, 0.5, r / 14, wd.hands);
      hand(min / 60, 0.78, r / 20, wd.hands);
      hand(sec / 60, 0.85, r / 40, wd.accent);
      break;
    }
  }
}

export function renderMock(layout: Layout, width: number, height: number, tMs: number): string {
  const cv = document.createElement("canvas");
  cv.width = width;
  cv.height = height;
  const c = cv.getContext("2d");
  if (!c) return "";
  const k = width / layout.width;
  c.scale(k, height / layout.height);
  const bg = layout.background;
  if (bg.kind === "solid") {
    c.fillStyle = bg.color;
    c.fillRect(0, 0, layout.width, layout.height);
  } else if (bg.kind === "gradient") {
    const a = (bg.angle * Math.PI) / 180;
    const cx = layout.width / 2;
    const cy = layout.height / 2;
    const d = Math.hypot(layout.width, layout.height) / 2;
    const g = c.createLinearGradient(
      cx - Math.sin(a) * d,
      cy + Math.cos(a) * d,
      cx + Math.sin(a) * d,
      cy - Math.cos(a) * d,
    );
    g.addColorStop(0, bg.from);
    g.addColorStop(1, bg.to);
    c.fillStyle = g;
    c.fillRect(0, 0, layout.width, layout.height);
  } else {
    c.fillStyle = "#000";
    c.fillRect(0, 0, layout.width, layout.height);
    drawImageFit(c, mockImages.get(bg.media_id), "(média de fond)", 0, 0, layout.width, layout.height, bg.fit);
  }
  const vals = mockValues(tMs);
  const now = new Date();
  for (const wd of layout.widgets) {
    c.save();
    c.translate(wd.x + wd.w / 2, wd.y + wd.h / 2);
    c.rotate((wd.rotation * Math.PI) / 180);
    c.globalAlpha = Math.min(1, Math.max(0, wd.opacity));
    drawWidget(c, wd, vals, now, tMs);
    c.restore();
  }
  return cv.toDataURL("image/png").replace(/^data:image\/png;base64,/, "");
}
