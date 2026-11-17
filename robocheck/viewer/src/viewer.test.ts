import { afterEach, describe, expect, it, vi } from 'vitest';
import { loadRecording, mountViewer } from './viewer';
import { robotLabel } from '../../../web/src/draw';

const gzip = async (text: string) => {
  const stream = new ReadableStream({ start(c) { c.enqueue(new TextEncoder().encode(text)); c.close(); } });
  const chunks: Uint8Array[] = [];
  const reader = stream.pipeThrough(new CompressionStream('gzip')).getReader();
  for (;;) { const { done, value } = await reader.read(); if (done) break; chunks.push(value); }
  return new Uint8Array(chunks.flatMap((c) => [...c]));
};

function recording(turns = 3) {
  const robot = (id: number) => ({
    id, position: { x: 100 + id * 200, y: 100 }, heading: 0, gun_heading: 0, radar_heading: 0,
    velocity: 0, energy: 100, gun_heat: 0, alive: true, radius: 18,
  });
  const empty = { bullet_fired: [], bullet_hits: [], wall_collisions: [], robot_collisions: [], robot_deaths: [] };
  return {
    format: 'roboverus-recording' as const,
    version: 1 as const,
    robots: [{ id: 0, name: 'Candidate' }, { id: 1, name: 'Circler' }],
    battlefield: { width: 800, height: 600 },
    turns: Array.from({ length: turns }, (_, turn) => ({ turn, robots: [robot(0), robot(1)], bullets: [], events: empty })),
    decisions: Array.from({ length: turns }, () => [null, null]),
    outcome: { winner: 0, turns, energies: [100, 0] },
  };
}

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  document.body.innerHTML = '';
});

function viewer() {
  const el = document.createElement('div');
  document.body.append(el);
  const v = mountViewer(el);
  v.load(recording());
  return { el, v };
}

describe('mountViewer', () => {
  it('seek clamps to the recording', () => {
    const { v } = viewer();
    expect(v.turns()).toBe(3);
    v.seek(99);
    expect(v.turn()).toBe(2);
    v.seek(-5);
    expect(v.turn()).toBe(0);
  });

  it('step advances one turn and stops at the end', () => {
    const { v } = viewer();
    const seen: number[] = [];
    v.onTurn((t) => seen.push(t));
    v.step();
    v.step();
    v.step();
    expect(v.turn()).toBe(2);
    expect(seen).toEqual([1, 2, 2]);
  });

  it('play advances on a timer and pause stops it', () => {
    vi.useFakeTimers();
    const el = document.createElement('div');
    const v = mountViewer(el);
    v.load(recording(100));
    v.play();
    expect(v.playing()).toBe(true);
    vi.advanceTimersByTime(1000);
    const at = v.turn();
    expect(at).toBeGreaterThan(5);
    v.pause();
    vi.advanceTimersByTime(1000);
    expect(v.turn()).toBe(at);
    expect(v.playing()).toBe(false);
  });

  it('reset goes back to turn 0 and stops', () => {
    const { v } = viewer();
    v.seek(2);
    v.reset();
    expect(v.turn()).toBe(0);
    expect(v.playing()).toBe(false);
  });

  it('3d without WebGL keeps 2d and says why', () => {
    const { el, v } = viewer();
    expect(() => v.setMode('3d')).not.toThrow();
    expect(el.textContent).toContain('3D needs WebGL');
    expect(el.querySelector('canvas')).not.toBeNull();
    expect(v.mode()).toBe('2d');
  });

  it('mark shows the violation turn and clears', () => {
    const { el, v } = viewer();
    v.mark(1, 'Turn 1: the robot breaks S2');
    expect(el.textContent).toContain('Turn 1: the robot breaks S2');
    v.mark(null);
    expect(el.textContent).not.toContain('breaks S2');
  });

  it('rename changes the drawn name', () => {
    const el = document.createElement('div');
    const v = mountViewer(el);
    v.load(recording(), { rename: { Candidate: 'RoboCheck ✕' } });
    const rec = v.recording()!;
    expect(rec.robots[0].name).toBe('RoboCheck ✕');
    expect(robotLabel(rec.turns[0].robots[0], rec.robots[0].name)).toContain('RoboCheck ✕');
  });
});

describe('loadRecording', () => {
  it('rejects with a readable message on 404', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('nope', { status: 404 })));
    await expect(loadRecording('/x/recording-open.json.gz')).rejects.toThrow(/Couldn't load the battle recording \(HTTP 404\)/);
  });

  it('rejects when the browser cannot decompress', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response(await gzip(JSON.stringify(recording())))));
    vi.stubGlobal('DecompressionStream', undefined);
    await expect(loadRecording('/x/r.json.gz')).rejects.toThrow(/Couldn't load the battle recording/);
  });

  it('reads a gzipped recording', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response(await gzip(JSON.stringify(recording(4))))));
    const rec = await loadRecording('/x/r.json.gz');
    expect(rec.turns.length).toBe(4);
  });

  it('reads a plain recording', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify(recording(2)))));
    expect((await loadRecording('/x/r.json')).turns.length).toBe(2);
  });
});
