// Draws one turn of a recording onto a canvas. The look is RoboVerus's own:
// an ink-blue plotting table with instrument-style robots.
import { drawMap } from './maplook';
import { drawTokyo, isCity } from './tokyo';
import { scanArc } from './geometry';
import { bulletSegment, impactFlashes } from './scene';
import type { Decision, Recording, RobotSnapshot } from './recording';
import type { Sprites } from './sprites';
import { TANKS, tankFor, type TankId } from './tanks/defs';
import type { TankImages } from './tanks/images';

export interface ViewOptions {
  /** Procedural ground, road and roof textures instead of flat colours (default off). */
  textures?: boolean;
  /** Daylight palette (light ground, dark labels) instead of the night one (default off). */
  daylight?: boolean;
  names: boolean;
  energy: boolean;
  scanArcs: boolean;
  effects: boolean;
}

export const ROBOT_COLORS = ['#e0a526', '#3fb6c9', '#ef5d79', '#9c7cf4'];

const ARENA = '#0f1e2e';
const GRID = '#1c3247';
const WRECK = '#4a5866';
const STEEL = '#e9eef2';
const LABEL = '#c9d6e2';
const EFFECT_TURNS = 8;
const BODY = 36;
const TRACER_GLOW = 'rgba(255,200,80,0.35)';
const TRACER_CORE = '#fff6c8';
const ROAD = '#1e2b3a';
const BUILDING = '#34465c';
const BUILDING_EDGE = '#56708c';

/** The two palettes the 2D drawing switches between: night (the default) and daylight. */
const NIGHT = { arena: ARENA, grid: GRID, road: ROAD, building: BUILDING, edge: BUILDING_EDGE, label: LABEL, tracerGlow: TRACER_GLOW, tracerCore: TRACER_CORE };
const DAY = {
  arena: '#668f4c', grid: 'rgba(20, 30, 20, 0.10)', road: '#7b828c', building: '#d4d7dd', edge: '#8e95a1', label: '#1b2330',
  tracerGlow: 'rgba(217, 72, 15, 0.3)', tracerCore: '#d9480f',
};
let P = NIGHT;
const MUZZLE_FLASH_TURNS = 2;
const IMPACT_TURNS = 4;
/** Muzzle distance of the plain (sprite or vector) robot's gun. */
const PLAIN_REACH = 20;
const SPARK_ANGLES = [0, 1, 2, 3, 4, 5].map((k) => (k * Math.PI) / 3 + 0.3);

const rad = (deg: number) => (deg * Math.PI) / 180;

/** The text under a robot: name plus energy, "Disabled" at 0, or "Dead". */
export function robotLabel(r: RobotSnapshot, name: string): string {
  if (!r.alive) return `${name} Dead`;
  if (r.energy <= 0) return `${name} Disabled`;
  return `${name} ${r.energy.toFixed(1)}`;
}

/** Horizontal centre for a label `textWidth` wide, kept inside the arena. */
export function labelX(x: number, width: number, textWidth: number): number {
  const half = textWidth / 2 + 4;
  return Math.min(Math.max(x, half), width - half);
}

/** Where a robot's label goes: below it, or above it near the bottom edge. */
export function labelY(y: number, height: number): number {
  const below = y + BODY / 2 + 14;
  return below > height - 4 ? y - BODY / 2 - 6 : below;
}

const LINE = 12;

/** "ATTACK · normal", or '' when the robot reports no intent. */
export function intentLabel(d: Decision | null | undefined): string {
  return d?.intent ? `${d.intent.maneuver.toUpperCase()} · ${d.intent.risk}` : '';
}

function labelFor(r: RobotSnapshot, name: string, view: ViewOptions): string {
  if (view.names && view.energy) return robotLabel(r, name);
  if (view.names) return name;
  if (view.energy) return robotLabel(r, '').trim();
  return '';
}

function drawGrid(ctx: CanvasRenderingContext2D, w: number, h: number) {
  ctx.fillStyle = P.arena;
  ctx.fillRect(0, 0, w, h);
  ctx.strokeStyle = P.grid;
  ctx.lineWidth = 1;
  ctx.beginPath();
  for (let x = 50; x < w; x += 50) {
    ctx.moveTo(x + 0.5, 0);
    ctx.lineTo(x + 0.5, h);
  }
  for (let y = 50; y < h; y += 50) {
    ctx.moveTo(0, y + 0.5);
    ctx.lineTo(w, y + 0.5);
  }
  ctx.stroke();
}

/** Radius of the team-colour dot on a tank's turret ring. */
const TEAM_DOT = 3;
const MUZZLE_W = 4;
const MUZZLE_H = 3;

