/// <reference lib="webworker" />
// The live simulation loop: steps the wasm engine on its own clock (paced by
// the Rust Pacer) and posts each turn's frame. Rendering happens on the page;
// this loop never waits for it.
import init, { LiveBattle, LivePacer } from './pkg/robocode_wasm.js';
import type { Frame, FromWorker, LiveHeader, ToWorker } from './live';

let battle: LiveBattle | null = null;
let pacer: LivePacer | null = null;
let timer: ReturnType<typeof setTimeout> | null = null;
let ready: Promise<unknown> | null = null;

const post = (m: FromWorker) => postMessage(m);

function stop() {
  if (timer !== null) clearTimeout(timer);
  timer = null;
  battle?.free();
  pacer?.free();
  battle = null;
  pacer = null;
}

function loop() {
  timer = null;
  if (!battle || !pacer) return;
  const due = pacer.due(performance.now());
  if (due > 0) {
    const frames = JSON.parse(battle.step_many(due)) as Frame[];
    post({ type: 'frames', frames, measuredTps: pacer.measured_tps() });
  }
  if (battle.finished()) {
    post({ type: 'done', recording: battle.recording() });
    stop();
    return;
  }
  timer = setTimeout(loop, 4);
}

self.onmessage = async (e: MessageEvent<ToWorker>) => {
  const m = e.data;
  try {
    switch (m.type) {
      case 'start': {
        stop();
        ready ??= init();
        await ready;
        battle = new LiveBattle(m.robots.join(','), m.width, m.height, m.maxTurns, m.urban);
        pacer = new LivePacer(m.tps ?? 0);
        post({
          type: 'started',
          header: JSON.parse(battle.header()) as LiveHeader,
          initial: JSON.parse(battle.initial_frame()) as Frame,
        });
        loop();
        break;
      }
      case 'pause':
        pacer?.pause();
        break;
      case 'resume':
        pacer?.resume(performance.now());
        break;
      case 'step':
        pacer?.step();
        break;
      case 'tps':
        pacer?.set_tps(m.tps ?? 0, performance.now());
        break;
      case 'stop':
        stop();
        break;
    }
  } catch (err) {
    post({ type: 'error', message: err instanceof Error ? err.message : String(err) });
  }
};
