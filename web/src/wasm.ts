// Loads the WebAssembly engine built by `npm run wasm` into ./pkg.
import init, { robot_names, run_battle } from './pkg/robocode_wasm.js';
import { parseRecording, type Recording } from './recording';

let ready: Promise<unknown> | null = null;

/** Loads the engine once; later calls reuse the same promise. */
export function loadEngine(): Promise<unknown> {
  ready ??= init();
  return ready;
}

export function robotNames(): string[] {
  return JSON.parse(robot_names()) as string[];
}

/** Runs a whole battle. Throws an Error with a readable message on bad input. */
export function runBattle(robots: string[], width: number, height: number, maxTurns: number, urban = false): Recording {
  return parseRecording(run_battle(robots.join(','), width, height, maxTurns, urban));
}
