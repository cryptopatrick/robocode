// Runs the engine worker and hands its frames to the page once per animation
// frame: the simulation runs on the worker's clock, rendering on the
// display's, and neither waits for the other.
import { useCallback, useEffect, useRef } from 'react';
import { controlLine } from './loadFromUrl';
import { startRecording, type Frame, type FromWorker, type LiveHeader, type ToWorker } from './live';
import { parseRecording, type Recording } from './recording';

export interface LiveHandlers {
  started: (rec: Recording) => void;
  grew: (frames: Frame[], measuredTps: number) => void;
  done: (rec: Recording) => void;
  error: (message: string) => void;
}

/**
 * Returns a function that sends commands to the live simulation. By default the
 * simulation runs in a Web Worker in this page. With `streamUrl` (from
 * `?live=`), it runs in `cargo robocode`'s battle program instead: frames
 * arrive as Server-Sent Events and commands go to `POST /control`.
 */
export function useLiveBattle(handlers: LiveHandlers, streamUrl: string | null = null): (m: ToWorker) => void {
  const worker = useRef<Worker | null>(null);
  const latest = useRef(handlers);
  latest.current = handlers;

  useEffect(() => {
    let buffered: Frame[] = [];
    let measured = 0;
    let finished: Recording | null = null;
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

    if (streamUrl) {
      const source = new EventSource(streamUrl);
      let header: LiveHeader | null = null;
      let started = false;
      let done = false;
      const arrivals: number[] = [];
      source.onmessage = (e: MessageEvent<string>) => {
        const data = JSON.parse(e.data) as LiveHeader | Frame | { done: true };
        if (!header) {
          header = data as LiveHeader;
        } else if (!started) {
          started = true;
          latest.current.started(startRecording(header, data as Frame));
        } else if ('done' in data) {
          done = true;
          source.close();
          fetch(new URL('/battle.json', window.location.href))
            .then((r) => r.text())
            .then((text) => {
              finished = parseRecording(text);
            })
            .catch((err: unknown) => latest.current.error(`Could not load the finished battle: ${String(err)}`));
        } else {
          const now = performance.now();
          arrivals.push(now);
          while (arrivals.length > 0 && arrivals[0] < now - 1000) arrivals.shift();
          measured = arrivals.length;
          buffered.push(data as Frame);
        }
      };
      source.onerror = () => {
        if (!done) latest.current.error('Lost the live battle stream (is cargo robocode still running?)');
        source.close();
      };
      raf = requestAnimationFrame(flush);
      return () => {
        cancelAnimationFrame(raf);
        source.close();
      };
    }

    const w = new Worker(new URL('./engine.worker.ts', import.meta.url), { type: 'module' });
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
    raf = requestAnimationFrame(flush);
    return () => {
      cancelAnimationFrame(raf);
      w.terminate();
      worker.current = null;
    };
  }, [streamUrl]);

  // Stable identity, so effects that depend on it don't re-run every render.
  return useCallback(
    (m: ToWorker) => {
      if (!streamUrl) {
        worker.current?.postMessage(m);
        return;
      }
      const line = controlLine(m);
      if (line) void fetch(new URL('/control', window.location.href), { method: 'POST', body: line });
    },
    [streamUrl],
  );
}
