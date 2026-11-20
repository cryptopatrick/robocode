// Procedural textures for the arena, drawn on a canvas once and tiled: no image files. Used by the 2D drawing
// (CanvasPattern) and the 3D scene (CanvasTexture). Everything is seeded, so a texture is the same every time.
import type { Rect } from './recording';

export type TextureKind = 'ground' | 'road' | 'path' | 'roof' | 'tile' | 'slate' | 'gravel' | 'wall' | 'brick' | 'stucco';

/** A small seeded random generator (mulberry32): the same seed always gives the same textures. */
export function seeded(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export const TILE = 256;

const canvases = new Map<string, HTMLCanvasElement | null>();

function make(kind: TextureKind, base: string): HTMLCanvasElement | null {
  if (typeof document === 'undefined') return null;
  const c = document.createElement('canvas');
  c.width = c.height = TILE;
  const g = c.getContext('2d');
  if (!g) return null;
  const rnd = seeded({ ground: 11, road: 23, path: 29, roof: 37, tile: 53, slate: 59, gravel: 61, wall: 41, brick: 67, stucco: 71 }[kind]);
  g.fillStyle = base;
  g.fillRect(0, 0, TILE, TILE);
  // Grain: many tiny light and dark specks.
  const speck = (n: number, alpha: number, light: boolean, max: number) => {
    for (let i = 0; i < n; i++) {
      g.fillStyle = light ? `rgba(190,210,235,${alpha * rnd()})` : `rgba(0,0,0,${alpha * rnd()})`;
      g.fillRect(Math.floor(rnd() * TILE), Math.floor(rnd() * TILE), 1 + Math.floor(rnd() * max), 1 + Math.floor(rnd() * max));
    }
  };
  if (kind === 'ground') {
    // Grass: soft light and dark patches, then thousands of short blades leaning every which way.
    for (let i = 0; i < 18; i++) {
      const x = rnd() * TILE, y = rnd() * TILE, r = 22 + rnd() * 44;
      const grad = g.createRadialGradient(x, y, 0, x, y, r);
      grad.addColorStop(0, rnd() > 0.5 ? 'rgba(130,185,95,0.07)' : 'rgba(5,30,15,0.2)');
      grad.addColorStop(1, 'rgba(0,0,0,0)');
      g.fillStyle = grad;
      g.fillRect(x - r, y - r, 2 * r, 2 * r);
    }
    g.lineCap = 'round';
    for (let i = 0; i < 2600; i++) {
      const x = rnd() * TILE, y = rnd() * TILE, a = -Math.PI / 2 + (rnd() - 0.5) * 1.2, len = 3 + rnd() * 5;
      g.strokeStyle = rnd() > 0.45 ? `rgba(125,185,85,${0.1 + 0.16 * rnd()})` : `rgba(6,32,14,${0.2 + 0.24 * rnd()})`;
      g.lineWidth = 1;
      g.beginPath();
      g.moveTo(x, y);
      g.lineTo(x + Math.cos(a) * len, y + Math.sin(a) * len);
      g.stroke();
    }
  } else if (kind === 'road') {
    // Asphalt: fine dense grain and a few patched repairs.
    speck(2200, 0.18, true, 1);
    speck(1800, 0.25, false, 1);
    for (let i = 0; i < 4; i++) {
      g.fillStyle = 'rgba(0,0,0,0.1)';
      g.fillRect(rnd() * TILE, rnd() * TILE, 20 + rnd() * 40, 8 + rnd() * 20);
    }
  } else if (kind === 'roof') {
    // Concrete slabs: grain, seams every 64 px, and a few rooftop units.
    speck(1200, 0.14, true, 2);
    speck(900, 0.18, false, 2);
    g.strokeStyle = 'rgba(0,0,0,0.28)';
    g.lineWidth = 1;
    for (let p = 0; p <= TILE; p += 64) {
      g.beginPath();
      g.moveTo(p + 0.5, 0);
      g.lineTo(p + 0.5, TILE);
      g.moveTo(0, p + 0.5);
      g.lineTo(TILE, p + 0.5);
      g.stroke();
    }
    for (let i = 0; i < 6; i++) {
      const w = 10 + Math.floor(rnd() * 14), h = 8 + Math.floor(rnd() * 12), x = Math.floor(rnd() * (TILE - w)), y = Math.floor(rnd() * (TILE - h));
      g.fillStyle = 'rgba(0,0,0,0.26)';
      g.fillRect(x + 2, y + 2, w, h);
      g.fillStyle = 'rgba(210,225,245,0.22)';
      g.fillRect(x, y, w, h);
      g.strokeStyle = 'rgba(0,0,0,0.3)';
      g.strokeRect(x + 0.5, y + 0.5, w - 1, h - 1);
    }
  } else if (kind === 'path') {
    // A gravel path: coarse pale and dark stones.
    speck(3200, 0.3, true, 2);
    speck(3200, 0.34, false, 2);
  } else if (kind === 'gravel') {
    // A gravel roof: stones, and a few rooftop units.
    speck(3000, 0.28, true, 2);
    speck(3000, 0.32, false, 2);
    for (let i = 0; i < 5; i++) {
      const w = 12 + Math.floor(rnd() * 14), h = 10 + Math.floor(rnd() * 12), x = Math.floor(rnd() * (TILE - w)), y = Math.floor(rnd() * (TILE - h));
      g.fillStyle = 'rgba(0,0,0,0.28)';
      g.fillRect(x + 2, y + 2, w, h);
      g.fillStyle = 'rgba(205,215,225,0.3)';
      g.fillRect(x, y, w, h);
    }
  } else if (kind === 'tile') {
    // Clay roof tiles: staggered rows, each tile a little different in tone.
    const th = 16, tw = 20;
    for (let row = 0, y = 0; y < TILE; row++, y += th) {
      for (let x = -((row % 2) * tw) / 2; x < TILE; x += tw) {
        g.fillStyle = rnd() > 0.5 ? `rgba(255,170,120,${0.08 + 0.12 * rnd()})` : `rgba(0,0,0,${0.06 + 0.14 * rnd()})`;
        g.fillRect(x, y, tw, th);
        g.fillStyle = 'rgba(0,0,0,0.3)';
        g.fillRect(x, y, 1, th);
      }
      g.fillStyle = 'rgba(0,0,0,0.35)';
      g.fillRect(0, y + th - 2, TILE, 2);
    }
    speck(500, 0.15, false, 1);
  } else if (kind === 'slate') {
    // Slate shingles: overlapping rows with a shadow under each.
    const th = 12, tw = 16;
    for (let row = 0, y = 0; y < TILE; row++, y += th) {
      for (let x = -((row % 2) * tw) / 2; x < TILE; x += tw) {
        g.fillStyle = rnd() > 0.5 ? `rgba(170,190,225,${0.05 + 0.1 * rnd()})` : `rgba(0,0,0,${0.06 + 0.14 * rnd()})`;
        g.fillRect(x, y, tw, th);
        g.fillStyle = 'rgba(0,0,0,0.25)';
        g.fillRect(x, y, 1, th);
      }
      g.fillStyle = 'rgba(0,0,0,0.4)';
      g.fillRect(0, y + th - 1, TILE, 1.5);
    }
  } else {
    // Facades ('wall' glass and concrete, 'brick', 'stucco'): a base, then a grid of windows, some lit.
    if (kind === 'brick') {
      const bh = 8, bw = 24;
      for (let row = 0, y = 0; y < TILE; row++, y += bh) {
        for (let x = -((row % 2) * bw) / 2; x < TILE; x += bw) {
          g.fillStyle = rnd() > 0.5 ? `rgba(255,150,110,${0.05 + 0.14 * rnd()})` : `rgba(0,0,0,${0.05 + 0.14 * rnd()})`;
          g.fillRect(x, y, bw, bh);
          g.fillStyle = 'rgba(230,220,205,0.3)';
          g.fillRect(x, y, 1, bh);
        }
        g.fillStyle = 'rgba(230,220,205,0.3)';
        g.fillRect(0, y, TILE, 1);
      }
    } else if (kind === 'stucco') {
      speck(2400, 0.2, true, 2);
      speck(1800, 0.16, false, 2);
    } else {
      speck(700, 0.12, true, 1);
      speck(500, 0.16, false, 1);
    }
    const cw = 32, ch = 32;
    for (let cy = 0; cy < TILE; cy += ch) {
      for (let cx = 0; cx < TILE; cx += cw) {
        const lit = rnd() > 0.72;
        g.fillStyle = lit ? 'rgba(255,225,150,0.85)' : 'rgba(10,20,34,0.78)';
        g.fillRect(cx + 7, cy + 8, cw - 14, ch - 16);
        g.fillStyle = 'rgba(255,255,255,0.08)';
        g.fillRect(cx + 7, cy + 8, cw - 14, 3);
      }
    }
  }
  return c;
}

/** The tile canvas for a kind in a base colour (cached), or null where there is no canvas (tests, old browsers). */
export function textureCanvas(kind: TextureKind, base: string): HTMLCanvasElement | null {
  const key = `${kind}|${base}`;
  if (!canvases.has(key)) canvases.set(key, make(kind, base));
  return canvases.get(key) ?? null;
}

/** A repeating 2D pattern, or null (then the caller keeps its flat colour). */
export function texturePattern(ctx: CanvasRenderingContext2D, kind: TextureKind, base: string): CanvasPattern | null {
  const c = textureCanvas(kind, base);
  return c ? ctx.createPattern(c, 'repeat') : null;
}

/** The dashes of a road's centre line: [x, y, w, h] rectangles along the long side, 3 px wide. */
export function roadDashes(road: Rect): Rect[] {
  const [x, y, w, h] = road;
  const horizontal = w >= h;
  const length = horizontal ? w : h;
  const across = horizontal ? h : w;
  if (across < 24) return [];
  const dash = 14, gap = 12, margin = 8;
  const out: Rect[] = [];
  for (let p = margin; p + dash <= length - margin; p += dash + gap) {
    out.push(horizontal ? [x + p, y + across / 2 - 1.5, dash, 3] : [x + across / 2 - 1.5, y + p, 3, dash]);
  }
  return out;
}

export interface HouseStyle {
  roof: TextureKind;
  roofColor: string;
  wall: TextureKind;
  wallColor: string;
}

const HOUSES = [
  { roof: 'tile', wall: 'brick', day: ['#b5603f', '#a2573f'], night: ['#6b3f3a', '#5f3d38'] },
  { roof: 'slate', wall: 'stucco', day: ['#5b6577', '#e3d8c2'], night: ['#343c4c', '#6d675f'] },
  { roof: 'gravel', wall: 'wall', day: ['#8d8f93', '#c8ccd3'], night: ['#4a505c', '#4d5868'] },
  { roof: 'roof', wall: 'brick', day: ['#d4d7dd', '#b0654a'], night: ['#4b5566', '#5a3d36'] },
] as const;

/** The look of building number `i`: roof and facade material and colour, varied from one building to the next. */
export function houseStyle(i: number, daylight: boolean): HouseStyle {
  const h = HOUSES[i % HOUSES.length];
  const [roofColor, wallColor] = daylight ? h.day : h.night;
  return { roof: h.roof, roofColor, wall: h.wall, wallColor };
}
