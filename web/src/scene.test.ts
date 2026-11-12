import { describe, expect, it } from 'vitest';
import { headingRotation, relativeRotation, terrainBoxes, tankPose, worldPoint } from './scene';
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
