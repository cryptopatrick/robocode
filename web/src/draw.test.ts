import { describe, expect, it } from 'vitest';
import { robotLabel } from './draw';
import type { RobotSnapshot } from './recording';

const robot = (energy: number, alive = true): RobotSnapshot => ({
  id: 0, position: { x: 0, y: 0 }, heading: 0, gun_heading: 0, radar_heading: 0,
  velocity: 0, energy, gun_heat: 0, alive, radius: 18,
});

describe('robotLabel', () => {
  it('shows name and energy to one decimal', () => {
    expect(robotLabel(robot(94.04), 'SafeShooter')).toBe('SafeShooter 94.0');
  });
  it('shows Disabled at zero energy while alive', () => {
    expect(robotLabel(robot(0), 'SafeShooter')).toBe('SafeShooter Disabled');
  });
  it('shows Dead when not alive', () => {
    expect(robotLabel(robot(-3.5, false), 'SafeShooter')).toBe('SafeShooter Dead');
  });
});
