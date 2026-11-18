import { useState } from 'react';
import { RATIOS, ratioSize } from '../wasm';

export interface BattleSetup {
  robots: string[];
  width: number;
  height: number;
  maxTurns: number;
  urban: boolean;
}

const field =
  'rounded-md border border-slate-300 bg-white px-2 py-1 text-sm focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-600';

interface Props {
  names: string[];
  onRun: (setup: BattleSetup) => void;
  /** Sizes come from the engine; tests pass a stub. */
  sizeOf?: (ratio: string) => [number, number];
  ratios?: readonly string[];
}

export default function NewBattle({ names, onRun, sizeOf = ratioSize, ratios = RATIOS }: Props) {
  const [robots, setRobots] = useState<string[]>(['Tactician', 'Circler']);
  const [arena, setArena] = useState(ratios[0]);
  const [maxTurns, setMaxTurns] = useState(5000);
  const [urban, setUrban] = useState(false);

  const setRobot = (i: number, name: string) => setRobots(robots.map((r, j) => (j === i ? name : r)));

  return (
    <form
      className="flex flex-wrap items-end gap-3"
      onSubmit={(e) => {
        e.preventDefault();
        const [width, height] = sizeOf(arena);
        onRun({ robots, width, height, maxTurns, urban });
      }}
    >
      {robots.map((r, i) => (
        <label key={i} className="flex flex-col gap-1 text-xs text-slate-600">
          Robot {i + 1}
          <span className="flex items-center gap-1">
            <select className={field} value={r} onChange={(e) => setRobot(i, e.target.value)}>
              {names.map((n) => (
                <option key={n}>{n}</option>
              ))}
            </select>
            {robots.length > 2 && (
              <button
                type="button"
                className="rounded px-1.5 text-slate-500 hover:text-slate-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-600"
                aria-label={`Remove robot ${i + 1}`}
                onClick={() => setRobots(robots.filter((_, j) => j !== i))}
              >
                ×
              </button>
            )}
          </span>
        </label>
      ))}
      {robots.length < 4 && (
        <button
          type="button"
          className="rounded-md border border-dashed border-slate-400 px-3 py-1 text-sm text-slate-600 hover:border-slate-700 hover:text-slate-900 focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-600"
          onClick={() => setRobots([...robots, names[0] ?? 'SittingDuck'])}
        >
          Add robot
        </button>
      )}
      <label className="flex flex-col gap-1 text-xs text-slate-600">
        Ratio
        <select className={field} value={arena} onChange={(e) => setArena(e.target.value)}>
          {ratios.map((r) => {
            const [w, h] = sizeOf(r);
            return (
              <option key={r} value={r}>
                {`${r} · ${w} × ${h}`}
              </option>
            );
          })}
        </select>
      </label>
      <label className="flex items-center gap-2 text-xs text-slate-600">
        <input type="checkbox" checked={urban} onChange={(e) => setUrban(e.target.checked)} />
        Urban (buildings block tanks and bullets)
      </label>
      <label className="flex flex-col gap-1 text-xs text-slate-600">
        Turn limit
        <input
          className={`${field} w-24 tabular-nums`}
          type="number"
          min={1}
          max={20000}
          value={maxTurns}
          onChange={(e) => setMaxTurns(Number(e.target.value))}
        />
      </label>
      <button
        type="submit"
        className="rounded-md bg-sky-700 px-4 py-1.5 text-sm font-semibold text-white hover:bg-sky-800 focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-600 focus-visible:ring-offset-2"
      >
        Run battle
      </button>
    </form>
  );
}
