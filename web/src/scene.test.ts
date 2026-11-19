import { describe, expect, it } from 'vitest';
import { drivenDistances, sceneKey, headingRotation, relativeRotation, terrainBoxes, tankPose, worldPoint } from './scene';
import type { Recording, RobotSnapshot } from './recording';

const close = (a: number, b: number) => expect(a).toBeCloseTo(b, 9);

/** The world direction a rotation points a tank's nose in (x, z). */
const nose = (rot: number) => ({ x: Math.sin(rot), z: Math.cos(rot) });

describe('scene mapping', () => {
  it('centres the arena and makes the screen y the world z', () => {
    expect(worldPoint(400, 300, 800, 600)).toEqual({ x: 0, z: 0 });
    expect(worldPoint(0, 0, 800, 600)).toEqual({ x: -400, z: -300 });
  });

  it('points a hull north at heading 0, east at 90, south at 180', () => {
    const north = nose(headingRotation(0));
    close(north.x, 0);
    close(north.z, -1); // north is -z (up the screen)
    const east = nose(headingRotation(90));
    close(east.x, 1);
    close(east.z, 0);
    const south = nose(headingRotation(180));
    close(south.z, 1);
  });

  it('turns a turret relative to the hull', () => {
    // A gun pointing east while the hull points north: a quarter turn right of the hull.
    const rot = relativeRotation(90, 0);
    close(rot, -Math.PI / 2);
    // Hull and gun both east: no relative turn.
    close(relativeRotation(90, 90), 0);
  });

  it('gives the absolute gun direction once the hull rotation is applied', () => {
    const hull = headingRotation(30);
    const gun = hull + relativeRotation(200, 30);
    const direction = nose(gun);
    const expected = nose(headingRotation(200));
    close(direction.x, expected.x);
    close(direction.z, expected.z);
  });

  it('builds a tank pose from a snapshot', () => {
    const robot: RobotSnapshot = {
      id: 0,
      position: { x: 100, y: 200 },
      heading: 0,
      gun_heading: 90,
      radar_heading: 0,
      energy: 100,
      alive: true,
    } as RobotSnapshot;
    const pose = tankPose(robot, 800, 600);
    expect(pose.x).toBe(-300);
    expect(pose.z).toBe(-100);
    expect(pose.alive).toBe(true);
  });

  it('centres map objects in world space', () => {
    const rec = {
      battlefield: { width: 800, height: 600 },
      terrain: { buildings: [[0, 0, 100, 50]], roads: [[350, 0, 100, 600]] },
    } as unknown as Recording;
    const boxes = terrainBoxes(rec);
    expect(boxes.buildings[0]).toEqual([-350, -275, 100, 50]);
    expect(boxes.roads[0]).toEqual([0, 0, 100, 600]); // centred on the arena
    expect(terrainBoxes({ battlefield: { width: 1, height: 1 } } as Recording)).toEqual({ buildings: [], roads: [] });
  });
});

import * as THREE from 'three';
import { cameraFor } from './scene';

describe('cameraFor', () => {
  const sizes: [number, number][] = [[800, 600], [1200, 600], [1067, 600], [600, 600]];
  for (const aspect of [16 / 9, 1, 9 / 16]) {
    it(`keeps all four walls in view at aspect ${aspect.toFixed(2)}`, () => {
      for (const [w, h] of sizes) {
        const c = cameraFor(w, h, aspect);
        const cam = new THREE.PerspectiveCamera(c.fov, aspect, 1, 5000);
        cam.position.set(...c.position);
        cam.lookAt(...c.target);
        cam.updateMatrixWorld();
        cam.updateProjectionMatrix();
        for (const [x, y] of [[0, 0], [w, 0], [w, h], [0, h]]) {
          const p = worldPoint(x, y, w, h);
          const ndc = new THREE.Vector3(p.x, 0, p.z).project(cam);
          expect(Math.abs(ndc.x), `${w}x${h} x`).toBeLessThanOrEqual(1);
          expect(Math.abs(ndc.y), `${w}x${h} y`).toBeLessThanOrEqual(1);
        }
      }
    });
  }
});

