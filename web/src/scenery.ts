// Scenery for the arena: a belt of trees along the walls and bushes, rocks and grass tufts on open ground. It is
// decoration only (physics never reads it) and is the same every time for a given arena, so 2D and 3D agree.
import type { Recording } from './recording';
import { seeded } from './textures';

export type PropKind = 'tree' | 'bush' | 'rock' | 'tuft';

export interface Prop {
  kind: PropKind;
  /** Arena pixels, like the robots' positions. */
  x: number;
  y: number;
  /** Radius in pixels. */
  r: number;
  /** A number from 0 to 1 that varies the colour and shape a little. */
  v: number;
}

type Box = [number, number, number, number];

const SPAWN_INSET = 100;
const inBox = (x: number, y: number, [bx, by, bw, bh]: Box, margin: number) =>
  x > bx - margin && x < bx + bw + margin && y > by - margin && y < by + bh + margin;

/** Where tanks start: 100 px in from the corners. Nothing is placed close to them. */
function spawns(width: number, height: number): [number, number][] {
  return [
    [SPAWN_INSET, SPAWN_INSET],
    [width - SPAWN_INSET, height - SPAWN_INSET],
    [width - SPAWN_INSET, SPAWN_INSET],
    [SPAWN_INSET, height - SPAWN_INSET],
  ];
}

export interface Road {
  rect: Box;
  /** The large road (paved, with a dashed centre line) as opposed to a small path (gravel). */
  big: boolean;
}

const roadCache = new WeakMap<Recording, Road[]>();

/** The roads to draw: the map's own if it has any, otherwise one large road and a few small paths laid in the gaps
 *  between the buildings. Decoration only: it never changes how a battle plays. */
export function roadsFor(rec: Recording): Road[] {
  const hit = roadCache.get(rec);
  if (hit) return hit;
  const { width, height } = rec.battlefield;
  const own = (rec.terrain?.roads ?? []) as Box[];
  if (own.length) {
    const roads = own.map((rect) => ({ rect, big: Math.min(rect[2], rect[3]) >= 30 }));
    roadCache.set(rec, roads);
    return roads;
  }
  const buildings = (rec.terrain?.buildings ?? []) as Box[];
  // A horizontal band `t` thick whose top is nearest `target`, clear of every building by a few pixels; null if none.
  const band = (horizontal: boolean, t: number, target: number): Box | null => {
    const span = horizontal ? height : width;
    let best: number | null = null;
    for (let at = 4; at + t <= span - 4; at += 2) {
      const clear = buildings.every(([bx, by, bw, bh]) => {
        const lo = horizontal ? by : bx, hi = lo + (horizontal ? bh : bw);
        return at + t + 4 <= lo || at - 4 >= hi;
      });
      if (clear && (best === null || Math.abs(at - target) < Math.abs(best - target))) best = at;
    }
    if (best === null) return null;
    return horizontal ? [0, best, width, t] : [best, 0, t, height];
  };
  const roads: Road[] = [];
  const big = band(true, 44, height / 2 - 22) ?? band(false, 44, width / 2 - 22);
  if (big) roads.push({ rect: big, big: true });
  const clash = (r: Box) => roads.some(({ rect }) => (rect[2] > rect[3]) === (r[2] > r[3]) && Math.abs(rect[0] - r[0]) + Math.abs(rect[1] - r[1]) < 70);
  for (const [horizontal, target] of [[false, width * 0.14], [false, width * 0.86], [true, height * 0.1], [true, height * 0.9]] as const) {
    const r = band(horizontal, 16, target - 8);
    if (r && !clash(r)) roads.push({ rect: r, big: false });
  }
  roadCache.set(rec, roads);
  return roads;
}

const cache = new WeakMap<Recording, Prop[]>();

/** The scenery of a recording's arena, avoiding its buildings, roads and starting points. */
export function sceneryFor(rec: Recording): Prop[] {
  const hit = cache.get(rec);
  if (hit) return hit;
  const { width, height } = rec.battlefield;
  const blockers: Box[] = [...(rec.terrain?.buildings ?? []), ...roadsFor(rec).map((r) => r.rect)] as Box[];
  const starts = spawns(width, height);
  const free = (x: number, y: number, r: number, spawnGap: number) =>
    x > r && y > r && x < width - r && y < height - r &&
    !blockers.some((b) => inBox(x, y, b, r + 3)) && !starts.some(([sx, sy]) => Math.hypot(x - sx, y - sy) < spawnGap);
  const rnd = seeded(Math.round(width * 31 + height * 17));
  const props: Prop[] = [];

  // Trees: small groves scattered about, not a wall round the field. A grove is 4 to 9 trees round a centre, with a
  // few single trees besides.
  const centres: [number, number][] = [];
  const groves = Math.max(3, Math.round((width * height) / 95000));
  for (let tries = 0; centres.length < groves && tries < 400; tries++) {
    const x = 30 + rnd() * (width - 60), y = 30 + rnd() * (height - 60);
    if (free(x, y, 24, 120) && centres.every(([cx, cy]) => Math.hypot(x - cx, y - cy) > 170)) centres.push([x, y]);
  }
  for (const [cx, cy] of centres) {
    const n = 4 + Math.floor(rnd() * 6), spread = 20 + n * 3.5;
    for (let k = 0, tries = 0; k < n && tries < n * 6; tries++) {
      const a = rnd() * Math.PI * 2, d = spread * Math.sqrt(rnd());
      const r = 7 + rnd() * 7, x = cx + Math.cos(a) * d, y = cy + Math.sin(a) * d;
      if (!free(x, y, r, 70) || props.some((q) => q.kind === 'tree' && Math.hypot(q.x - x, q.y - y) < (q.r + r) * 0.8)) continue;
      props.push({ kind: 'tree', x, y, r, v: rnd() });
      k++;
    }
  }
  for (let k = 0, tries = 0; k < 6 && tries < 80; tries++) {
    const x = rnd() * width, y = rnd() * height, r = 8 + rnd() * 6;
    if (free(x, y, r, 80)) {
      props.push({ kind: 'tree', x, y, r, v: rnd() });
      k++;
    }
  }

  // Open ground: low things only, so a tank driving over them still looks right.
  const wanted = Math.round((width * height) / 5200);
  for (let tries = 0, placed = 0; placed < wanted && tries < wanted * 8; tries++) {
    const x = 50 + rnd() * (width - 100), y = 50 + rnd() * (height - 100);
    const roll = rnd();
    const kind: PropKind = roll < 0.5 ? 'tuft' : roll < 0.78 ? 'bush' : 'rock';
    const r = kind === 'tuft' ? 3 + rnd() * 3 : kind === 'bush' ? 5 + rnd() * 5 : 3 + rnd() * 4;
    if (!free(x, y, r, 46)) continue;
    props.push({ kind, x, y, r, v: rnd() });
    placed++;
  }
  cache.set(rec, props);
  return props;
}
