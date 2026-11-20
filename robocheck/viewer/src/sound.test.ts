import { describe, expect, it } from 'vitest';
import { cuesForTurn, engineLevels, enginePans } from '../../../web/src/sound';
import type { Recording } from '../../../web/src/recording';

const robot = (id: number, x: number, velocity: number, alive = true) => ({ id, position: { x, y: 100 }, velocity, alive });
const events = (e: Record<string, unknown>) => ({
  bullet_fired: [], bullet_hits: [], wall_collisions: [], robot_collisions: [], robot_deaths: [], ...e,
});
const rec = {
  battlefield: { width: 1000, height: 600 },
  turns: [
    { turn: 0, robots: [robot(0, 0, 0), robot(1, 1000, 8)], bullets: [], events: events({}) },
    {
      turn: 1,
      robots: [robot(0, 250, 4), robot(1, 750, -8, false)],
      bullets: [],
      events: events({
        bullet_fired: [{ shooter: 0, bullet_id: 1, power: 3 }],
        bullet_hits: [{ shooter: 0, victim: 1, bullet_id: 1, power: 3 }],
        bullet_hit_obstacles: [{ bullet_id: 2, owner: 0, power: 1, position: { x: 500, y: 300 } }],
        wall_collisions: [{ robot_id: 0, position: { x: 0, y: 50 } }],
        robot_collisions: [
          { robot_id: 0, other_robot_id: 1, at_fault: true },
          { robot_id: 1, other_robot_id: 0, at_fault: false },
        ],
        robot_deaths: [1],
      }),
    },
  ],
} as unknown as Recording;

describe('battle sounds', () => {
  it('a quiet turn makes no sound', () => {
    expect(cuesForTurn(rec, 0)).toEqual([]);
    expect(cuesForTurn(rec, 99)).toEqual([]);
  });

  it('every kind of event gives one cue, panned by where it happened', () => {
    const cues = cuesForTurn(rec, 1);
    expect(cues.map((c) => c.kind).sort()).toEqual(['collision', 'death', 'fire', 'hit', 'obstacle', 'wall']);
    const by = (k: string) => cues.find((c) => c.kind === k)!;
    expect(by('fire').pan).toBeCloseTo(-0.4);   // the shooter at x = 250 of 1000
    expect(by('hit').pan).toBeCloseTo(0.4);     // the victim at x = 750
    expect(by('obstacle').pan).toBeCloseTo(0);  // the building struck at x = 500
    expect(by('wall').pan).toBeCloseTo(-0.8);   // the left wall
    expect(by('fire').power).toBe(3);
  });

  it('a collision sounds once, not once per tank', () => {
    expect(cuesForTurn(rec, 1).filter((c) => c.kind === 'collision')).toHaveLength(1);
  });

  it('engine loudness follows speed and stops for a destroyed tank', () => {
    expect(engineLevels(rec, 0)).toEqual([0, 1]);
    expect(engineLevels(rec, 1)).toEqual([0.5, 0]);
    expect(enginePans(rec, 0)).toEqual([-0.8, 0.8]);
  });
});