describe('drivenDistances', () => {
  const rob = (velocity: number) => ({ velocity }) as RobotSnapshot;
  const rec = (rows: number[][]) => ({ turns: rows.map((r) => ({ robots: r.map(rob) })) }) as unknown as Recording;

  it('driven distance sums velocity', () => {
    const d = drivenDistances(rec([[0], [8], [8], [-2]]));
    expect(d.map((t) => t[0])).toEqual([0, 8, 16, 14]);
  });

  it('driven distance is per robot', () => {
    const d = drivenDistances(rec([[5, 1], [2, -3], [4, 0]]));
    expect(d[0]).toEqual([0, 0]);
    expect(d[2]).toEqual([6, -3]);
  });
});

describe('sceneKey', () => {
  const base = {
    robots: ['a', 'b'],
    battlefield: { width: 800, height: 600 },
    terrain: { buildings: [[0, 0, 10, 10]], roads: [] },
    tanks: ['abrams', 'k2'],
    turns: [{ robots: [] }],
  } as unknown as Recording;

  it('stays the same when frames are appended', () => {
    const more = { ...base, turns: [...base.turns, { robots: [] }] } as unknown as Recording;
    expect(sceneKey(more)).toBe(sceneKey(base));
  });

  it('changes with tanks, robots, size and terrain', () => {
    const k = sceneKey(base);
    expect(sceneKey({ ...base, tanks: ['k2', 'k2'] } as unknown as Recording)).not.toBe(k);
    expect(sceneKey({ ...base, robots: ['a'] } as unknown as Recording)).not.toBe(k);
    expect(sceneKey({ ...base, battlefield: { width: 900, height: 600 } } as unknown as Recording)).not.toBe(k);
    expect(sceneKey({ ...base, terrain: undefined } as unknown as Recording)).not.toBe(k);
  });
});

import { bulletSegment, impactFlashes, muzzlePoint } from './scene';
import { TANKS, tankFor } from './tanks/defs';

describe('bullet segments', () => {
  const shooter = (gun: number): RobotSnapshot => ({
    id: 7, position: { x: 100, y: 200 }, heading: 0, gun_heading: gun, radar_heading: 0,
    velocity: 0, energy: 90, gun_heat: 0, alive: true, radius: 18,
  });
  const bullet = (id: number, x: number, y: number) => ({ id, owner_id: 7, position: { x, y }, heading: 90, power: 1, speed: 300, alive: true });
  const ev = { bullet_fired: [], bullet_hits: [], wall_collisions: [], robot_collisions: [], robot_deaths: [] };
  const rec = {
    battlefield: { width: 800, height: 600 },
    robots: [{ id: 7, name: 'A' }],
    turns: [
      { turn: 0, robots: [shooter(90)], bullets: [], events: ev },
      { turn: 1, robots: [shooter(90)], bullets: [bullet(3, 150, 200)], events: ev },
      { turn: 2, robots: [shooter(90)], bullets: [bullet(9, 140, 200), bullet(3, 450, 200)], events: ev },
    ],
  } as unknown as Recording;

  it('puts the muzzle along the gun heading', () => {
    const p = muzzlePoint(shooter(90), 20);
    close(p.x, 120);
    close(p.y, 200);
    const n = muzzlePoint(shooter(0), 20);
    close(n.x, 100);
    close(n.y, 180);
  });

  it('runs from the same bullet id on the previous turn', () => {
    const s = bulletSegment(rec, 2, 1);
    expect(s.first).toBe(false);
    expect(s.from).toEqual({ x: 150, y: 200 });
    expect(s.to).toEqual({ x: 450, y: 200 });
  });

  it('runs from the muzzle on a bullet first turn', () => {
    const reach = TANKS[tankFor(rec, 0)].barrelReach;
    const s = bulletSegment(rec, 2, 0);
    expect(s.first).toBe(true);
    close(s.from.x, 100 + reach);
    close(s.from.y, 200);
    expect(s.to).toEqual({ x: 140, y: 200 });
    // On turn 1 there is no earlier bullet; a custom reach (the plain tank) is used.
    const plain = bulletSegment(rec, 1, 0, () => 20);
    close(plain.from.x, 120);
  });
});

