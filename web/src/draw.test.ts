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

import { labelY } from './draw';

describe('labelY', () => {
  it('puts the label below the robot normally', () => {
    expect(labelY(300, 600)).toBe(300 + 18 + 14);
  });
  it('puts it above the robot near the bottom edge', () => {
    expect(labelY(576, 600)).toBe(576 - 18 - 6);
  });
});

import { labelX } from './draw';

describe('labelX', () => {
  it('centres the label on the robot when it fits', () => {
    expect(labelX(400, 800, 80)).toBe(400);
  });
  it('keeps the label inside the right edge', () => {
    expect(labelX(780, 800, 80)).toBe(800 - 40 - 4);
  });
  it('keeps the label inside the left edge', () => {
    expect(labelX(10, 800, 80)).toBe(40 + 4);
  });
});
