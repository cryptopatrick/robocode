// Types and parser for battle recordings produced by the `robocode-wasm` crate
// (format "roboverus-recording", versions 1 and 2). Units in snapshots are the
// engine's: pixels and degrees. Units in decisions are fixed-point: milli-units
// for energy/heat/power and decidegrees for angles.

export interface Point { x: number; y: number }

export type Rect = [number, number, number, number];

export interface RobotSnapshot {
  id: number;
  position: Point;
  heading: number;
  gun_heading: number;
  radar_heading: number;
  velocity: number;
  energy: number;
  gun_heat: number;
  alive: boolean;
  radius: number;
  /** Recording v2, battles with buildings: the radar's visible polygon in arena pixels. */
  beam?: [number, number][];
}

export interface BulletSnapshot {
  id: number;
  owner_id: number;
  position: Point;
  heading: number;
  power: number;
  speed: number;
  alive: boolean;
}

export interface TurnEvents {
  bullet_fired: { shooter: number; bullet_id: number; power: number }[];
  bullet_hits: { shooter: number; victim: number; bullet_id: number; power: number }[];
  wall_collisions: { robot_id: number; position: Point }[];
  robot_collisions: { robot_id: number; other_robot_id: number; at_fault: boolean }[];
  robot_deaths: number[];
  /** Only in battles with buildings. */
  bullet_hit_obstacles?: { bullet_id: number; owner: number; power: number; position: Point }[];
}

export interface TurnSnapshot {
  turn: number;
  robots: RobotSnapshot[];
  bullets: BulletSnapshot[];
  events: TurnEvents;
}

export interface Decision {
  energy: number;
  gun_heat: number;
  gun_heading: number;
  target: { bearing: number; distance: number; age: number } | null;
  fire: number | null;
  ahead: number | null;
  turn_body: number | null;
  turn_gun: number | null;
  turn_radar: number | null;
  /** Strategies that report one (Tactician): the maneuver and risk this turn. */
  intent?: { maneuver: string; risk: string };
}

export type BulletMode = 'realistic' | 'arcade';

/** The recording's bullet mode; a missing field means arcade (pre-0.7.0). */
export function bulletMode(rec: Pick<Recording, 'bullets'>): BulletMode {
  return rec.bullets === 'realistic' ? 'realistic' : 'arcade';
}

export interface Recording {
  format: 'roboverus-recording';
  version: 1 | 2;
  robots: { id: number; name: string }[];
  battlefield: { width: number; height: number };
  /** Map objects as [x, y, width, height]. Absent in an empty arena. */
  terrain?: {
    buildings: Rect[];
    roads: Rect[];
    /** A real-city map (see tokyo.ts): street polylines to draw, building heights in metres, scale, and credit. Drawing
     *  only; the engine never reads these. */
    road_lines?: { class: string; width: number; points: [number, number][] }[];
    heights?: number[];
    px_per_m?: number;
    name?: string;
    attribution?: string;
  };
  /** Tank id per robot, when the battle picked them. Unknown ids fall back to the slot default. */
  tanks?: string[];
  /** Bullet speed model. Absent in recordings made before bullet modes: those are arcade. */
  bullets?: BulletMode;
  turns: TurnSnapshot[];
  decisions: (Decision | null)[][];
  outcome: { winner: number | null; turns: number; energies: number[] };
}

/** Parses a recording, throwing an `Error` with a readable message if it isn't one. */
export function parseRecording(text: string): Recording {
  let data: unknown;
  try {
    data = JSON.parse(text);
  } catch {
    throw new Error('Not a JSON file');
  }
  if (typeof data !== 'object' || data === null || Array.isArray(data)) {
    throw new Error('Not a RoboVerus recording');
  }
  const rec = data as Partial<Recording> & { version?: unknown };
  if (rec.format !== 'roboverus-recording') {
    throw new Error('Not a RoboVerus recording');
  }
  if (rec.version !== 1 && rec.version !== 2) {
    throw new Error(`Unsupported recording version ${String(rec.version)}`);
  }
  if (
    !Array.isArray(rec.robots) ||
    !Array.isArray(rec.turns) ||
    rec.turns.length === 0 ||
    !Array.isArray(rec.decisions) ||
    rec.decisions.length !== rec.turns.length ||
    typeof rec.battlefield?.width !== 'number' ||
    typeof rec.outcome !== 'object'
  ) {
    throw new Error('Not a RoboVerus recording');
  }
  if (rec.bullets !== undefined && rec.bullets !== 'realistic' && rec.bullets !== 'arcade') {
    throw new Error(`Unknown bullet mode ${String(rec.bullets)}`);
  }
  return rec as Recording;
}
