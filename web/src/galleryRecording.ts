// A tiny synthetic recording for the dev-only tank gallery: one tank id, an
// alive copy (turret turned 30 degrees off the hull) and a dead copy.
import type { Recording, RobotSnapshot } from './recording';
import type { TankId } from './tanks/defs';

export const GALLERY_WIDTH = 160;
export const GALLERY_HEIGHT = 100;

const robot = (id: number, x: number, alive: boolean): RobotSnapshot => ({
  id,
  position: { x, y: GALLERY_HEIGHT / 2 },
  heading: 0,
  gun_heading: 30,
  radar_heading: 30,
  velocity: 0,
  energy: alive ? 100 : 0,
  gun_heat: 0,
  alive,
  radius: 18,
});

export function galleryRecording(tank: TankId): Recording {
  return {
    format: 'roboverus-recording',
    version: 2,
    robots: [
      { id: 0, name: 'alive' },
      { id: 1, name: 'dead' },
    ],
    battlefield: { width: GALLERY_WIDTH, height: GALLERY_HEIGHT },
    tanks: [tank, tank],
    turns: [
      {
        turn: 0,
        robots: [robot(0, 45, true), robot(1, 115, false)],
        bullets: [],
        events: { bullet_fired: [], bullet_hits: [], wall_collisions: [], robot_collisions: [], robot_deaths: [] },
      },
    ],
    decisions: [[null, null]],
    outcome: { winner: null, turns: 1, energies: [100, 0] },
  };
}