/** One of the five tanks: hull, turret, radar dish, then the team-colour muzzle cap. */
function drawTank(ctx: CanvasRenderingContext2D, r: RobotSnapshot, color: string, id: TankId, imgs: TankImages[TankId] & object) {
  const def = TANKS[id];
  const s = def.scale;
  ctx.save();
  ctx.translate(r.position.x, r.position.y);
  if (!r.alive) {
    // Opaque, darkened grey: with transparency the hull showed through the turret.
    ctx.filter = 'grayscale(1) brightness(0.6)';
  }

  // Hull: centred on the robot, rotated by heading.
  ctx.save();
  ctx.rotate(rad(r.heading));
  ctx.drawImage(imgs.hull, (-def.art.hull.w * s) / 2, (-def.art.hull.h * s) / 2, def.art.hull.w * s, def.art.hull.h * s);
  ctx.restore();

  // Turret: pivots about the turret ring centre (pivotX off the hull axis, pivotY ahead), then turns with the gun.
  const ringX = (def.art.hull.pivotX - def.art.hull.w / 2) * s;
  const toRing = () => {
    ctx.rotate(rad(r.heading));
    ctx.translate(ringX, -def.pivotY);
  };
  ctx.save();
  toRing();
  ctx.rotate(rad(r.gun_heading - r.heading));
  const t = def.art.turret;
  ctx.drawImage(imgs.turret, -t.pivotX * s, -t.pivotY * s, t.w * s, t.h * s);
  ctx.restore();

  // Team marker: a small dot in the team colour on the turret ring.
  ctx.save();
  toRing();
  ctx.fillStyle = r.alive ? color : WRECK;
  ctx.beginPath();
  ctx.arc(0, 0, TEAM_DOT, 0, Math.PI * 2);
  ctx.fill();
  ctx.restore();

  // Muzzle cap, also in the team colour (as is the team dot above).
  if (r.alive) {
    ctx.save();
    toRing();
    ctx.rotate(rad(r.gun_heading - r.heading));
    ctx.translate(0, -(def.barrelReach - def.pivotY));
    ctx.fillStyle = color;
    ctx.fillRect(-MUZZLE_W / 2, 0, MUZZLE_W, MUZZLE_H);
    ctx.restore();
  }
  ctx.restore();
}

function drawRobot(
  ctx: CanvasRenderingContext2D,
  r: RobotSnapshot,
  color: string,
  sprites: Sprites,
  tank: TankId,
  tanks: TankImages,
) {
  const imgs = tanks[tank];
  if (imgs && !sprites.body) return drawTank(ctx, r, color, tank, imgs);
  const tint = r.alive ? color : WRECK;
  ctx.save();
  ctx.translate(r.position.x, r.position.y);
  ctx.globalAlpha = r.alive ? 1 : 0.55;
  const scale = sprites.body ? BODY / sprites.body.naturalWidth : undefined;

  // Body with track marks.
  ctx.save();
  ctx.rotate(rad(r.heading));
  if (sprites.body && scale) {
    const img = sprites.body;
    ctx.drawImage(img, (-img.naturalWidth * scale) / 2, (-img.naturalHeight * scale) / 2, img.naturalWidth * scale, img.naturalHeight * scale);
  } else {
    ctx.fillStyle = tint;
    ctx.beginPath();
    ctx.roundRect(-BODY / 2, -BODY / 2, BODY, BODY, 7);
    ctx.fill();
    ctx.fillStyle = 'rgba(15, 30, 46, 0.45)';
    for (const side of [-BODY / 2, BODY / 2 - 7]) {
      for (let y = -BODY / 2 + 3; y < BODY / 2 - 2; y += 6) ctx.fillRect(side, y, 7, 3);
    }
  }
  ctx.restore();

  // Gun.
  ctx.save();
  ctx.rotate(rad(r.gun_heading));
  if (sprites.gun) {
    const img = sprites.gun;
    const s = scale ?? BODY / img.naturalWidth;
    ctx.drawImage(img, (-img.naturalWidth * s) / 2, (-img.naturalHeight * s) / 2, img.naturalWidth * s, img.naturalHeight * s);
  } else {
    ctx.strokeStyle = STEEL;
    ctx.lineWidth = 4;
    ctx.lineCap = 'round';
    ctx.beginPath();
    ctx.moveTo(0, 0);
    ctx.lineTo(0, -27);
    ctx.stroke();
    ctx.fillStyle = P.arena;
    ctx.beginPath();
    ctx.arc(0, 0, 8, 0, Math.PI * 2);
    ctx.fill();
    ctx.strokeStyle = STEEL;
    ctx.lineWidth = 2;
    ctx.stroke();
  }
  ctx.restore();

  // Radar dish.
  ctx.save();
  ctx.rotate(rad(r.radar_heading));
  if (sprites.radar) {
    const img = sprites.radar;
    const s = scale ?? BODY / img.naturalWidth;
    ctx.drawImage(img, (-img.naturalWidth * s) / 2, (-img.naturalHeight * s) / 2, img.naturalWidth * s, img.naturalHeight * s);
  } else {
    ctx.strokeStyle = tint;
    ctx.lineWidth = 2.5;
    ctx.beginPath();
    ctx.arc(0, 4, 9, rad(-140), rad(-40));
    ctx.stroke();
  }
  ctx.restore();
  ctx.restore();
}


