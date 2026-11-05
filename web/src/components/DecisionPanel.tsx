import type { Decision } from '../recording';
import { safetyChecks } from '../safety';

interface Props {
  decision: Decision | null;
  dead: boolean;
  /** False for Robot API robots, which never record decisions. */
  recordsDecisions?: boolean;
}

const deg = (decideg: number) => `${(decideg / 10).toFixed(1)}°`;

function actionLines(d: Decision): string[] {
  const lines: string[] = [];
  if (d.fire !== null) lines.push(`Fire ${(d.fire / 1000).toFixed(1)}`);
  if (d.ahead !== null) lines.push(d.ahead >= 0 ? `Ahead ${d.ahead} px` : `Back ${-d.ahead} px`);
  if (d.turn_body !== null) lines.push(`Turn body ${deg(d.turn_body)}`);
  if (d.turn_gun !== null) lines.push(`Turn gun ${deg(d.turn_gun)}`);
  if (d.turn_radar !== null) lines.push(`Turn radar ${deg(d.turn_radar)}`);
  return lines.length ? lines : ['Idle'];
}

export default function DecisionPanel({ decision, dead, recordsDecisions = true }: Props) {
  if (!decision && !recordsDecisions) {
    return (
      <p className="text-sm text-slate-500">
        This robot is written with the Robot API. Only fixed-point strategy robots, like SafeShooter, record their
        decisions for this panel.
      </p>
    );
  }
  if (!decision) {
    return <p className="text-sm text-slate-500">{dead ? 'no decision (dead)' : 'no decision yet'}</p>;
  }
  const power = decision.fire ?? 1000;
  const checks = safetyChecks(decision, power);
  const t = decision.target;
  return (
    <div className="flex flex-col gap-3 text-sm">
      <dl className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-0.5 tabular-nums text-slate-600">
        <dt>Saw energy</dt>
        <dd className="text-slate-900">{(decision.energy / 1000).toFixed(3)}</dd>
        <dt>Gun heat</dt>
        <dd className="text-slate-900">{(decision.gun_heat / 1000).toFixed(3)}</dd>
        <dt>Target</dt>
        <dd className="text-slate-900">
          {t ? `${deg(t.bearing)}, ${t.distance} px, seen ${t.age === 0 ? 'this turn' : `${t.age} turns ago`}` : 'none'}
        </dd>
      </dl>
      <ul className="flex flex-col gap-0.5 font-medium text-slate-900">
        {actionLines(decision).map((l) => (
          <li key={l}>{l}</li>
        ))}
      </ul>
      <div className="rounded-md bg-[#0f1e2e] px-3 py-2.5 text-[#c9d6e2]">
        <p className="mb-1.5 text-xs text-[#8aa0b4]">
          {decision.fire !== null ? 'safe_to_fire, as Verus proved it' : 'Would a 1.0 shot be safe?'}
        </p>
        <ul className="flex flex-col gap-1">
          {checks.map((c) => (
            <li key={c.label} className="flex items-center gap-2">
              <span aria-hidden className={c.ok ? 'text-[#5fd39b]' : 'text-[#ff7b72]'}>
                {c.ok ? '✓' : '✗'}
              </span>
              <span>{c.label}</span>
              <span className="sr-only">{c.ok ? 'holds' : 'fails'}</span>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
