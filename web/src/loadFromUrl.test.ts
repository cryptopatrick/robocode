import { describe, expect, it } from 'vitest';
import { loadRecording, recordingUrl } from './loadFromUrl';

const minimal = {
  format: 'roboverus-recording', version: 1,
  robots: [{ id: 0, name: 'MyRobot' }, { id: 1, name: 'SittingDuck' }],
  battlefield: { width: 800, height: 600 },
  turns: [{ turn: 0, robots: [], bullets: [], events: { bullet_fired: [], bullet_hits: [], wall_collisions: [], robot_collisions: [], robot_deaths: [] } }],
  decisions: [[null, null]],
  outcome: { winner: 0, turns: 0, energies: [100, 0] },
};

const stubFetch = (status: number, body: string) =>
  (async () => new Response(body, { status })) as unknown as typeof fetch;

describe('recordingUrl', () => {
  it('reads the recording parameter', () => {
    expect(recordingUrl('?recording=/battle.json')).toBe('/battle.json');
    expect(recordingUrl('?x=1&recording=%2Fa%20b.json')).toBe('/a b.json');
  });
  it('is null without the parameter', () => {
    expect(recordingUrl('')).toBeNull();
    expect(recordingUrl('?recording=')).toBeNull();
  });
});

describe('loadRecording', () => {
  it('fetches and parses', async () => {
    const rec = await loadRecording('/battle.json', stubFetch(200, JSON.stringify(minimal)));
    expect(rec.robots[0].name).toBe('MyRobot');
  });
  it('reports HTTP errors', async () => {
    await expect(loadRecording('/battle.json', stubFetch(404, 'nope'))).rejects.toThrow('Could not load /battle.json: HTTP 404');
  });
  it('reports bad content', async () => {
    await expect(loadRecording('/battle.json', stubFetch(200, '{}'))).rejects.toThrow('Not a RoboVerus recording');
  });
});
