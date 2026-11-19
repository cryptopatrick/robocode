// Loads the WebAssembly engine built by `npm run wasm` into ./pkg.
import init, { ratio_size, robot_names, run_battle } from './pkg/robocode_wasm.js';
import type { TankId } from './tanks/defs';
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
export function runBattle(robots: string[], width: number, height: number, maxTurns: number, urban = false, tanks: TankId[] = []): Recording {
  return parseRecording(run_battle(robots.join(','), width, height, maxTurns, urban, tanks.join(',')));
}

/** The battlefield ratios the engine offers, in picker order. */
export const RATIOS = ['4:3', '2:1', '16:9', '1:1'] as const;

/** `[width, height]` of a ratio, straight from the engine (Rust `Ratio::size`). */
export function ratioSize(name: string): [number, number] {
  const [w, h] = ratio_size(name);
  return [w, h];
}
