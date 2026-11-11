import { useCallback, useEffect, useReducer, useRef, useState } from 'react';
import ArenaCanvas from './components/ArenaCanvas';
import Controls from './components/Controls';
import LiveControls, { type LiveUi } from './components/LiveControls';
import DecisionPanel from './components/DecisionPanel';
import NewBattle, { type BattleSetup } from './components/NewBattle';
import RecordingFile from './components/RecordingFile';
import Results from './components/Results';
import RobotPanel from './components/RobotPanel';
import ViewToggles from './components/ViewToggles';
import type { ViewOptions } from './draw';
import { appendFrames } from './live';
import { playbackReducer } from './playback';
import type { Recording } from './recording';
import { loadRecording, recordingUrl } from './loadFromUrl';
import { loadSprites, type Sprites } from './sprites';
import { useLiveBattle } from './useLiveBattle';
import { loadEngine, robotNames } from './wasm';

const DEFAULT_VIEW: ViewOptions = { names: true, energy: true, scanArcs: true, effects: true };

const quiet =
  'rounded-md border border-slate-300 bg-white px-3 py-1 text-sm text-slate-700 hover:bg-slate-50 focus-within:ring-2 focus-within:ring-sky-600';

export default function App() {
  const [names, setNames] = useState<string[]>([]);
  const [recording, setRecording] = useState<Recording | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [view, setView] = useState<ViewOptions>(DEFAULT_VIEW);
  const [selected, setSelected] = useState(0);
  const [sprites, setSprites] = useState<Sprites>({});
  const [playback, dispatch] = useReducer(playbackReducer, { turn: 0, last: 0, playing: false, speed: 30 });
  /** Set while a battle runs live; `null` when replaying a recording. */
  const [live, setLive] = useState<LiveUi | null>(null);
  const following = useRef(true);
  const lastTurn = useRef(0);
  const liveTps = useRef<number | null>(30);

  const send = useLiveBattle({
    started: (rec) => {
      setRecording(rec);
      setSelected(0);
      setError(null);
      lastTurn.current = 0;
      following.current = true;
      dispatch({ type: 'load', last: 0 });
      setLive({ paused: false, tps: liveTps.current, measuredTps: 0, finished: false, following: true });
    },
    grew: (frames, measuredTps) => {
      setRecording((r) => (r ? appendFrames(r, frames) : r));
      lastTurn.current += frames.length;
      dispatch({ type: 'extend', last: lastTurn.current, follow: following.current });
      setLive((l) => (l ? { ...l, measuredTps } : l));
    },
    done: (rec) => {
      setRecording(rec);
      lastTurn.current = rec.turns.length - 1;
      dispatch({ type: 'extend', last: lastTurn.current, follow: following.current });
      setLive((l) => (l ? { ...l, finished: true } : l));
    },
    error: (message) => setError(message),
  });

  /** Replay a finished recording (opened file or `?recording=`). */
  const show = useCallback(
    (rec: Recording) => {
      send({ type: 'stop' });
      setLive(null);
      setRecording(rec);
      setSelected(0);
      setError(null);
      dispatch({ type: 'load', last: rec.turns.length - 1 });
    },
    [send],
  );

  /** Start a live battle in the engine worker. */
  const run = useCallback(
    (setup: BattleSetup) => {
      setError(null);
      send({ type: 'start', robots: setup.robots, width: setup.width, height: setup.height, maxTurns: setup.maxTurns, tps: liveTps.current });
    },
    [send],
  );

  const setFollowing = (f: boolean) => {
    following.current = f;
    setLive((l) => (l ? { ...l, following: f } : l));
  };

  useEffect(() => {
    const fromUrl = recordingUrl(window.location.search);
    if (fromUrl) {
      loadRecording(fromUrl)
        .then((rec) => {
          show(rec);
          dispatch({ type: 'play' });
        })
        .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
    }
    loadEngine()
      .then(() => {
        setNames(robotNames());
        if (!fromUrl) run({ robots: ['SafeShooter', 'Spinner'], width: 800, height: 600, maxTurns: 5000 });
      })
      .catch((e: unknown) => setError(`Could not load the battle engine: ${e instanceof Error ? e.message : String(e)}`));
  }, [run, show]);

  // Advance playback at `speed` turns per second.
  const carry = useRef(0);
  useEffect(() => {
    if (!playback.playing) return;
    let frame = 0;
    let before = performance.now();
    const loop = (now: number) => {
      carry.current += ((now - before) * playback.speed) / 1000;
      before = now;
      while (carry.current >= 1) {
        carry.current -= 1;
        dispatch({ type: 'tick' });
      }
      frame = requestAnimationFrame(loop);
    };
    frame = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(frame);
  }, [playback.playing, playback.speed]);

  return (
    <div className="min-h-screen bg-[#eef2f5] text-[#1b2733]">
      <header className="border-b border-slate-300 bg-white">
        <div className="mx-auto flex max-w-7xl flex-wrap items-end justify-between gap-4 px-4 py-3">
          <div>
            <h1 className="text-lg font-semibold tracking-tight">RoboVerus Arena</h1>
            <p className="text-sm text-slate-500">Pick robots, run a battle, and watch every decision.</p>
          </div>
          {names.length > 0 && <NewBattle names={names} onRun={run} />}
        </div>
      </header>

      <main className="mx-auto grid max-w-7xl gap-6 px-4 py-6 lg:grid-cols-[minmax(0,1fr)_320px]">
        <section className="flex flex-col gap-3">
          {error && (
            <p role="alert" className="rounded-md border border-[#c9433b]/40 bg-[#c9433b]/10 px-3 py-2 text-sm text-[#8f2a24]">
              {error}
            </p>
          )}
          {recording ? (
            <>
              <ArenaCanvas recording={recording} turn={playback.turn} view={view} sprites={sprites} />
              {live ? (
                <LiveControls
                  live={live}
                  turn={playback.turn}
                  last={playback.last}
                  onPause={() => {
                    send({ type: 'pause' });
                    setLive((l) => (l ? { ...l, paused: true } : l));
                  }}
                  onResume={() => {
                    send({ type: 'resume' });
                    setLive((l) => (l ? { ...l, paused: false } : l));
                  }}
                  onStep={() => send({ type: 'step' })}
                  onTps={(tps) => {
                    liveTps.current = tps;
                    send({ type: 'tps', tps });
                    setLive((l) => (l ? { ...l, tps } : l));
                  }}
                  onSeek={(turn) => {
                    setFollowing(turn >= lastTurn.current);
                    dispatch({ type: 'seek', turn });
                  }}
                  onBackToLive={() => {
                    setFollowing(true);
                    dispatch({ type: 'extend', last: lastTurn.current, follow: true });
                  }}
                />
              ) : (
                <Controls playback={playback} dispatch={dispatch} />
              )}
            </>
          ) : (
            !error && <p className="text-sm text-slate-500">Loading the battle engine…</p>
          )}
        </section>

        <aside className="flex flex-col gap-5">
          {recording && (
            <>
              <section aria-labelledby="robots-h" className="flex flex-col gap-2">
                <h2 id="robots-h" className="text-sm font-semibold">Robots</h2>
                <RobotPanel
                  names={recording.robots.map((r) => r.name)}
                  robots={recording.turns[playback.turn].robots}
                  selected={selected}
                  onSelect={setSelected}
                />
              </section>
              <section aria-labelledby="decision-h" className="flex flex-col gap-2">
                <h2 id="decision-h" className="text-sm font-semibold">
                  {recording.robots[selected]?.name}'s decision this turn
                </h2>
                <DecisionPanel
                  decision={recording.decisions[playback.turn][selected] ?? null}
                  dead={playback.turn > 0 && !recording.turns[playback.turn - 1].robots[selected]?.alive}
                  recordsDecisions={recording.decisions.some((row) => row[selected] !== null)}
                />
              </section>
              {playback.turn === playback.last && (!live || live.finished) && <Results recording={recording} />}
              <section aria-labelledby="view-h" className="flex flex-col gap-2">
                <h2 id="view-h" className="text-sm font-semibold">Show</h2>
                <ViewToggles view={view} onChange={setView} />
              </section>
            </>
          )}
          <RecordingFile recording={recording} onOpen={show} onError={setError} />
          <div className="flex flex-wrap gap-2">
            <label className={`${quiet} cursor-pointer`}>
              Robot images…
              <input
                type="file"
                accept="image/*"
                multiple
                className="sr-only"
                onChange={(e) => {
                  const files = Array.from(e.target.files ?? []);
                  e.target.value = '';
                  loadSprites(files)
                    .then((s) => {
                      setSprites(s);
                      setError(null);
                    })
                    .catch((err: unknown) => setError(err instanceof Error ? err.message : String(err)));
                }}
              />
            </label>
            {Object.keys(sprites).length > 0 && (
              <button type="button" className={quiet} onClick={() => setSprites({})}>
                Reset images
              </button>
            )}
          </div>
        </aside>
      </main>
    </div>
  );
}
