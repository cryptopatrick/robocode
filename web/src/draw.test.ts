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

import { drawArena, ROBOT_COLORS } from './draw';
import { TANKS } from './tanks/defs';
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

describe('recorded beam', () => {
  type Call = [string, ...number[]];
  const run = (beam: [number, number][] | undefined, scanArcs = true) => {
    const calls: Call[] = [];
    const fills: unknown[] = [];
    const noop = () => undefined;
    const ctx = new Proxy({} as Record<string, unknown>, {
      get: (t, k: string) => {
        if (k === 'measureText') return () => ({ width: 10 });
        if (k === 'arc' || k === 'moveTo' || k === 'lineTo') return (...a: number[]) => calls.push([k, ...a]);
        if (k === 'fill') return () => { if (String(t.fillStyle).endsWith('12')) fills.push(t.fillStyle); };
        return k in t ? t[k] : noop;
      },
      set: (t, k: string, v) => ((t[k] = v), true),
    }) as unknown as CanvasRenderingContext2D;
    const mk = (radar: number) => ({ ...robot(94), position: { x: 100, y: 100 }, radar_heading: radar, ...(beam ? { beam } : {}) });
    const rec = {
      battlefield: { width: 800, height: 600 },
      robots: [{ id: 0, name: 'A' }],
      turns: [
        { turn: 0, robots: [mk(0)], bullets: [], events: {} },
        { turn: 1, robots: [mk(30)], bullets: [], events: {} },
      ],
    } as unknown as Recording;
    drawArena(ctx, rec, 1, { names: false, energy: false, scanArcs, effects: false } as never);
    return { calls, fills };
  };
  const pts: [number, number][] = [[110, 100], [300, 50], [500, 120], [300, 200], [105, 105]];

  it('fills the recorded beam', () => {
    const { calls, fills } = run(pts);
    const base = run(pts, false).calls.length;
    const path = calls.filter((c) => c[0] === 'moveTo' || c[0] === 'lineTo').map((c) => [c[1], c[2]]);
    expect(calls.length - base).toBe(pts.length);
    expect(JSON.stringify(path)).toContain(JSON.stringify(pts).slice(1, -1));
    expect(calls.some((c) => c[0] === 'arc' && c[3] === 1200)).toBe(false);
    expect(fills).toHaveLength(1);
  });

  it('draws nothing for empty beam', () => {
    const { calls, fills } = run([]);
    expect(calls).toHaveLength(run([], false).calls.length);
    expect(fills).toHaveLength(0);
  });

  it('falls back to the wedge without beam', () => {
    const { calls } = run(undefined);
    expect(calls.find((c) => c[0] === 'arc')?.[3]).toBe(1200);
  });
});

describe('intent label', () => {
  it('shows the intent under the name', () => {
    const texts: string[] = [];
    const noop = () => undefined;
    const ctx = new Proxy({} as Record<string, unknown>, {
      get: (t, k: string) => {
        if (k === 'measureText') return () => ({ width: 10 });
        if (k === 'fillText') return (text: string) => texts.push(text);
        return k in t ? t[k] : noop;
      },
      set: (t, k: string, v) => ((t[k] = v), true),
    }) as unknown as CanvasRenderingContext2D;
    const rec = {
      battlefield: { width: 800, height: 600 },
      robots: [{ id: 0, name: 'Tactician' }],
      turns: [{ turn: 0, robots: [{ ...robot(94), position: { x: 100, y: 100 } }], bullets: [], events: {} }],
      decisions: [[{ intent: { maneuver: 'attack', risk: 'normal' } }]],
    } as unknown as Recording;
    drawArena(ctx, rec, 0, { names: true, energy: true, scanArcs: false, effects: false } as never);
    expect(texts).toContain('ATTACK · normal');
  });
});

