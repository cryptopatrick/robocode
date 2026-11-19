import { ROBOT_COLORS } from '../draw';
import type { Decision, RobotSnapshot } from '../recording';

interface Props {
  names: string[];
  robots: RobotSnapshot[];
  selected: number;
  onSelect: (i: number) => void;
  /** The recording's decisions, one row per turn; with `turn`, shows the maneuver's length. */
  decisions?: (Decision | null)[][];
  turn?: number;
}

/** "Attack · 14 turns": the selected robot's maneuver and how long it has run. */
export function maneuverRun(decisions: (Decision | null)[][], turn: number, i: number): string {
  const name = decisions[turn]?.[i]?.intent?.maneuver;
  if (!name) return '';
  let n = 1;
  while (n <= turn && decisions[turn - n]?.[i]?.intent?.maneuver === name) n++;
  return `${name[0].toUpperCase()}${name.slice(1)} · ${n} ${n === 1 ? 'turn' : 'turns'}`;
}

// Tailwind needs literal class names, so the swatch colours are listed here.
const SWATCH = ['bg-[#e0a526]', 'bg-[#3fb6c9]', 'bg-[#ef5d79]', 'bg-[#9c7cf4]'];

function status(r: RobotSnapshot): string {
  if (!r.alive) return 'Dead';
  if (r.energy <= 0) return 'Disabled';
  return r.energy.toFixed(1);
}

export default function RobotPanel({ names, robots, selected, onSelect, decisions, turn }: Props) {
  const r = robots[selected];
  const run = decisions && turn !== undefined ? maneuverRun(decisions, turn, selected) : '';
  return (
    <div className="flex flex-col gap-3">
      <ul className="flex flex-col gap-1">
        {robots.map((robot, i) => (
          <li key={robot.id}>
            <button
              type="button"
              aria-pressed={i === selected}
              onClick={() => onSelect(i)}
              className={`grid w-full grid-cols-[12px_1fr_auto] items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-600 ${
                i === selected ? 'bg-white shadow-sm ring-1 ring-slate-300' : 'hover:bg-white/60'
              } ${robot.alive ? '' : 'text-slate-400'}`}
            >
              <span className={`h-3 w-3 rounded-sm ${SWATCH[i % SWATCH.length]}`} />
              <span className="flex flex-col gap-1">
                <span className="font-medium">{names[i]}</span>
                <svg viewBox="0 0 100 4" className="h-1 w-full" aria-hidden>
                  <rect width="100" height="4" rx="2" fill="#d6dde4" />
                  <rect width={Math.max(0, Math.min(robot.energy, 100))} height="4" rx="2" fill={ROBOT_COLORS[i % ROBOT_COLORS.length]} />
                </svg>
              </span>
              <span className="tabular-nums text-slate-600">{status(robot)}</span>
            </button>
          </li>
        ))}
      </ul>
      {r && (
        <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 px-2 text-sm tabular-nums text-slate-600">
          {run && (
            <>
              <dt>Maneuver</dt>
              <dd className="text-slate-900">{run}</dd>
            </>
          )}
          <dt>Energy</dt>
          <dd className="text-slate-900">{r.energy.toFixed(2)}</dd>
          <dt>Gun heat</dt>
          <dd className="text-slate-900">{r.gun_heat.toFixed(2)}</dd>
          <dt>Velocity</dt>
          <dd className="text-slate-900">{r.velocity.toFixed(1)} px/turn</dd>
          <dt>Body</dt>
          <dd className="text-slate-900">{r.heading.toFixed(1)}°</dd>
          <dt>Gun</dt>
          <dd className="text-slate-900">{r.gun_heading.toFixed(1)}°</dd>
          <dt>Radar</dt>
          <dd className="text-slate-900">{r.radar_heading.toFixed(1)}°</dd>
          <dt>Position</dt>
          <dd className="text-slate-900">
            {r.position.x.toFixed(0)}, {r.position.y.toFixed(0)}
          </dd>
        </dl>
      )}
    </div>
  );
}
