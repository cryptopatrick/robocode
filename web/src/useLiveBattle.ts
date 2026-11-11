// Runs the engine worker and hands its frames to the page once per animation
// frame: the simulation runs on the worker's clock, rendering on the
// display's, and neither waits for the other.
import { useCallback, useEffect, useRef } from 'react';
import { startRecording, type Frame, type FromWorker, type ToWorker } from './live';
import { parseRecording, type Recording } from './recording';

export interface LiveHandlers {
  started: (rec: Recording) => void;
  grew: (frames: Frame[], measuredTps: number) => void;
  done: (rec: Recording) => void;
  error: (message: string) => void;
}

/** Returns a function that sends commands to the engine worker. */
export function useLiveBattle(handlers: LiveHandlers): (m: ToWorker) => void {
  const worker = useRef<Worker | null>(null);
  const latest = useRef(handlers);
  latest.current = handlers;

  useEffect(() => {
    const w = new Worker(new URL('./engine.worker.ts', import.meta.url), { type: 'module' });
    let buffered: Frame[] = [];
    let measured = 0;
    let finished: Recording | null = null;
    w.onmessage = (e: MessageEvent<FromWorker>) => {
      const m = e.data;
      if (m.type === 'started') {
        buffered = [];
        finished = null;
        latest.current.started(startRecording(m.header, m.initial));
      } else if (m.type === 'frames') {
        buffered.push(...m.frames);
        measured = m.measuredTps;
      } else if (m.type === 'done') {
        finished = parseRecording(m.recording);
      } else {
        latest.current.error(m.message);
      }
    };
    worker.current = w;
    let raf = 0;
    const flush = () => {
      if (buffered.length > 0) {
        const frames = buffered;
        buffered = [];
        latest.current.grew(frames, measured);
      }
      if (finished) {
        latest.current.done(finished);
        finished = null;
      }
      raf = requestAnimationFrame(flush);
    };
    raf = requestAnimationFrame(flush);
    return () => {
      cancelAnimationFrame(raf);
      w.terminate();
      worker.current = null;
    };
  }, []);

  // Stable identity, so effects that depend on it don't re-run every render.
  return useCallback((m: ToWorker) => worker.current?.postMessage(m), []);
}