/** Roads, then buildings. Map objects only: they are drawn under the robots. */
function drawTerrain(ctx: CanvasRenderingContext2D, terrain: Recording['terrain']) {
  if (!terrain) return;
  ctx.fillStyle = P.road;
  for (const [x, y, w, h] of terrain.roads) ctx.fillRect(x, y, w, h);
  ctx.fillStyle = P.building;
  for (const [x, y, w, h] of terrain.buildings) ctx.fillRect(x, y, w, h);
  ctx.strokeStyle = P.edge;
  ctx.lineWidth = 1;
  for (const [x, y, w, h] of terrain.buildings) ctx.strokeRect(x + 0.5, y + 0.5, w - 1, h - 1);
}

/** Tracers, muzzle flashes and impact flashes. Only the tracer core ignores `view.effects`. */
function drawShots(ctx: CanvasRenderingContext2D, rec: Recording, turn: number, view: ViewOptions, reachFor: (i: number) => number) {
  ctx.save();
  ctx.lineCap = 'round';
  if (view.effects) {
    // Muzzle flashes: a bright disc on the fire turn, fading over the next.
    for (let k = Math.max(0, turn - MUZZLE_FLASH_TURNS + 1); k <= turn; k++) {
      const fade = 1 - (turn - k) / MUZZLE_FLASH_TURNS;
      rec.turns[k].bullets.forEach((b, j) => {
        if (!b.alive) return;
        const s = bulletSegment(rec, k, j, reachFor);
        if (!s.first) return;
        ctx.fillStyle = `rgba(255, 250, 220, ${fade})`;
        ctx.beginPath();
        ctx.arc(s.from.x, s.from.y, 6, 0, Math.PI * 2);
        ctx.fill();
      });
    }
    // Impact flashes: an expanding disc plus six sparks, fading out.
    for (const f of impactFlashes(rec, turn, IMPACT_TURNS)) {
      const fade = 1 - f.age / IMPACT_TURNS;
      const r = 6 + f.power * 2 + f.age * 4;
      ctx.fillStyle = `rgba(255, 240, 190, ${0.85 * fade})`;
      ctx.beginPath();
      ctx.arc(f.x, f.y, r, 0, Math.PI * 2);
      ctx.fill();
      ctx.strokeStyle = `rgba(255, 220, 120, ${fade})`;
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      for (const a of SPARK_ANGLES) {
        ctx.moveTo(f.x + Math.cos(a) * r, f.y + Math.sin(a) * r);
        ctx.lineTo(f.x + Math.cos(a) * (r + 6), f.y + Math.sin(a) * (r + 6));
      }
      ctx.stroke();
    }
  }
  rec.turns[turn].bullets.forEach((b, j) => {
    if (!b.alive) return;
    const { from, to } = bulletSegment(rec, turn, j, reachFor);
    const layers: [string, number][] = view.effects ? [[TRACER_GLOW, 4], [TRACER_CORE, 1.5]] : [[TRACER_CORE, 1.5]];
    for (const [color, width] of layers) {
      ctx.strokeStyle = color;
      ctx.lineWidth = width;
      ctx.beginPath();
      ctx.moveTo(from.x, from.y);
      ctx.lineTo(to.x, to.y);
      ctx.stroke();
    }
  });
  ctx.restore();
}

const staticLayers = new WeakMap<Recording, { key: string; canvas: HTMLCanvasElement }>();

/** Ground, map and scenery: the same every frame. With scenery on it is drawn once into an offscreen canvas and copied
 *  (hundreds of shaded trees and bushes per frame made the replay crawl); otherwise it is drawn directly. */
