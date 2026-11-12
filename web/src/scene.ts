// Pure mapping from battle snapshots to 3D poses. No three.js here, so it is
// unit-tested without WebGL.
import type { Recording, RobotSnapshot } from './recording';

/** Height of a building in the 3D view, in battlefield pixels. */
export const BUILDING_HEIGHT = 28;
/** Height of a tank's hull. */
export const HULL_HEIGHT = 10;

const toRad = (deg: number) => (deg * Math.PI) / 180;

/**
 * Battlefield pixels to the 3D world, centred on the arena: x stays x and
 * the screen's y becomes the world's z (so "north", at y = 0, is -z).
 */
export function worldPoint(x: number, y: number, width: number, height: number): { x: number; z: number } {
  return { x: x - width / 2, z: y - height / 2 };
}

/**
 * Rotation about the vertical axis that points a tank's nose (local +z) along
 * a heading. Headings are clockwise from north, as in the battle.
 */
export function headingRotation(deg: number): number {
  return Math.PI - toRad(deg);
}

/**
 * Rotation of a part (turret, gun, radar) relative to its parent hull, so the
 * part points along `absoluteDeg` in the battle.
 */
export function relativeRotation(absoluteDeg: number, hullDeg: number): number {
  return -toRad(absoluteDeg - hullDeg);
}

export interface TankPose {
  x: number;
  z: number;
  hull: number;
  gun: number;
  radar: number;
  alive: boolean;
}

export function tankPose(r: RobotSnapshot, width: number, height: number): TankPose {
  const p = worldPoint(r.position.x, r.position.y, width, height);
  return {
    x: p.x,
    z: p.z,
    hull: headingRotation(r.heading),
    gun: relativeRotation(r.gun_heading, r.heading),
    radar: relativeRotation(r.radar_heading, r.heading),
    alive: r.alive,
  };
}

/** The map objects of a recording, as world-space boxes (`x, z, width, depth`). */
export function terrainBoxes(rec: Recording): {
  buildings: [number, number, number, number][];
  roads: [number, number, number, number][];
} {
  const { width, height } = rec.battlefield;
  const toBox = ([x, y, w, h]: [number, number, number, number]): [number, number, number, number] => {
    const c = worldPoint(x + w / 2, y + h / 2, width, height);
    return [c.x, c.z, w, h];
  };
  return {
    buildings: (rec.terrain?.buildings ?? []).map(toBox),
    roads: (rec.terrain?.roads ?? []).map(toBox),
  };
}
