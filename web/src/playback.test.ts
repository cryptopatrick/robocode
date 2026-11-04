import { describe, expect, it } from 'vitest';
import { playbackReducer, type Playback } from './playback';

const at = (turn: number, playing = false): Playback => ({ turn, last: 10, playing, speed: 30 });

describe('playbackReducer', () => {
  it('step clamps at both ends and pauses', () => {
    expect(playbackReducer(at(10, true), { type: 'step', by: 1 })).toEqual(at(10));
    expect(playbackReducer(at(0), { type: 'step', by: -1 })).toEqual(at(0));
    expect(playbackReducer(at(4, true), { type: 'step', by: 1 })).toEqual(at(5));
  });

  it('tick advances only while playing and stops at the last turn', () => {
    expect(playbackReducer(at(3, true), { type: 'tick' })).toEqual(at(4, true));
    expect(playbackReducer(at(3), { type: 'tick' })).toEqual(at(3));
    expect(playbackReducer(at(9, true), { type: 'tick' })).toEqual(at(10, false));
  });

  it('play at the end restarts from turn 0', () => {
    expect(playbackReducer(at(10), { type: 'play' })).toEqual(at(0, true));
    expect(playbackReducer(at(10), { type: 'toggle' })).toEqual(at(0, true));
    expect(playbackReducer(at(5, true), { type: 'toggle' })).toEqual(at(5));
  });

  it('seek clamps and pauses', () => {
    expect(playbackReducer(at(5, true), { type: 'seek', turn: 99 })).toEqual(at(10));
    expect(playbackReducer(at(5), { type: 'seek', turn: -3 })).toEqual(at(0));
  });

  it('load resets to turn 0, paused, with the new length', () => {
    expect(playbackReducer(at(7, true), { type: 'load', last: 42 })).toEqual({ turn: 0, last: 42, playing: false, speed: 30 });
  });

  it('restart and speed', () => {
    expect(playbackReducer(at(7, true), { type: 'restart' })).toEqual(at(0, true));
    expect(playbackReducer(at(7), { type: 'speed', speed: 120 }).speed).toBe(120);
  });
});
