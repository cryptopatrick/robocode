// Draws one turn of a recording onto a canvas. The look is RoboVerus's own:
// an ink-blue plotting table with instrument-style robots.
import { scanArc } from './geometry';
import type { Recording, RobotSnapshot } from './recording';
import type { Sprites } from './sprites';

export interface ViewOptions {
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

const rad = (deg: number) => (deg * Math.PI) / 180;

/** The text under a robot: name plus energy, "Disabled" at 0, or "Dead". */
export function robotLabel(r: RobotSnapshot, name: string): string {
  if (!r.alive) return `${name} Dead`;
  if (r.energy <= 0) return `${name} Disabled`;
  return `${name} ${r.energy.toFixed(1)}`;
}

function labelFor(r: RobotSnapshot, name: string, view: ViewOptions): string {
  if (view.names && view.energy) return robotLabel(r, name);
  if (view.names) return name;
  if (view.energy) return robotLabel(r, '').trim();
  return '';
}

function drawGrid(ctx: CanvasRenderingContext2D, w: number, h: number) {
  ctx.fillStyle = ARENA;
  ctx.fillRect(0, 0, w, h);
  ctx.strokeStyle = GRID;
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

function drawRobot(ctx: CanvasRenderingContext2D, r: RobotSnapshot, color: string, sprites: Sprites) {
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
    ctx.fillStyle = ARENA;
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

/** Draws turn `turn` of `rec`. */
export function drawArena(
  ctx: CanvasRenderingContext2D,
  rec: Recording,
  turn: number,
  view: ViewOptions,
  sprites: Sprites = {},
): void {
  const { width: w, height: h } = rec.battlefield;
  const snap = rec.turns[turn];
  const prev = rec.turns[Math.max(turn - 1, 0)];
  drawGrid(ctx, w, h);

  if (view.scanArcs) {
    snap.robots.forEach((r, i) => {
      if (!r.alive || turn === 0) return;
      const { start, sweep } = scanArc(prev.robots[i].radar_heading, r.radar_heading);
      const color = ROBOT_COLORS[i % ROBOT_COLORS.length];
      ctx.save();
      ctx.translate(r.position.x, r.position.y);
      if (Math.abs(sweep) < rad(0.5)) {
        ctx.strokeStyle = `${color}55`;
        ctx.beginPath();
        ctx.moveTo(0, 0);
        ctx.lineTo(Math.cos(start) * 1200, Math.sin(start) * 1200);
        ctx.stroke();
      } else {
        ctx.fillStyle = `${color}22`;
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
      for (const hit of at.events.bullet_hits) {
        const victim = at.robots.find((r) => r.id === hit.victim);
        if (!victim) continue;
        ctx.strokeStyle = `rgba(255, 214, 120, ${fade})`;
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.arc(victim.position.x, victim.position.y, 12 + age * 4, 0, Math.PI * 2);
        ctx.stroke();
      }
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
  for (const [r, i] of order) drawRobot(ctx, r, ROBOT_COLORS[i % ROBOT_COLORS.length], sprites);

  ctx.fillStyle = STEEL;
  for (const b of snap.bullets) {
    if (!b.alive) continue;
    ctx.beginPath();
    ctx.arc(b.position.x, b.position.y, 1.5 + b.power, 0, Math.PI * 2);
    ctx.fill();
  }

  if (view.names || view.energy) {
    ctx.font = '11px system-ui, sans-serif';
    ctx.textAlign = 'center';
    ctx.fillStyle = LABEL;
    snap.robots.forEach((r, i) => {
      const text = labelFor(r, rec.robots[i]?.name ?? `Robot ${i}`, view);
      if (text) ctx.fillText(text, r.position.x, r.position.y + BODY / 2 + 14);
    });
  }
}

