// Live battles: a Web Worker steps the engine on a clock and posts frames; the
// page appends them to a growing v1 recording and renders independently.
import type { Decision, Recording, TurnSnapshot } from './recording';

/** One turn, as the engine streams it (`recording::Frame::to_json`). */
export interface Frame {
  turn: TurnSnapshot;
  decisions: (Decision | null)[];
}

/** The stream's first line (`recording::header_json`). */
export type LiveHeader = Pick<Recording, 'format' | 'version' | 'robots' | 'battlefield'>;

function provisionalOutcome(turns: TurnSnapshot[]): Recording['outcome'] {
  const last = turns[turns.length - 1];
  return { winner: null, turns: turns.length - 1, energies: last.robots.map((r) => r.energy) };
}

/** A recording holding only turn 0. */
export function startRecording(header: LiveHeader, initial: Frame): Recording {
  const turns = [initial.turn];
  return { ...header, turns, decisions: [initial.decisions], outcome: provisionalOutcome(turns) };
}

/** A new recording with `frames` appended (the input is not modified). */
export function appendFrames(rec: Recording, frames: Frame[]): Recording {
  if (frames.length === 0) return rec;
  const turns = rec.turns.concat(frames.map((f) => f.turn));
  const decisions = rec.decisions.concat(frames.map((f) => f.decisions));
  return { ...rec, turns, decisions, outcome: provisionalOutcome(turns) };
}

/** Messages to the engine worker. `tps: null` means as fast as possible. */
export type ToWorker =
  | { type: 'start'; robots: string[]; width: number; height: number; maxTurns: number; urban: boolean; tps: number | null }
  | { type: 'pause' }
  | { type: 'resume' }
  | { type: 'step' }
  | { type: 'tps'; tps: number | null }
  | { type: 'stop' };

/** Messages from the engine worker. */
export type FromWorker =
  | { type: 'started'; header: LiveHeader; initial: Frame }
  | { type: 'frames'; frames: Frame[]; measuredTps: number }
  | { type: 'done'; recording: string }
  | { type: 'error'; message: string };
