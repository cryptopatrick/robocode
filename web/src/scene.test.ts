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
