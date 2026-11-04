// Mirrors the Verus spec `safe_to_fire` from src/robots/safe_shooter.rs, so
// the viewer can show which firing conditions held on a given turn.
import type { Decision } from './recording';

/** Largest misalignment that counts as aimed, in decidegrees (3.0°). */
export const AIM_TOLERANCE = 30;

/** Signed shortest turn in decidegrees within (-1800, 1800], like `units::angle_diff`. */
export function angleDiff(to: number, from: number): number {
  const d = to - from;
  if (d > 1800) return d - 3600;
  if (d <= -1800) return d + 3600;
  return d;
}

export interface Check {
  label: string;
  ok: boolean;
}

/** The conditions of `safe_to_fire` for firing with `power` (milli-units). */
export function safetyChecks(d: Decision, power: number): Check[] {
  const t = d.target;
  return [
    { label: 'power is 0.1 to 3.0', ok: power >= 100 && power <= 3000 },
    { label: 'energy > power', ok: d.energy > power },
    { label: 'gun is cool', ok: d.gun_heat === 0 },
    { label: 'target seen this turn', ok: t?.age === 0 },
    { label: 'aimed within 3.0°', ok: t !== null && Math.abs(angleDiff(t.bearing, d.gun_heading)) <= AIM_TOLERANCE },
  ];
}
