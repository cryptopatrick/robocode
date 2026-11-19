// Pure mapping from battle snapshots to 3D poses. No three.js here, so it is
// unit-tested without WebGL.
import type { Point, Recording, RobotSnapshot } from './recording';
import { TANKS, tankFor } from './tanks/defs';

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

/**
 * Distance each robot has driven by each turn, `[turn][robot]`: the running sum
 * of the signed recorded velocity (turn 0 is zero), so reversing runs the tracks back.
 */
export function drivenDistances(rec: Recording): number[][] {
  const out: number[][] = [];
  let prev: number[] = [];
  rec.turns.forEach((turn, t) => {
    const row = turn.robots.map((r, i) => (t === 0 ? 0 : (prev[i] ?? 0) + r.velocity));
    out.push(row);
    prev = row;
  });
  return out;
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

/** Vertical field of view of the 3D camera, in degrees. */
export const CAMERA_FOV = 40;
/** Direction from the arena centre towards the camera (not normalised). */
const CAMERA_TILT: [number, number, number] = [0, 0.9, 0.55];
/** Spare room around the arena, as a factor on the fitted distance. */
const CAMERA_MARGIN = 1.05;

/**
 * Where to put the 3D camera so all four walls of a `width` x `height` arena
 * are in view at the given canvas aspect (width / height). The camera looks at
 * the arena centre from a fixed tilt; the distance is the smallest at which
 * every floor corner fits the vertical and the horizontal field of view.
 */
export function cameraFor(
  width: number,
  height: number,
  aspect: number,
): { position: [number, number, number]; target: [number, number, number]; fov: number } {
  const len = Math.hypot(...CAMERA_TILT);
  const dir = CAMERA_TILT.map((c) => c / len) as [number, number, number];
  // Camera right = up x back, normalised; up = back x right.
  const rl = Math.hypot(dir[2], dir[0]);
  const right: [number, number, number] = [dir[2] / rl, 0, -dir[0] / rl];
  const up: [number, number, number] = [
    dir[1] * right[2] - dir[2] * right[1],
    dir[2] * right[0] - dir[0] * right[2],
    dir[0] * right[1] - dir[1] * right[0],
  ];
  const dot = (a: number[], b: number[]) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
  const tanY = Math.tan((CAMERA_FOV * Math.PI) / 360);
  const tanX = tanY * aspect;
  let distance = 0;
  for (const sx of [-1, 1]) {
    for (const sz of [-1, 1]) {
      const p = [(sx * width) / 2, 0, (sz * height) / 2];
      const need = Math.max(
        dot(p, dir) + Math.abs(dot(p, right)) / tanX,
        dot(p, dir) + Math.abs(dot(p, up)) / tanY,
      );
      distance = Math.max(distance, need);
    }
  }
  distance *= CAMERA_MARGIN;
  return {
    position: [dir[0] * distance, dir[1] * distance, dir[2] * distance],
    target: [0, 0, 0],
    fov: CAMERA_FOV,
  };
}

/**
 * Identity of the 3D scene a recording needs: arena size, terrain, robots and
 * tanks. Appending live frames leaves it unchanged, so the scene is not rebuilt.
 */
export function sceneKey(rec: Recording): string {
  return JSON.stringify([rec.battlefield, rec.terrain ?? null, rec.robots, rec.tanks ?? null]);
}

/** The tip of a robot's gun: `reach` px from its centre along the gun heading. */
export function muzzlePoint(r: RobotSnapshot, reach: number): Point {
  const g = toRad(r.gun_heading);
  return { x: r.position.x + Math.sin(g) * reach, y: r.position.y - Math.cos(g) * reach };
}

/** Barrel reach of robot `i`'s tank (its default when the recording names none). */
const tankReach = (rec: Recording, i: number) => TANKS[tankFor(rec, i)].barrelReach;

/**
 * The path bullet `bulletIndex` of turn `turn` covered to get where it is:
 * from the same bullet id on the previous turn, or, on its first turn, from
 * its shooter's muzzle (`reachFor(robotIndex)` px out; the tank's barrel by
 * default). `first` marks that first turn.
 */
export function bulletSegment(
  rec: Recording,
  turn: number,
  bulletIndex: number,
  reachFor: (robotIndex: number) => number = (i) => tankReach(rec, i),
): { from: Point; to: Point; first: boolean } {
  const snap = rec.turns[turn];
  const b = snap.bullets[bulletIndex];
  const before = turn > 0 ? rec.turns[turn - 1].bullets.find((p) => p.id === b.id) : undefined;
  if (before) return { from: before.position, to: b.position, first: false };
  const i = snap.robots.findIndex((r) => r.id === b.owner_id);
  const from = i < 0 ? b.position : muzzlePoint(snap.robots[i], reachFor(i));
  return { from, to: b.position, first: true };
}

/** Where bullet `id` was recorded on the turn before `turn`, if it was. */
function lastSeen(rec: Recording, turn: number, id: number): Point | undefined {
  return rec.turns[turn - 1]?.bullets.find((p) => p.id === id)?.position;
}

export interface ImpactFlash extends Point {
  /** Turns since the hit (0 on the hit turn). */
  age: number;
  power: number;
}

/**
 * Impact flashes alive at `turn`, from hits up to `turns - 1` turns old.
 * A building hit flashes where it is recorded. A robot hit has no recorded
 * position (the bullet is gone on the hit turn), so it flashes on the
 * victim's edge facing the bullet's position on the previous turn, or the
 * shooter when it was fired and hit on the same turn.
 */
export function impactFlashes(rec: Recording, turn: number, turns: number): ImpactFlash[] {
  const out: ImpactFlash[] = [];
  for (let k = Math.max(1, turn - turns + 1); k <= turn; k++) {
    const at = rec.turns[k];
    const age = turn - k;
    for (const hit of at.events.bullet_hits ?? []) {
      const v = at.robots.find((r) => r.id === hit.victim);
      if (!v) continue;
      let { x, y } = v.position;
      // Fired and hit on the same turn: no recorded position, so face the shooter.
      const from = lastSeen(rec, k, hit.bullet_id) ?? at.robots.find((r) => r.id === hit.shooter)?.position;
      if (from) {
        const d = Math.hypot(from.x - x, from.y - y);
        const pull = Math.min(v.radius, d);
        if (d > 0) {
          x += ((from.x - x) / d) * pull;
          y += ((from.y - y) / d) * pull;
        }
      }
      out.push({ x, y, age, power: hit.power });
    }
    for (const hit of at.events.bullet_hit_obstacles ?? []) {
      out.push({ x: hit.position.x, y: hit.position.y, age, power: hit.power });
    }
  }
  return out;
}

/** Shortest 3D streak, so a bullet that has not moved still shows. */
const MIN_STREAK = 2;

/**
 * Pose of a box streak (unit box along local z) from `from` to `to` in
 * battlefield pixels: world centre, length, and rotation about the vertical.
 */
export function streakPose(from: Point, to: Point, width: number, height: number): { x: number; z: number; length: number; rotation: number } {
  const c = worldPoint((from.x + to.x) / 2, (from.y + to.y) / 2, width, height);
  const dx = to.x - from.x;
  const dz = to.y - from.y;
  return { x: c.x, z: c.z, length: Math.max(Math.hypot(dx, dz), MIN_STREAK), rotation: Math.atan2(dx, dz) };
}