describe('impact flash fallbacks', () => {
  const mk = (id: number, x: number, y: number) => ({ id, position: { x, y }, heading: 0, gun_heading: 0, radar_heading: 0, velocity: 0, energy: 50, gun_heat: 0, alive: true, radius: 18 });
  const ev = (over: object = {}) => ({ bullet_fired: [], bullet_hits: [], wall_collisions: [], robot_collisions: [], robot_deaths: [], ...over });
  const hit = { bullet_hits: [{ shooter: 0, victim: 1, bullet_id: 5, power: 2 }] };
  const robots = [mk(0, 400, 100), mk(1, 400, 300)];
  const old = { id: 5, owner_id: 0, position: { x: 100, y: 300 }, heading: 90, power: 2, speed: 300, alive: true };
  const mkRec = (turns: object[]) => ({ battlefield: { width: 800, height: 600 }, robots: [{ id: 0, name: 'S' }, { id: 1, name: 'V' }], turns }) as unknown as Recording;

  it('ignores a sighting older than the previous turn', () => {
    const rec = mkRec([
      { turn: 0, robots, bullets: [old], events: ev() },
      { turn: 1, robots, bullets: [], events: ev() },
      { turn: 2, robots, bullets: [], events: ev(hit) },
    ]);
    const f = impactFlashes(rec, 2, 4);
    close(f[0].x, 400);
    close(f[0].y, 282);
  });

  it('faces the shooter when the bullet was fired and hit on the same turn', () => {
    const rec = mkRec([
      { turn: 0, robots, bullets: [], events: ev() },
      { turn: 1, robots, bullets: [], events: ev(hit) },
    ]);
    const f = impactFlashes(rec, 1, 4);
    close(f[0].x, 400);
    close(f[0].y, 282);
  });
});

describe('impact flashes', () => {
  const victim = { id: 1, position: { x: 400, y: 300 }, heading: 0, gun_heading: 0, radar_heading: 0, velocity: 0, energy: 50, gun_heat: 0, alive: true, radius: 18 };
  const ev = (over: object = {}) => ({ bullet_fired: [], bullet_hits: [], wall_collisions: [], robot_collisions: [], robot_deaths: [], ...over });
  const turns = [
    { turn: 0, robots: [victim], bullets: [], events: ev() },
    { turn: 1, robots: [victim], bullets: [{ id: 5, owner_id: 0, position: { x: 100, y: 300 }, heading: 90, power: 2, speed: 300, alive: true }], events: ev() },
    {
      turn: 2, robots: [victim], bullets: [],
      events: ev({
        bullet_hits: [{ shooter: 0, victim: 1, bullet_id: 5, power: 2 }],
        bullet_hit_obstacles: [{ bullet_id: 6, owner: 0, power: 1, position: { x: 50, y: 60 } }],
      }),
    },
    ...[3, 4, 5, 6].map((t) => ({ turn: t, robots: [victim], bullets: [], events: ev() })),
  ];
  const rec = { battlefield: { width: 800, height: 600 }, robots: [{ id: 1, name: 'V' }], turns } as unknown as Recording;

  it('flashes a robot hit on the victim edge facing the shot, and an obstacle hit where it struck', () => {
    const f = impactFlashes(rec, 2, 4);
    expect(f).toHaveLength(2);
    expect(f[0]).toMatchObject({ age: 0, power: 2 });
    close(f[0].x, 400 - 18);
    close(f[0].y, 300);
    expect(f[1]).toMatchObject({ x: 50, y: 60, age: 0, power: 1 });
  });

  it('ages and then drops the flash after the fade turns', () => {
    expect(impactFlashes(rec, 5, 4).map((f) => f.age)).toEqual([3, 3]);
    expect(impactFlashes(rec, 6, 4)).toHaveLength(0);
  });
});

import { streakPose } from './scene';

describe('3D streak', () => {
  it('centres a streak between the two points, as long as the gap, pointing along it', () => {
    const s = streakPose({ x: 100, y: 300 }, { x: 400, y: 300 }, 800, 600);
    close(s.x, -150);
    close(s.z, 0);
    close(s.length, 300);
    // Local +z turned by rotation y lands on world +x (east).
    close(Math.sin(s.rotation), 1);
    close(Math.cos(s.rotation), 0);
  });
  it('keeps a minimum length for a bullet that has not moved', () => {
    const s = streakPose({ x: 10, y: 10 }, { x: 10, y: 10 }, 800, 600);
    expect(s.length).toBeGreaterThan(0);
  });
});
