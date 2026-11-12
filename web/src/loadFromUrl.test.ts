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

import { liveUrl, controlLine } from './loadFromUrl';

describe('liveUrl', () => {
  it('reads the live parameter', () => {
    expect(liveUrl('?live=/live')).toBe('/live');
    expect(liveUrl('?recording=/battle.json')).toBeNull();
    expect(liveUrl('?live=')).toBeNull();
  });
});

describe('controlLine', () => {
  it('turns viewer commands into the server control protocol', () => {
    expect(controlLine({ type: 'pause' })).toBe('pause');
    expect(controlLine({ type: 'resume' })).toBe('resume');
    expect(controlLine({ type: 'step' })).toBe('step');
    expect(controlLine({ type: 'tps', tps: 60 })).toBe('tps 60');
    expect(controlLine({ type: 'tps', tps: null })).toBe('tps max');
    expect(controlLine({ type: 'stop' })).toBeNull();
  });
});
