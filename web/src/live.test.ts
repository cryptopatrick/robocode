import { describe, expect, it } from 'vitest';
import { appendFrames, startRecording, type Frame, type LiveHeader } from './live';

const header: LiveHeader = {
  format: 'roboverus-recording', version: 1,
  robots: [{ id: 0, name: 'A' }, { id: 1, name: 'B' }],
  battlefield: { width: 800, height: 600 },
};

const frame = (turn: number, energies: [number, number]): Frame => ({
  turn: {
    turn,
    robots: energies.map((e, id) => ({
      id, position: { x: 100, y: 100 }, heading: 0, gun_heading: 0, radar_heading: 0,
      velocity: 0, energy: e, gun_heat: 0, alive: e > 0, radius: 18,
    })),
    bullets: [],
    events: { bullet_fired: [], bullet_hits: [], wall_collisions: [], robot_collisions: [], robot_deaths: [] },
  },
  decisions: [null, null],
});

describe('live recording', () => {
  it('starts as a v1 recording holding turn 0', () => {
    const rec = startRecording(header, frame(0, [100, 100]));
    expect(rec.format).toBe('roboverus-recording');
    expect(rec.turns).toHaveLength(1);
    expect(rec.decisions).toHaveLength(1);
    expect(rec.outcome).toEqual({ winner: null, turns: 0, energies: [100, 100] });
  });

  it('grows with frames and keeps a provisional outcome', () => {
    const rec = startRecording(header, frame(0, [100, 100]));
    const more = appendFrames(rec, [frame(1, [99, 97]), frame(2, [98, 90])]);
    expect(more.turns.map((t) => t.turn)).toEqual([0, 1, 2]);
    expect(more.decisions).toHaveLength(3);
    expect(more.outcome).toEqual({ winner: null, turns: 2, energies: [98, 90] });
    expect(rec.turns).toHaveLength(1); // the original is not mutated (React state)
  });

  it('appending nothing returns the same object', () => {
    const rec = startRecording(header, frame(0, [100, 100]));
    expect(appendFrames(rec, [])).toBe(rec);
  });
});
