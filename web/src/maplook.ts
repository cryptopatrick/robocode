// The arena drawn like a planning map: pale lime ground with darker green meadows, peach and pink plots, white roads,
// cross-hatched buildings with heavy outlines, small pale tree symbols, plot numbers and spot heights. Shared by the
// 2D drawing (drawMap) and the 3D scene (palette, plots, meadows, hatch texture). Everything is decoration, seeded
// from the arena size so it is the same every time.
import type { Recording } from './recording';
import { roadsFor, sceneryFor } from './scenery';
import { seeded } from './textures';

type Box = [number, number, number, number];

export interface MapPalette {
  ground: string;
  meadow: string;
  plot: string;
  pink: string;
  road: string;
  /** Linework and outlines. */
  ink: string;
  /** Plot numbers and spot heights. */
  label: string;
  treeFill: string;
  treeEdge: string;
  /** Roads and their edges are lightly drawn so a plot's border reads against them. */
  hairline: string;
  buildings: BuildingLook[];
}

export interface BuildingLook {
  fill: string;
  hatch: string;
  /** Outline width in arena pixels. */
  outline: number;
  /** What lies round it: a peach plot, a pink plot, or just the ground. */
  plot: 'plot' | 'pink' | null;
}

export const MAP_DAY: MapPalette = {
  ground: '#e6f1b8',
  meadow: '#b3d66f',
  plot: '#f4ddbc',
  pink: '#f7c4cb',
  road: '#ffffff',
  ink: '#2a2a2a',
  label: '#262626',
  treeFill: '#c3e08a',
  treeEdge: '#6f9a3d',
  hairline: 'rgba(60, 50, 40, 0.55)',
  buildings: [
    { fill: '#ead3b1', hatch: '#6a5844', outline: 2.4, plot: 'plot' },
    { fill: '#d9ebab', hatch: '#587a35', outline: 2, plot: null },
    { fill: '#cdb2c2', hatch: '#5a4558', outline: 3.6, plot: 'pink' },
    { fill: '#dbd68f', hatch: '#6b6a2c', outline: 2.4, plot: 'plot' },
  ],
};

export const MAP_NIGHT: MapPalette = {
  ground: '#1d2b25',
  meadow: '#17331f',
  plot: '#33302a',
  pink: '#43303a',
  road: '#4a515c',
  ink: '#9aa5b3',
  label: '#c9d2dc',
  treeFill: '#3f6b45',
  treeEdge: '#7fb083',
  hairline: 'rgba(190, 200, 215, 0.35)',
  buildings: [
    { fill: '#5b4e3e', hatch: '#2d251c', outline: 2.4, plot: 'plot' },
    { fill: '#41562f', hatch: '#1f2c15', outline: 2, plot: null },
    { fill: '#5a4658', hatch: '#2a1f29', outline: 3.6, plot: 'pink' },
    { fill: '#5d5a2b', hatch: '#2e2d12', outline: 2.4, plot: 'plot' },
  ],
};

export const mapPalette = (daylight: boolean): MapPalette => (daylight ? MAP_DAY : MAP_NIGHT);

const PLOT_MARGIN = 18;

/** The plot of each building that has one: the building's footprint grown by a margin, kept inside the arena. */
export function plotsFor(rec: Recording, pal: MapPalette): { rect: Box; color: string }[] {
  const { width, height } = rec.battlefield;
  const out: { rect: Box; color: string }[] = [];
  ((rec.terrain?.buildings ?? []) as Box[]).forEach(([x, y, w, h], i) => {
    const look = pal.buildings[i % pal.buildings.length];
    if (!look.plot) return;
    const x0 = Math.max(0, x - PLOT_MARGIN), y0 = Math.max(0, y - PLOT_MARGIN);
    const x1 = Math.min(width, x + w + PLOT_MARGIN), y1 = Math.min(height, y + h + PLOT_MARGIN);
    out.push({ rect: [x0, y0, x1 - x0, y1 - y0], color: look.plot === 'pink' ? pal.pink : pal.plot });
  });
  return out;
}

