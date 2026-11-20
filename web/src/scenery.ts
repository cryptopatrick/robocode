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

const cache = new WeakMap<Recording, Prop[]>();

/** The scenery of a recording's arena, avoiding its buildings, roads and starting points. */
export function sceneryFor(rec: Recording): Prop[] {
  const hit = cache.get(rec);
  if (hit) return hit;
  const { width, height } = rec.battlefield;
  const blockers: Box[] = [...(rec.terrain?.buildings ?? []), ...(rec.terrain?.roads ?? [])] as Box[];
  const starts = spawns(width, height);
  const free = (x: number, y: number, r: number, spawnGap: number) =>
    !blockers.some((b) => inBox(x, y, b, r + 3)) && !starts.some(([sx, sy]) => Math.hypot(x - sx, y - sy) < spawnGap);
  const rnd = seeded(Math.round(width * 31 + height * 17));
  const props: Prop[] = [];

  // Trees: a belt along all four walls, one or two trees deep, with gaps where a road meets the wall.
  const belt = (length: number, at: (along: number, inset: number) => [number, number]) => {
    for (let along = 14 + rnd() * 12; along < length - 10; along += 26 + rnd() * 16) {
      for (const row of [0, 1]) {
        if (row === 1 && rnd() < 0.45) continue;
        const r = (row === 0 ? 9 : 7) + rnd() * 7;
        const inset = (row === 0 ? 10 : 36) + rnd() * 10;
        const [x, y] = at(along + row * 13, inset);
        if (free(x, y, r, 62)) props.push({ kind: 'tree', x, y, r, v: rnd() });
      }
    }
  };
  belt(width, (a, i) => [a, i]);
  belt(width, (a, i) => [a, height - i]);
  belt(height, (a, i) => [i, a]);
  belt(height, (a, i) => [width - i, a]);

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
