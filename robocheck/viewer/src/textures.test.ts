import { describe, expect, it } from 'vitest';
import { roadsFor, sceneryFor } from '../../../web/src/scenery';
import type { Recording } from '../../../web/src/recording';
import { roadDashes, seeded, textureCanvas } from '../../../web/src/textures';

describe('textures', () => {
  it('a seed always gives the same numbers, different seeds differ', () => {
    const a = seeded(5), b = seeded(5), c = seeded(6);
    const xs = [a(), a(), a()], ys = [b(), b(), b()];
    expect(xs).toEqual(ys);
    expect(xs).not.toEqual([c(), c(), c()]);
    expect(xs.every((v) => v >= 0 && v < 1)).toBe(true);
  });

  it('a horizontal road gets dashes along its length, centred across it, inside its ends', () => {
    const road: [number, number, number, number] = [0, 100, 400, 40];
    const dashes = roadDashes(road);
    expect(dashes.length).toBeGreaterThan(5);
    for (const [x, y, w, h] of dashes) {
      expect(y + h / 2).toBeCloseTo(120);
      expect(x).toBeGreaterThanOrEqual(8);
      expect(x + w).toBeLessThanOrEqual(392);
    }
  });

  it('a vertical road gets vertical dashes; a narrow lane gets none', () => {
    const [x, y, w, h] = roadDashes([300, 0, 40, 600])[0];
    expect(h).toBeGreaterThan(w);
    expect(x + w / 2).toBeCloseTo(320);
    expect(roadDashes([0, 0, 400, 10])).toEqual([]);
  });

  it('draws nothing without a canvas to draw on (tests, old browsers)', () => {
    // jsdom has no 2D context: the caller keeps its flat colour.
    expect(textureCanvas('ground', '#123456')).toBeNull();
  });

  const arena = (terrain?: Recording['terrain']) =>
    ({ battlefield: { width: 1200, height: 600 }, terrain, robots: [], turns: [] }) as unknown as Recording;

  it('scenery is the same every time and keeps off buildings, roads and the starting points', () => {
    const rec = arena({ buildings: [[500, 200, 120, 120]], roads: [[0, 280, 1200, 40]] });
    const a = sceneryFor(rec);
    expect(a.length).toBeGreaterThan(40);
    expect(sceneryFor(arena({ buildings: [[500, 200, 120, 120]], roads: [[0, 280, 1200, 40]] }))).toEqual(a);
    for (const p of a) {
      expect(p.x > 497 - p.r && p.x < 623 + p.r && p.y > 197 - p.r && p.y < 323 + p.r).toBe(false);
      expect(p.y + p.r > 277 && p.y - p.r < 323).toBe(false);
      for (const [sx, sy] of [[100, 100], [1100, 500], [1100, 100], [100, 500]]) expect(Math.hypot(p.x - sx, p.y - sy)).toBeGreaterThan(40);
    }
  });

  it('trees grow in small groves, not in a ring along the walls', () => {
    const props = sceneryFor(arena());
    const trees = props.filter((p) => p.kind === 'tree');
    expect(trees.length).toBeGreaterThan(15);
    expect(trees.length).toBeLessThan(90);
    // Plenty of them stand well away from the walls, and some stand apart from any other tree.
    expect(trees.filter((t) => Math.min(t.x, t.y, 1200 - t.x, 600 - t.y) > 100).length).toBeGreaterThan(trees.length / 3);
    expect(props.some((p) => p.kind === 'bush' || p.kind === 'rock')).toBe(true);
  });

  it('a map without roads gets one large road and small paths clear of the buildings', () => {
    const buildings = [[360, 120, 168, 96], [672, 120, 168, 96], [360, 384, 168, 96], [672, 384, 168, 96], [564, 270, 72, 60]];
    const roads = roadsFor(arena({ buildings, roads: [] } as never));
    expect(roads.filter((r) => r.big)).toHaveLength(1);
    expect(roads.some((r) => !r.big)).toBe(true);
    for (const { rect: [x, y, w, h] } of roads) {
      for (const [bx, by, bw, bh] of buildings) expect(x < bx + bw && x + w > bx && y < by + bh && y + h > by).toBe(false);
    }
  });

  it('a map with its own roads keeps them', () => {
    const own: [number, number, number, number][] = [[0, 280, 1200, 40]];
    expect(roadsFor(arena({ buildings: [], roads: own } as never)).map((r) => r.rect)).toEqual(own);
  });
});

describe('the planning-map look', () => {
  const rec = {
    battlefield: { width: 1200, height: 600 },
    terrain: { buildings: [[360, 120, 168, 96], [672, 120, 168, 96], [360, 384, 168, 96], [672, 384, 168, 96], [564, 270, 72, 60]], roads: [] },
    robots: [],
    turns: [],
  } as unknown as Recording;

  it('buildings take their look in turn, and only some sit on a plot', async () => {
    const { MAP_DAY, plotsFor } = await import('../../../web/src/maplook');
    expect(MAP_DAY.buildings.length).toBe(4);
    const plots = plotsFor(rec, MAP_DAY);
    // Four of the five buildings have a plot (the green-hatched look has none), each a bit bigger than its building.
    expect(plots.length).toBe(4);
    const [x, y, w, h] = plots[0].rect;
    expect([x, y, w, h]).toEqual([342, 102, 204, 132]);
    for (const { rect: [px, py, pw, ph] } of plots) expect(px >= 0 && py >= 0 && px + pw <= 1200 && py + ph <= 600).toBe(true);
  });

  it('the meadows are the same every time and stay inside the arena', async () => {
    const { meadowsFor } = await import('../../../web/src/maplook');
    const a = meadowsFor(rec);
    expect(a.length).toBeGreaterThanOrEqual(2);
    for (const pts of a) for (const [x, y] of pts) expect(x >= 0 && x <= 1200 && y >= 0 && y <= 600).toBe(true);
  });
});

describe('the real-city look', () => {
  it('a recording with street lines is a city, and taller buildings shade lighter', async () => {
    const { isCity, heightShade, roofColor, TOKYO_NIGHT } = await import('../../../web/src/tokyo');
    expect(isCity({ terrain: { buildings: [], roads: [] } } as unknown as Recording)).toBe(false);
    expect(isCity({ terrain: { buildings: [], roads: [], road_lines: [{ class: 'primary', width: 100, points: [[0, 0], [1, 1]] }] } } as unknown as Recording)).toBe(true);
    expect(heightShade(5)).toBe(0);
    expect(heightShade(80)).toBe(1);
    expect(roofColor(TOKYO_NIGHT, 12)).not.toEqual(roofColor(TOKYO_NIGHT, 50));
  });
});
