import type { Recording } from '../recording';

export default function Results({ recording }: { recording: Recording }) {
  const { winner, turns, energies } = recording.outcome;
  const name = (i: number) => recording.robots[i]?.name ?? `Robot ${i}`;
  return (
    <div className="rounded-md border border-slate-300 bg-white px-3 py-2.5 text-sm">
      <p className="font-semibold">{winner === null ? 'No winner' : `${name(winner)} wins`}</p>
      <p className="text-slate-600">after {turns} turns</p>
      <ul className="mt-2 flex flex-col gap-0.5 tabular-nums">
        {energies.map((e, i) => (
          <li key={i} className="flex justify-between">
            <span>{name(i)}</span>
            <span className="text-slate-600">{Math.max(e, 0).toFixed(1)}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}
