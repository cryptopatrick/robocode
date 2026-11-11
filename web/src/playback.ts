// Playback state for replaying a recording, like the classic battle window's
// Pause / Next Turn / Replay controls and speed slider.

/** Selectable speeds in turns per second. */
export const SPEEDS = [1, 2, 5, 10, 20, 30, 60, 120] as const;

export interface Playback {
  turn: number;
  last: number;
  playing: boolean;
  speed: number;
}

export type PlaybackAction =
  | { type: 'play' }
  | { type: 'pause' }
  | { type: 'toggle' }
  | { type: 'step'; by: 1 | -1 }
  | { type: 'seek'; turn: number }
  | { type: 'restart' }
  | { type: 'speed'; speed: number }
  | { type: 'tick' }
  | { type: 'load'; last: number }
  /** A live recording grew to `last`; `follow` keeps the view on the newest turn. */
  | { type: 'extend'; last: number; follow: boolean };

const clamp = (turn: number, last: number) => Math.min(Math.max(Math.round(turn), 0), last);

export function playbackReducer(s: Playback, a: PlaybackAction): Playback {
  switch (a.type) {
    case 'play':
      return s.turn >= s.last ? { ...s, turn: 0, playing: true } : { ...s, playing: true };
    case 'pause':
      return { ...s, playing: false };
    case 'toggle':
      return s.playing ? { ...s, playing: false } : playbackReducer(s, { type: 'play' });
    case 'step':
      return { ...s, turn: clamp(s.turn + a.by, s.last), playing: false };
    case 'seek':
      return { ...s, turn: clamp(a.turn, s.last), playing: false };
    case 'restart':
      return { ...s, turn: 0, playing: true };
    case 'speed':
      return { ...s, speed: a.speed };
    case 'tick': {
      if (!s.playing) return s;
      const turn = Math.min(s.turn + 1, s.last);
      return { ...s, turn, playing: turn < s.last };
    }
    case 'load':
      return { ...s, turn: 0, last: Math.max(a.last, 0), playing: false };
    case 'extend': {
      const last = Math.max(a.last, 0);
      return { ...s, last, turn: a.follow ? last : clamp(s.turn, last) };
    }
  }
}