function drawStatic(ctx: CanvasRenderingContext2D, rec: Recording, view: ViewOptions) {
  const { width: w, height: h } = rec.battlefield;
  const paint = (c: CanvasRenderingContext2D) => {
    if (view.textures) return isCity(rec) ? drawTokyo(c, rec, !!view.daylight) : drawMap(c, rec, !!view.daylight);
    drawGrid(c, w, h);
    drawTerrain(c, rec.terrain);
  };
  if (!view.textures || typeof document === 'undefined') return paint(ctx);
  const t = ctx.getTransform();
  const scale = Math.max(1, Math.hypot(t.a, t.b));
  const key = `${!!view.daylight}|${scale.toFixed(2)}`;
  let layer = staticLayers.get(rec);
  if (!layer || layer.key !== key) {
    const canvas = document.createElement('canvas');
    canvas.width = Math.ceil(w * scale);
    canvas.height = Math.ceil(h * scale);
    const c = canvas.getContext('2d');
    if (!c) return paint(ctx);
    c.scale(scale, scale);
    paint(c);
    layer = { key, canvas };
    staticLayers.set(rec, layer);
  }
  ctx.drawImage(layer.canvas, 0, 0, w, h);
}

export function drawArena(
  ctx: CanvasRenderingContext2D,
  rec: Recording,
  turn: number,
  view: ViewOptions,
  sprites: Sprites = {},
  tanks: TankImages = {},
): void {
  const { width: w, height: h } = rec.battlefield;
  const snap = rec.turns[turn];
  const prev = rec.turns[Math.max(turn - 1, 0)];
  P = view.daylight ? DAY : NIGHT;
  drawStatic(ctx, rec, view);

  if (view.scanArcs) {
    snap.robots.forEach((r, i) => {
      if (!r.alive || turn === 0) return;
      const color = ROBOT_COLORS[i % ROBOT_COLORS.length];
      if (r.beam) {
        // Recorded beam (v2): absolute arena coordinates, drawn as is.
        if (r.beam.length < 3) return;
        ctx.save();
        ctx.fillStyle = `${color}12`;
        ctx.beginPath();
        r.beam.forEach(([bx, by], k) => (k === 0 ? ctx.moveTo(bx, by) : ctx.lineTo(bx, by)));
        ctx.closePath();
        ctx.fill();
        ctx.restore();
        return;
      }
      const { start, sweep } = scanArc(prev.robots[i].radar_heading, r.radar_heading);
      ctx.save();
      ctx.translate(r.position.x, r.position.y);
      if (Math.abs(sweep) < rad(0.5)) {
        ctx.strokeStyle = `${color}40`;
        ctx.beginPath();
        ctx.moveTo(0, 0);
        ctx.lineTo(Math.cos(start) * 1200, Math.sin(start) * 1200);
        ctx.stroke();
      } else {
        ctx.fillStyle = `${color}12`;
        ctx.beginPath();
        ctx.moveTo(0, 0);
        ctx.arc(0, 0, 1200, start, start + sweep, sweep < 0);
        ctx.closePath();
        ctx.fill();
      }
      ctx.restore();
    });
  }

  if (view.effects) {
    for (let k = Math.max(1, turn - EFFECT_TURNS + 1); k <= turn; k++) {
      const age = turn - k;
      const fade = 1 - age / EFFECT_TURNS;
      const at = rec.turns[k];
      for (const id of at.events.robot_deaths) {
        const dead = at.robots.find((r) => r.id === id);
        if (!dead) continue;
        ctx.fillStyle = `rgba(239, 93, 121, ${fade * 0.35})`;
        ctx.beginPath();
        ctx.arc(dead.position.x, dead.position.y, 20 + age * 8, 0, Math.PI * 2);
        ctx.fill();
      }
    }
  }

  // Dead robots first so living ones draw on top.
  const order = snap.robots.map((r, i) => [r, i] as const).sort(([a], [b]) => Number(a.alive) - Number(b.alive));
  for (const [r, i] of order) drawRobot(ctx, r, ROBOT_COLORS[i % ROBOT_COLORS.length], sprites, tankFor(rec, i), tanks);

  drawShots(ctx, rec, turn, view, (i) => (tanks[tankFor(rec, i)] && !sprites.body ? TANKS[tankFor(rec, i)].barrelReach : PLAIN_REACH));

  if (view.names || view.energy) {
    ctx.font = '11px system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillStyle = P.label;
    snap.robots.forEach((r, i) => {
      const text = labelFor(r, rec.robots[i]?.name ?? `Robot ${i}`, view);
      const intent = r.alive ? intentLabel(rec.decisions?.[turn]?.[i]) : '';
      // With an intent line, leave room for it: it goes under the name, or
      // the pair moves above the robot near the bottom edge.
      const base = labelY(r.position.y, h - (intent ? LINE : 0));
      const above = base < r.position.y;
      if (text) {
        ctx.fillText(text, labelX(r.position.x, w, ctx.measureText(text).width), above && intent ? base - LINE : base);
      }
      if (intent) {
        ctx.font = '10px ui-monospace, monospace';
        ctx.fillText(intent, labelX(r.position.x, w, ctx.measureText(intent).width), text && !above ? base + LINE : base);
        ctx.font = '11px system-ui, sans-serif';
      }
    });
  }
}

