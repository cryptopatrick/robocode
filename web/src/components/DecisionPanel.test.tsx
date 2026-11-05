import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import DecisionPanel from './DecisionPanel';
import type { Decision } from '../recording';

const firing: Decision = {
  energy: 50_000, gun_heat: 0, gun_heading: 0,
  target: { bearing: 0, distance: 300, age: 0 },
  fire: 1000, ahead: 100, turn_body: -150, turn_gun: 0, turn_radar: 450,
};

describe('DecisionPanel', () => {
  it('says when a dead robot has no decision', () => {
    render(<DecisionPanel decision={null} dead />);
    expect(screen.getByText('no decision (dead)')).toBeTruthy();
  });

  it('says when there is no decision yet', () => {
    render(<DecisionPanel decision={null} dead={false} />);
    expect(screen.getByText('no decision yet')).toBeTruthy();
  });

  it('shows the action in human units and the safety checks', () => {
    render(<DecisionPanel decision={firing} dead={false} />);
    expect(screen.getByText('Fire 1.0')).toBeTruthy();
    expect(screen.getByText('Turn body -15.0°')).toBeTruthy();
    expect(screen.getByText('gun is cool')).toBeTruthy();
  });

  it('asks whether a shot would be safe when not firing', () => {
    render(<DecisionPanel decision={{ ...firing, fire: null }} dead={false} />);
    expect(screen.getByText('Would a 1.0 shot be safe?')).toBeTruthy();
  });
});

describe('DecisionPanel for Robot API robots', () => {
  it('explains that only strategy robots record decisions', () => {
    render(<DecisionPanel decision={null} dead={false} recordsDecisions={false} />);
    expect(screen.getByText(/written with the Robot API/)).toBeTruthy();
  });
});