describe('five tanks', () => {
  type Rec = { k: string; args: unknown[]; fillStyle?: unknown; filter?: unknown };
  const fakeImg = (name: string, w: number, h: number) => ({ name, naturalWidth: w, naturalHeight: h }) as unknown as HTMLImageElement;
  const k2 = { hull: fakeImg('hull', 203, 332), turret: fakeImg('turret', 174, 313) };
  const run = (robots: RobotSnapshot[], tanks: Parameters<typeof drawArena>[5], sprites: Parameters<typeof drawArena>[4] = {}) => {
    const log: Rec[] = [];
    const state: Record<string, unknown> = {};
    const stack: Record<string, unknown>[] = [];
    const ctx = new Proxy(state, {
      get: (t, k: string) => {
        if (k === 'save') return () => stack.push({ ...t });
        if (k === 'restore') return () => { Object.assign(t, stack.pop()); };
        if (k === 'measureText') return () => ({ width: 10 });
        if (k in t) return t[k];
        return (...args: unknown[]) => log.push({ k, args, fillStyle: t.fillStyle, filter: t.filter });
      },
      set: (t, k: string, v) => {
        t[k] = v;
        if (k === 'filter') log.push({ k: 'setFilter', args: [v], filter: v });
        return true;
      },
    }) as unknown as CanvasRenderingContext2D;
    const rec = {
      battlefield: { width: 800, height: 600 },
      robots: robots.map((_, i) => ({ id: i, name: `R${i}` })),
      turns: [{ turn: 0, robots, bullets: [], events: { bullet_hits: [], robot_deaths: [] } }],
    } as unknown as Recording;
    drawArena(ctx, rec, 0, { names: false, energy: false, scanArcs: false, effects: false }, sprites, tanks);
    return log;
  };
  const at = (i: number, alive = true): RobotSnapshot => ({ ...robot(80, alive), id: i, position: { x: 100 + i * 50, y: 100 } });
  const k2Tanks = { leopard2a8: k2 };

  it('draws hull then turret for the picked tank', () => {
    // Robot 0 defaults to leopard2a8; its images are the fakes.
    const log = run([at(0)], k2Tanks);
    const imgs = log.filter((l) => l.k === 'drawImage').map((l) => l.args[0]);
    expect(imgs).toEqual([k2.hull, k2.turret]);
  });

  it('muzzle cap uses the team colour', () => {
    const log = run([at(0)], k2Tanks);
    const turretAt = log.findIndex((l) => l.k === 'drawImage' && l.args[0] === k2.turret);
    const cap = log.slice(turretAt).find((l) => l.k === 'fillRect' && l.fillStyle === ROBOT_COLORS[0]);
    expect(cap).toBeDefined();
    expect(cap!.args.slice(2)).toEqual([4, 3]);
  });

  it('team marker is a small full dot in the team colour', () => {
    const log = run([at(0)], k2Tanks);
    const dot = log.find((l) => l.k === 'arc' && l.args[2] === 3 && l.args[4] === Math.PI * 2);
    expect(dot).toBeDefined();
    const fill = log.slice(log.indexOf(dot!)).find((l) => l.k === 'fill');
    expect(fill?.fillStyle).toBe(ROBOT_COLORS[0]);
  });

  it('same_tank_twice_keeps_both_muzzle_colours', () => {
    const both = { k2 };
    const log = run([at(0), at(1)], { leopard2a8: both.k2, abrams: both.k2 });
    const fills = log.filter((l) => l.k === 'fillRect' && (l.fillStyle === ROBOT_COLORS[0] || l.fillStyle === ROBOT_COLORS[1])).map((l) => l.fillStyle);
    expect(fills).toEqual([ROBOT_COLORS[0], ROBOT_COLORS[1]]);
  });

  it('dead tank is grey with no muzzle colour', () => {
    const log = run([at(0, false)], k2Tanks);
    expect(log.some((l) => l.k === 'setFilter' && l.filter === 'grayscale(1) brightness(0.6)')).toBe(true);
    expect(log.filter((l) => l.k === 'drawImage')).toHaveLength(2);
    expect(log.some((l) => l.k === 'fillRect' && l.fillStyle === ROBOT_COLORS[0])).toBe(false);
  });

  it('falls_back_to_plain_tank_before_images_load', () => {
    let log: Rec[] = [];
    expect(() => (log = run([at(0)], {}))).not.toThrow();
    expect(log.some((l) => l.k === 'roundRect')).toBe(true);
    expect(log.some((l) => l.k === 'drawImage')).toBe(false);
  });

  it('user sprites still win', () => {
    const body = fakeImg('body', 36, 36);
    const log = run([at(0)], k2Tanks, { body });
    const imgs = log.filter((l) => l.k === 'drawImage').map((l) => l.args[0]);
    expect(imgs).toContain(body);
    expect(imgs).not.toContain(k2.hull);
    expect(imgs).not.toContain(k2.turret);
  });

  it('turret and muzzle sit on the turret ring', () => {
    const def = TANKS.abrams;
    const ringX = (def.art.hull.pivotX - def.art.hull.w / 2) * def.scale;
    expect(Math.abs(ringX)).toBeGreaterThan(0.5);
    const abrams = { hull: fakeImg('hull', 1, 1), turret: fakeImg('turret', 1, 1) };
    const r = { ...at(1), heading: 0, gun_heading: 90 };
    const log = run([at(0), r], { abrams });
    const ops = log.filter((l) => l.k === 'translate' || l.k === 'rotate' || l.k === 'drawImage' || l.k === 'fillRect');
    const start = ops.findIndex((l) => l.k === 'translate' && l.args[0] === r.position.x);
    const mine = ops.slice(start);
    const turretAt = mine.findIndex((l) => l.k === 'drawImage' && l.args[0] === abrams.turret);
    const ring = mine.slice(0, turretAt).filter((l) => l.k === 'translate').pop()!;
    expect(ring.args[0]).toBeCloseTo(ringX);
    expect(ring.args[1]).toBeCloseTo(-def.pivotY);
    const capAt = mine.findIndex((l) => l.k === 'fillRect' && l.args[2] === 4);
    const capMove = mine.slice(0, capAt).filter((l) => l.k === 'translate').pop()!;
    expect(capMove.args[0]).toBeCloseTo(0);
    expect(capMove.args[1]).toBeCloseTo(-(def.barrelReach - def.pivotY));
    const gunTurn = mine.slice(0, capAt).filter((l) => l.k === 'rotate').pop()!;
    expect(gunTurn.args[0]).toBeCloseTo(Math.PI / 2);
  });
});