const meadowCache = new WeakMap<Recording, [number, number][][]>();

/** Darker green meadows: a few soft closed blobs, as lists of points in arena pixels. */
export function meadowsFor(rec: Recording): [number, number][][] {
  const hit = meadowCache.get(rec);
  if (hit) return hit;
  const { width, height } = rec.battlefield;
  const rnd = seeded(Math.round(width * 13 + height * 7));
  const count = Math.max(2, Math.round((width * height) / 160000));
  const blobs: [number, number][][] = [];
  for (let k = 0; k < count; k++) {
    const cx = rnd() * width, cy = rnd() * height, radius = 90 + rnd() * 110;
    const pts: [number, number][] = [];
    for (let i = 0; i < 10; i++) {
      const a = (i / 10) * Math.PI * 2, r = radius * (0.65 + 0.5 * rnd());
      pts.push([Math.max(0, Math.min(width, cx + Math.cos(a) * r * 1.3)), Math.max(0, Math.min(height, cy + Math.sin(a) * r * 0.8))]);
    }
    blobs.push(pts);
  }
  meadowCache.set(rec, blobs);
  return blobs;
}

const hatchTiles = new Map<string, HTMLCanvasElement | null>();

/** A cross-hatch tile (diagonal lines both ways, 8 px apart) over a fill. */
export function hatchCanvas(fill: string, line: string): HTMLCanvasElement | null {
  if (typeof document === 'undefined') return null;
  const key = `${fill}|${line}`;
  if (!hatchTiles.has(key)) {
    const c = document.createElement('canvas');
    c.width = c.height = 8;
    const g = c.getContext('2d');
    if (g) {
      g.fillStyle = fill;
      g.fillRect(0, 0, 8, 8);
      g.strokeStyle = line;
      g.lineWidth = 0.9;
      g.beginPath();
      for (const [a, b, c2, d] of [[0, 8, 8, 0], [-1, 1, 1, -1], [7, 9, 9, 7], [0, 0, 8, 8], [-1, 7, 1, 9], [7, -1, 9, 1]]) {
        g.moveTo(a, b);
        g.lineTo(c2, d);
      }
      g.stroke();
    }
    hatchTiles.set(key, g ? c : null);
  }
  return hatchTiles.get(key) ?? null;
}

