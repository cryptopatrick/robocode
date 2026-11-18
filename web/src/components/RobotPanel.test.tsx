import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import RobotPanel from './RobotPanel';
import type { Decision, RobotSnapshot } from '../recording';

const robot: RobotSnapshot = {
  id: 0, position: { x: 0, y: 0 }, heading: 0, gun_heading: 0, radar_heading: 0,
  velocity: 0, energy: 80, gun_heat: 0, alive: true, radius: 18,
};

const dec = (maneuver: string): Decision => ({
  energy: 80_000, gun_heat: 0, gun_heading: 0, target: null, fire: null,
  ahead: null, turn_body: null, turn_gun: null, turn_radar: null,
  intent: { maneuver, risk: 'normal' },
});

describe('RobotPanel', () => {
  it('shows the maneuver and its length', () => {
    const rows = [
      ...Array.from({ length: 3 }, () => [dec('search')]),
      ...Array.from({ length: 14 }, () => [dec('attack')]),
    ];
    render(
      <RobotPanel names={['T']} robots={[robot]} selected={0} onSelect={() => {}}
        decisions={rows} turn={rows.length - 1} />,
    );
    expect(screen.getByText('Attack · 14 turns')).toBeTruthy();
  });
});
