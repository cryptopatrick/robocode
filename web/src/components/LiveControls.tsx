import { SPEEDS } from '../playback';

/** What the page knows about the running simulation. */
export interface LiveUi {
  paused: boolean;
  /** Target turns per second; `null` = as fast as possible. */
  tps: number | null;
  /** Turns per second the simulation actually achieves. */
  measuredTps: number;
  finished: boolean;
  /** Whether the view follows the newest turn (vs. a turn scrubbed back to). */
  following: boolean;
}

interface Props {
  live: LiveUi;
  turn: number;
  last: number;
  onPause: () => void;
  onResume: () => void;
  onStep: () => void;
  onTps: (tps: number | null) => void;
  onSeek: (turn: number) => void;
  onBackToLive: () => void;
}

const base =
  'rounded-md border px-3 py-1.5 text-sm font-medium focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-600 ' +
  'disabled:cursor-not-allowed disabled:opacity-40';
const button = `${base} border-slate-300 bg-white text-slate-800 hover:bg-slate-50`;
const primary = `${base} min-w-20 border-slate-800 bg-slate-800 text-white hover:bg-slate-700`;

export default function LiveControls({ live, turn, last, onPause, onResume, onStep, onTps, onSeek, onBackToLive }: Props) {
  const running = !live.finished;
  return (
    <div className="flex flex-wrap items-center gap-2">
      {running ? (
        <span className="flex items-center gap-1.5 rounded-md bg-[#c9433b]/10 px-2 py-1 text-xs font-semibold text-[#8f2a24]">
          <span aria-hidden className={`h-2 w-2 rounded-full bg-[#c9433b] ${live.paused ? '' : 'animate-pulse'}`} />
          LIVE
        </span>
      ) : (
        <span className="rounded-md bg-slate-200 px-2 py-1 text-xs font-semibold text-slate-700">Finished</span>
      )}
      <button
        type="button"
        className={primary}
        disabled={!running}
        onClick={live.paused ? onResume : onPause}
      >
        {live.paused ? 'Resume' : 'Pause'}
      </button>
      <button type="button" className={button} disabled={!running || !live.paused} onClick={onStep} aria-label="Step forward">
        ▶︎
      </button>
      <label className="ml-2 flex items-center gap-1.5 text-sm text-slate-600">
        Speed
        <select
          className="rounded-md border border-slate-300 bg-white px-2 py-1 text-sm focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-600"
          value={live.tps === null ? 'max' : String(live.tps)}
          onChange={(e) => onTps(e.target.value === 'max' ? null : Number(e.target.value))}
          disabled={!running}
        >
          {SPEEDS.map((s) => (
            <option key={s} value={s}>
              {s} turns/s
            </option>
          ))}
          <option value="max">Max</option>
        </select>
      </label>
      <span className="text-sm tabular-nums text-slate-500">
        {Math.round(live.measuredTps)} / {live.tps === null ? 'max' : live.tps} turns/s
      </span>
      <input
        type="range"
        aria-label="Seek"
        className="mx-2 min-w-40 flex-1 accent-slate-800"
        min={0}
        max={last}
        value={turn}
        onChange={(e) => onSeek(Number(e.target.value))}
      />
      {!live.following && (
        <button type="button" className={button} onClick={onBackToLive}>
          Back to live
        </button>
      )}
      <span className="w-28 text-right text-sm tabular-nums text-slate-700">
        Turn {turn} / {last}
      </span>
    </div>
  );
}
