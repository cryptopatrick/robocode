import { describe, expect, it } from 'vitest';
import { parseRecording } from './recording';

const minimal = {
  format: 'roboverus-recording',
  version: 1,
  robots: [{ id: 0, name: 'SafeShooter' }, { id: 1, name: 'SittingDuck' }],
  battlefield: { width: 800, height: 600 },
  turns: [
    {
      battle_id: 'viewer',
      turn: 0,
      config: { battlefield_width: 800, battlefield_height: 600, gun_cooling_rate: 0.1 },
      robots: [],
      bullets: [],
      events: { bullet_fired: [], bullet_hits: [], wall_collisions: [], robot_collisions: [], robot_deaths: [] },
    },
  ],
  decisions: [[null, null]],
  outcome: { winner: null, turns: 0, energies: [100, 100] },
};

describe('parseRecording', () => {
  it('parses a valid recording', () => {
    const rec = parseRecording(JSON.stringify(minimal));
    expect(rec.robots[1].name).toBe('SittingDuck');
    expect(rec.turns).toHaveLength(1);
    expect(rec.outcome.winner).toBeNull();
  });

  it('rejects non-JSON', () => {
    expect(() => parseRecording('not json {')).toThrow('Not a JSON file');
  });

  it('rejects other JSON', () => {
    expect(() => parseRecording('{}')).toThrow('Not a RoboVerus recording');
    expect(() => parseRecording('[1,2]')).toThrow('Not a RoboVerus recording');
  });

  it('rejects a future version', () => {
    expect(() => parseRecording(JSON.stringify({ ...minimal, version: 2 }))).toThrow('Unsupported recording version 2');
  });

  it('rejects mismatched turns and decisions', () => {
    expect(() => parseRecording(JSON.stringify({ ...minimal, decisions: [] }))).toThrow('Not a RoboVerus recording');
  });
});