describe('shot look', () => {
  type Call = { k: string; args: number[]; strokeStyle: unknown; fillStyle: unknown };
  const run = (turn: number, effects = true) => {
    const calls: Call[] = [];
    const noop = () => undefined;
    const ctx = new Proxy({} as Record<string, unknown>, {
      get: (t, k: string) => {
        if (k === 'measureText') return () => ({ width: 10 });
        if (['arc', 'moveTo', 'lineTo', 'stroke', 'fill'].includes(k))
          return (...a: number[]) => calls.push({ k, args: a, strokeStyle: t.strokeStyle, fillStyle: t.fillStyle });
        return k in t ? t[k] : noop;
      },
      set: (t, k: string, v) => ((t[k] = v), true),
    }) as unknown as CanvasRenderingContext2D;
    const shooter = { ...robot(90), id: 0, position: { x: 100, y: 300 }, gun_heading: 90 };
    const victim = { ...robot(90), id: 1, position: { x: 700, y: 300 } };
    const b = (x: number) => ({ id: 4, owner_id: 0, position: { x, y: 300 }, heading: 90, power: 1, speed: 300, alive: true });
    const ev = (over: object = {}) => ({ bullet_fired: [], bullet_hits: [], wall_collisions: [], robot_collisions: [], robot_deaths: [], ...over });
    const rec = {
      battlefield: { width: 800, height: 600 },
      robots: [{ id: 0, name: 'A' }, { id: 1, name: 'B' }],
      turns: [
        { turn: 0, robots: [shooter, victim], bullets: [], events: ev() },
        { turn: 1, robots: [shooter, victim], bullets: [b(150)], events: ev() },
        { turn: 2, robots: [shooter, victim], bullets: [b(450)], events: ev() },
        { turn: 3, robots: [shooter, victim], bullets: [], events: ev({ bullet_hits: [{ shooter: 0, victim: 1, bullet_id: 4, power: 1 }] }) },
        ...[4, 5, 6, 7].map((t) => ({ turn: t, robots: [shooter, victim], bullets: [], events: ev() })),
      ],
    } as unknown as Recording;
    drawArena(ctx, rec, turn, { names: false, energy: false, scanArcs: false, effects });
    return calls;
  };
  /** Segments stroked in `color`: each moveTo then lineTo before that stroke. */
  const lines = (calls: Call[], color: string) => {
    const out: number[][] = [];
    let path: number[] = [];
    for (const c of calls) {
      if (c.k === 'moveTo') path = [c.args[0], c.args[1]];
      if (c.k === 'lineTo') path.push(c.args[0], c.args[1]);
      if (c.k === 'stroke' && c.strokeStyle === color && path.length === 4) out.push(path);
    }
    return out;
  };
  const CORE = '#fff6c8';
  const GLOW = 'rgba(255,200,80,0.35)';

  it('draws the tracer from the previous recorded position', () => {
    const calls = run(2);
    expect(lines(calls, CORE)).toEqual([[150, 300, 450, 300]]);
    expect(lines(calls, GLOW)).toEqual([[150, 300, 450, 300]]);
  });

  it('starts the tracer at the muzzle on the first turn, with a muzzle flash', () => {
    const calls = run(1);
    const reach = 20; // no tank images loaded: the plain tank's muzzle
    const [seg] = lines(calls, CORE);
    expect(seg[0]).toBeCloseTo(100 + reach);
    expect(seg.slice(1)).toEqual([300, 150, 300]);
    expect(calls.some((c) => c.k === 'arc' && c.args[2] === 6 && Math.abs(c.args[0] - (100 + reach)) < 1e-9)).toBe(true);
  });

  it('keeps only the tracer core with effects off', () => {
    const calls = run(2, false);
    expect(lines(calls, CORE)).toHaveLength(1);
    expect(lines(calls, GLOW)).toHaveLength(0);
  });

  const flash = (calls: Call[]) => calls.filter((c) => c.k === 'fill' && String(c.fillStyle).startsWith('rgba(255, 240, 190'));
  it('flashes the impact on the hit turn and fades it out after 4 turns', () => {
    expect(flash(run(2))).toHaveLength(0);
    expect(flash(run(3))).toHaveLength(1);
    expect(flash(run(6))).toHaveLength(1);
    expect(flash(run(7))).toHaveLength(0);
    expect(flash(run(3, false))).toHaveLength(0);
  });
});
