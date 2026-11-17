import { describe, expect, it } from 'vitest';
import { robotLabel } from './draw';
import type { RobotSnapshot } from './recording';

const robot = (energy: number, alive = true): RobotSnapshot => ({
  id: 0, position: { x: 0, y: 0 }, heading: 0, gun_heading: 0, radar_heading: 0,
  velocity: 0, energy, gun_heat: 0, alive, radius: 18,
});

describe('robotLabel', () => {
  it('shows name and energy to one decimal', () => {
    expect(robotLabel(robot(94.04), 'SafeShooter')).toBe('SafeShooter 94.0');
  });
  it('shows Disabled at zero energy while alive', () => {
    expect(robotLabel(robot(0), 'SafeShooter')).toBe('SafeShooter Disabled');
  });
  it('shows Dead when not alive', () => {
    expect(robotLabel(robot(-3.5, false), 'SafeShooter')).toBe('SafeShooter Dead');
  });
});

import { labelY } from './draw';

describe('labelY', () => {
  it('puts the label below the robot normally', () => {
    expect(labelY(300, 600)).toBe(300 + 18 + 14);
  });
  it('puts it above the robot near the bottom edge', () => {
    expect(labelY(576, 600)).toBe(576 - 18 - 6);
  });
});

import { labelX } from './draw';

describe('labelX', () => {
  it('centres the label on the robot when it fits', () => {
    expect(labelX(400, 800, 80)).toBe(400);
  });
  it('keeps the label inside the right edge', () => {
    expect(labelX(780, 800, 80)).toBe(800 - 40 - 4);
  });
  it('keeps the label inside the left edge', () => {
    expect(labelX(10, 800, 80)).toBe(40 + 4);
  });
});

import { drawArena } from './draw';
import type { Recording } from './recording';

describe('canvas fits each ratio', () => {
  const sizes: [number, number][] = [[800, 600], [1200, 600], [1067, 600], [600, 600]];
  for (const [w, h] of sizes) {
    it(`draws a corner robot's label inside ${w} × ${h}`, () => {
      const r = { ...robot(94), position: { x: w - 20, y: h - 20 } };
      const rec = {
        battlefield: { width: w, height: h },
        robots: [{ id: 0, name: 'SafeShooter' }],
        turns: [{ turn: 0, robots: [r], bullets: [], events: {} }],
      } as unknown as Recording;
      const texts: { text: string; x: number; y: number; width: number }[] = [];
      const noop = () => undefined;
      const ctx = new Proxy({} as Record<string, unknown>, {
        get: (t, k: string) => {
          if (k === 'measureText') return (text: string) => ({ width: text.length * 6 });
          if (k === 'fillText') return (text: string, x: number, y: number) => texts.push({ text, x, y, width: text.length * 6 });
          return k in t ? t[k] : noop;
        },
        set: (t, k: string, v) => ((t[k] = v), true),
      }) as unknown as CanvasRenderingContext2D;
      drawArena(ctx, rec, 0, { names: true, energy: true, scanArcs: false, effects: false } as never);
      expect(texts).toHaveLength(1);
      const t = texts[0];
      expect(t.x + t.width / 2).toBeLessThanOrEqual(w);
      expect(t.y).toBeLessThanOrEqual(h);
    });
  }
});
