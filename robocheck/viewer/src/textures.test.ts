import { describe, expect, it } from 'vitest';
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
});