/** The whole map layer in 2D (ground, meadows, plots, roads, buildings, trees, numbers). */
export function drawMap(ctx: CanvasRenderingContext2D, rec: Recording, daylight: boolean): void {
  const pal = mapPalette(daylight);
  const { width: w, height: h } = rec.battlefield;
  const buildings = (rec.terrain?.buildings ?? []) as Box[];
  ctx.fillStyle = pal.ground;
  ctx.fillRect(0, 0, w, h);

  // Meadows: smooth closed curves through the blob points.
  ctx.fillStyle = pal.meadow;
  for (const pts of meadowsFor(rec)) {
    ctx.beginPath();
    const mid = (a: [number, number], b: [number, number]): [number, number] => [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
    const start = mid(pts[pts.length - 1], pts[0]);
    ctx.moveTo(start[0], start[1]);
    pts.forEach((p, i) => {
      const m = mid(p, pts[(i + 1) % pts.length]);
      ctx.quadraticCurveTo(p[0], p[1], m[0], m[1]);
    });
    ctx.closePath();
    ctx.fill();
  }

  // Plots with a hairline border.
  ctx.lineWidth = 0.8;
  for (const { rect: [x, y, pw, ph], color } of plotsFor(rec, pal)) {
    ctx.fillStyle = color;
    ctx.fillRect(x, y, pw, ph);
    ctx.strokeStyle = pal.hairline;
    ctx.strokeRect(x + 0.5, y + 0.5, pw - 1, ph - 1);
  }

  // Roads: plain white (the large road and the small paths differ only in width).
  ctx.fillStyle = pal.road;
  for (const { rect: [x, y, rw, rh] } of roadsFor(rec)) ctx.fillRect(x, y, rw, rh);

  // Buildings: a cross-hatched fill and a heavy outline, with a small boxed cross (a chimney) on the larger roofs.
  buildings.forEach(([x, y, bw, bh], i) => {
    const look = pal.buildings[i % pal.buildings.length];
    const tile = hatchCanvas(look.fill, look.hatch);
    ctx.fillStyle = (tile && ctx.createPattern(tile, 'repeat')) || look.fill;
    ctx.fillRect(x, y, bw, bh);
    ctx.strokeStyle = pal.ink;
    ctx.lineWidth = look.outline;
    ctx.strokeRect(x + look.outline / 2, y + look.outline / 2, bw - look.outline, bh - look.outline);
    if (bw >= 100 && bh >= 60) {
      const bx = x + bw * 0.68, by = y + bh * 0.3;
      ctx.fillStyle = look.fill;
      ctx.fillRect(bx, by, 14, 14);
      ctx.lineWidth = 1;
      ctx.strokeRect(bx + 0.5, by + 0.5, 13, 13);
      ctx.beginPath();
      ctx.moveTo(bx, by);
      ctx.lineTo(bx + 14, by + 14);
      ctx.moveTo(bx + 14, by);
      ctx.lineTo(bx, by + 14);
      ctx.stroke();
    }
  });

  // Trees as pale mushroom symbols; bushes as small open circles.
  const props = sceneryFor(rec);
  ctx.lineWidth = 0.9;
  for (const p of props) {
    if (p.kind === 'tree') {
      ctx.fillStyle = pal.treeEdge;
      ctx.fillRect(p.x - 1.5, p.y, 3, p.r * 0.9);
      ctx.fillStyle = pal.treeFill;
      ctx.strokeStyle = pal.treeEdge;
      ctx.beginPath();
      ctx.arc(p.x, p.y - p.r * 0.25, p.r, 0, Math.PI * 2);
      ctx.fill();
      ctx.stroke();
    } else if (p.kind === 'bush') {
      ctx.strokeStyle = pal.ink;
      ctx.beginPath();
      ctx.arc(p.x, p.y, Math.max(2, p.r * 0.45), 0, Math.PI * 2);
      ctx.stroke();
    }
  }

  // Plot numbers, and a few spot heights on open ground.
  ctx.fillStyle = pal.label;
  ctx.textBaseline = 'alphabetic';
  ctx.font = '700 13px system-ui, sans-serif';
  buildings.forEach(([x, y], i) => {
    const look = pal.buildings[i % pal.buildings.length];
    ctx.fillText(String(12 + i), look.plot ? Math.max(2, x - PLOT_MARGIN + 4) : x, look.plot ? Math.max(14, y - PLOT_MARGIN + 14) : Math.max(12, y - 5));
  });
  const rnd = seeded(Math.round(w * 3 + h * 5));
  ctx.font = '10px system-ui, sans-serif';
  ctx.globalAlpha = 0.8;
  const roads = roadsFor(rec);
  for (let k = 0, tries = 0; k < 5 && tries < 60; tries++) {
    const x = 20 + rnd() * (w - 70), y = 20 + rnd() * (h - 40);
    const clash = buildings.some(([bx, by, bw, bh]) => x > bx - 36 && x < bx + bw + 20 && y > by - 36 && y < by + bh + 20) ||
      roads.some(({ rect: [rx, ry, rw, rh] }) => x > rx - 30 && x < rx + rw + 4 && y > ry - 14 && y < ry + rh + 14);
    if (clash) continue;
    ctx.fillText(`${(24 + rnd() * 2.8).toFixed(1)}`, x, y);
    k++;
  }
  ctx.globalAlpha = 1;
}
