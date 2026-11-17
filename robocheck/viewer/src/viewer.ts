// The RoboCheck page's battle replay: robocode web/'s 2D drawing (drawArena) and 3D scene
// (ArenaScene3D), driven by web/'s playback reducer, behind a small imperative API the page's plain
// script can call. Nothing here simulates: it only replays a recording the verified robot fought.
import { Component, createElement, type ReactNode } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import ArenaScene3D from '../../../web/src/components/ArenaScene3D';
import { drawArena, type ViewOptions } from '../../../web/src/draw';
import { playbackReducer, type Playback, type PlaybackAction } from '../../../web/src/playback';
import { parseRecording, type Recording } from '../../../web/src/recording';

export type { Recording };
export type Mode = '2d' | '3d';

export interface LoadOptions {
  /** Robot names to replace in the drawn labels, e.g. { Candidate: 'RoboCheck ✕' }. */
  rename?: Record<string, string>;
}

export interface Viewer {
  load(rec: Recording, opts?: LoadOptions): void;
  recording(): Recording | null;
  setMode(m: Mode): void;
  mode(): Mode;
  seek(turn: number): void;
  step(): void;
  play(): void;
  pause(): void;
  reset(): void;
  turn(): number;
  turns(): number;
  playing(): boolean;
  onTurn(cb: (turn: number) => void): void;
  /** Shows a line under the arena for a violation turn; null clears it. */
  mark(turn: number | null, text?: string): void;
  destroy(): void;
}

/** Turns per second while playing. */
const SPEED = 30;
const VIEW: ViewOptions = { names: true, energy: true, scanArcs: true, effects: true };

const fail = (why: string) => new Error(`Couldn't load the battle recording (${why})`);

/** Fetches a recording; a URL ending in .gz is gunzipped in the browser. */
export async function loadRecording(url: string): Promise<Recording> {
  let res: Response;
  try {
    res = await fetch(url);
  } catch {
    throw fail('network error');
  }
  if (!res.ok) throw fail(`HTTP ${res.status}`);
  let text: string;
  if (url.endsWith('.gz')) {
    if (typeof DecompressionStream === 'undefined' || !res.body) throw fail('this browser cannot unpack it');
    try {
      text = await new Response(res.body.pipeThrough(new DecompressionStream('gzip'))).text();
    } catch {
      throw fail('the file is damaged');
    }
  } else {
    text = await res.text();
  }
  try {
    return parseRecording(text);
  } catch (e) {
    throw fail((e as Error).message);
  }
}

function hasWebGL(): boolean {
  try {
    const c = document.createElement('canvas');
    return !!(c.getContext('webgl2') || c.getContext('webgl'));
  } catch {
    return false;
  }
}

class Guard extends Component<{ onError: () => void; children: ReactNode }, { failed: boolean }> {
  state = { failed: false };
  static getDerivedStateFromError() {
    return { failed: true };
  }
  componentDidCatch() {
    this.props.onError();
  }
  render() {
    return this.state.failed ? null : this.props.children;
  }
}

export function mountViewer(el: HTMLElement): Viewer {
  el.classList.add('rcv');
  const canvas = document.createElement('canvas');
  canvas.className = 'rcv-2d';
  canvas.setAttribute('role', 'img');
  const box3d = document.createElement('div');
  box3d.className = 'rcv-3d';
  box3d.hidden = true;
  const note = document.createElement('p');
  note.className = 'rcv-note';
  note.hidden = true;
  const marker = document.createElement('p');
  marker.className = 'rcv-mark';
  marker.hidden = true;
  el.append(canvas, box3d, note, marker);

  let rec: Recording | null = null;
  let state: Playback = { turn: 0, last: 0, playing: false, speed: SPEED };
  let mode: Mode = '2d';
  let timer: ReturnType<typeof setInterval> | null = null;
  let root: Root | null = null;
  const listeners: ((turn: number) => void)[] = [];

  const say = (text: string | null) => {
    note.hidden = !text;
    note.textContent = text ?? '';
  };

  const draw = () => {
    if (!rec) return;
    if (mode === '2d') {
      const ctx = canvas.getContext('2d');
      if (ctx) drawArena(ctx, rec, state.turn, VIEW);
      canvas.setAttribute('aria-label', `Battle arena, turn ${state.turn}`);
    } else {
      root ??= createRoot(box3d);
      root.render(
        createElement(Guard, { onError: () => fallback() }, createElement(ArenaScene3D, { recording: rec, turn: state.turn })),
      );
    }
  };

  const fallback = () => {
    setMode('2d');
    say('3D needs WebGL, which this browser has turned off; showing 2D.');
  };

  const dispatch = (a: PlaybackAction) => {
    const before = state;
    state = playbackReducer(state, a);
    if (state.playing && !timer) timer = setInterval(() => dispatch({ type: 'tick' }), 1000 / SPEED);
    if (!state.playing && timer) {
      clearInterval(timer);
      timer = null;
    }
    draw();
    const moved = a.type === 'step' || a.type === 'seek' || a.type === 'tick';
    if (moved || before.turn !== state.turn) {
      listeners.forEach((cb) => cb(state.turn));
    }
  };

  function setMode(m: Mode) {
    if (m === '3d' && !hasWebGL()) {
      say('3D needs WebGL, which this browser has turned off; showing 2D.');
      m = '2d';
    } else {
      say(null);
    }
    mode = m;
    canvas.hidden = m !== '2d';
    box3d.hidden = m !== '3d';
    if (m === '2d' && root) {
      root.unmount();
      root = null;
    }
    draw();
  }

  return {
    load(r, opts = {}) {
      const rename = opts.rename ?? {};
      rec = { ...r, robots: r.robots.map((b) => ({ ...b, name: rename[b.name] ?? b.name })) };
      canvas.width = rec.battlefield.width;
      canvas.height = rec.battlefield.height;
      marker.hidden = true;
      dispatch({ type: 'load', last: rec.turns.length - 1 });
    },
    recording: () => rec,
    setMode,
    mode: () => mode,
    seek: (turn) => dispatch({ type: 'seek', turn }),
    step: () => dispatch({ type: 'step', by: 1 }),
    play: () => dispatch({ type: 'play' }),
    pause: () => dispatch({ type: 'pause' }),
    reset: () => dispatch({ type: 'seek', turn: 0 }),
    turn: () => state.turn,
    turns: () => (rec ? rec.turns.length : 0),
    playing: () => state.playing,
    onTurn: (cb) => void listeners.push(cb),
    mark(turn, text) {
      marker.hidden = turn === null;
      marker.textContent = turn === null ? '' : (text ?? `Turn ${turn}`);
    },
    destroy() {
      if (timer) clearInterval(timer);
      root?.unmount();
      el.replaceChildren();
    },
  };
}
