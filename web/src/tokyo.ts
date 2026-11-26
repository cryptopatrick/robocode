// The real-city look: a dark charcoal map with white street linework and cream extruded buildings, drawn from the
// street polylines and building heights a city map carries in its terrain (see recording.ts). Shared by 2D (drawTokyo)
// and 3D (the palette). Daylight is the same map printed light on paper.
import type { Recording } from './recording';

export interface TokyoPalette {
  ground: string;
  /** The street surface, a shade off the ground. */
  street: string;
  /** Street and footpath linework. */
  line: string;
  /** Building roofs from low to tall, and the wall colour. */
  roofLow: string;
  roofHigh: string;
  wall: string;
  shadow: string;
  label: string;
}

export const TOKYO_NIGHT: TokyoPalette = {
  ground: '#232225',
  street: '#2f2e32',
  line: '#ece8de',
  roofLow: '#8d8575',
  roofHigh: '#eadfc6',
  wall: '#6f6859',
  shadow: 'rgba(0, 0, 0, 0.45)',
  label: '#d9d4c7',
};

export const TOKYO_DAY: TokyoPalette = {
  ground: '#e9e6de',
  street: '#fbfaf6',
  line: '#a39d90',
  roofLow: '#c9c1af',
  roofHigh: '#fffdf6',
  wall: '#aaa292',
  shadow: 'rgba(60, 50, 30, 0.28)',
  label: '#3b372f',
};

export const tokyoPalette = (daylight: boolean): TokyoPalette => (daylight ? TOKYO_DAY : TOKYO_NIGHT);

/** True when the recording carries a real-city map. */
export const isCity = (rec: Recording): boolean => !!rec.terrain?.road_lines?.length;

const mix = (a: string, b: string, t: number): string => {
  const pa = [1, 3, 5].map((i) => parseInt(a.slice(i, i + 2), 16));
  const pb = [1, 3, 5].map((i) => parseInt(b.slice(i, i + 2), 16));
  return `rgb(${pa.map((v, i) => Math.round(v + (pb[i] - v) * t)).join(',')})`;
};

/** How tall a building is on a 0 (lowest) to 1 (tallest) scale, for shading. */
export const heightShade = (m: number): number => Math.max(0, Math.min(1, (m - 10) / 46));

export const roofColor = (pal: TokyoPalette, m: number): string => mix(pal.roofLow, pal.roofHigh, heightShade(m));

/** The map layer in 2D: ground, streets, then buildings as oblique blocks (the taller, the longer their sides). */
export function drawTokyo(ctx: CanvasRenderingContext2D, rec: Recording, daylight: boolean): void {
  const pal = tokyoPalette(daylight);
  const t = rec.terrain!;
  const { width: w, height: h } = rec.battlefield;
  ctx.fillStyle = pal.ground;
  ctx.fillRect(0, 0, w, h);

  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  const trace = (pts: [number, number][]) => {
    ctx.beginPath();
    pts.forEach(([x, y], i) => (i ? ctx.lineTo(x, y) : ctx.moveTo(x, y)));
  };
  const lines = t.road_lines ?? [];
  // Street surfaces first, widest last drawn on top so junctions merge, then the thin linework over all of them.
  ctx.strokeStyle = pal.street;
  for (const r of lines.filter((l) => l.width > 6)) {
    ctx.lineWidth = r.width;
    trace(r.points);
    ctx.stroke();
  }
  ctx.strokeStyle = pal.line;
  for (const r of lines) {
    const street = r.width > 6;
    ctx.globalAlpha = street ? 0.85 : 0.45;
    ctx.lineWidth = street ? 1.4 : 0.8;
    trace(r.points);
    ctx.stroke();
  }
  ctx.globalAlpha = 1;

  // Buildings, back to front: the roof is lifted up and to the left by the height, so the walls show on the right
  // and below; a lower building in front hides the walls of one behind it.
  const heights = t.heights ?? [];
  const order = t.buildings.map((b, i) => ({ b, i })).sort((a, c) => a.b[1] + a.b[3] - (c.b[1] + c.b[3]));
  for (const { b: [x, y, bw, bh], i } of order) {
    const m = heights[i] ?? 25;
    const o = Math.min(14, m * 0.2);
    ctx.fillStyle = pal.shadow;
    ctx.fillRect(x + o * 0.8, y + o * 1.1, bw, bh);
    ctx.fillStyle = pal.wall;
    ctx.beginPath();
    ctx.moveTo(x + bw - o, y - o);
    ctx.lineTo(x + bw, y);
    ctx.lineTo(x + bw, y + bh);
    ctx.lineTo(x, y + bh);
    ctx.lineTo(x - o, y + bh - o);
    ctx.lineTo(x + bw - o, y + bh - o);
    ctx.closePath();
    ctx.fill();
    ctx.fillStyle = roofColor(pal, m);
    ctx.fillRect(x - o, y - o, bw, bh);
    ctx.strokeStyle = 'rgba(0, 0, 0, 0.35)';
    ctx.lineWidth = 0.8;
    ctx.strokeRect(x - o + 0.4, y - o + 0.4, bw - 0.8, bh - 0.8);
  }

  // The credit the map's licence asks for.
  if (t.attribution) {
    ctx.font = '10px system-ui, sans-serif';
    ctx.fillStyle = pal.label;
    ctx.globalAlpha = 0.7;
    ctx.textAlign = 'right';
    ctx.fillText(t.attribution, w - 6, h - 6);
    ctx.textAlign = 'left';
    ctx.globalAlpha = 1;
  }
}
