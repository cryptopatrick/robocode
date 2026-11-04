import { SPEEDS, type Playback, type PlaybackAction } from '../playback';

interface Props {
  playback: Playback;
  dispatch: (a: PlaybackAction) => void;
}

const base =
  'rounded-md border px-3 py-1.5 text-sm font-medium focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-600 ' +
  'disabled:cursor-not-allowed disabled:opacity-40';
const button = `${base} border-slate-300 bg-white text-slate-800 hover:bg-slate-50`;
const primary = `${base} min-w-20 border-slate-800 bg-slate-800 text-white hover:bg-slate-700`;

export default function Controls({ playback, dispatch }: Props) {
  const { turn, last, playing, speed } = playback;
  return (
    <div className="flex flex-wrap items-center gap-2">
      <button type="button" className={button} onClick={() => dispatch({ type: 'restart' })}>
        Restart
      </button>
      <button type="button" className={button} disabled={turn === 0} onClick={() => dispatch({ type: 'step', by: -1 })} aria-label="Step back">
        ◀︎
      </button>
      <button
        type="button"
        className={primary}
        onClick={() => dispatch({ type: 'toggle' })}
      >
        {playing ? 'Pause' : 'Play'}
      </button>
      <button type="button" className={button} disabled={turn === last} onClick={() => dispatch({ type: 'step', by: 1 })} aria-label="Step forward">
        ▶︎
      </button>
      <label className="ml-2 flex items-center gap-1.5 text-sm text-slate-600">
        Speed
        <select
          className="rounded-md border border-slate-300 bg-white px-2 py-1 text-sm focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-600"
          value={speed}
          onChange={(e) => dispatch({ type: 'speed', speed: Number(e.target.value) })}
        >
          {SPEEDS.map((s) => (
            <option key={s} value={s}>
              {s} turns/s
            </option>
          ))}
        </select>
      </label>
      <input
        type="range"
        aria-label="Seek"
        className="mx-2 min-w-40 flex-1 accent-slate-800"
        min={0}
        max={last}
        value={turn}
        onChange={(e) => dispatch({ type: 'seek', turn: Number(e.target.value) })}
      />
      <span className="w-28 text-right text-sm tabular-nums text-slate-700">
        Turn {turn} / {last}
      </span>
    </div>
  );
}
