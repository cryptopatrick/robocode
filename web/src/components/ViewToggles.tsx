import type { ViewOptions } from '../draw';

const LABELS: [keyof ViewOptions, string][] = [
  ['names', 'Names'],
  ['energy', 'Energy'],
  ['scanArcs', 'Scan arcs'],
  ['effects', 'Hit effects'],
];

interface Props {
  view: ViewOptions;
  onChange: (v: ViewOptions) => void;
}

export default function ViewToggles({ view, onChange }: Props) {
  return (
    <fieldset className="flex flex-wrap gap-x-4 gap-y-1 text-sm text-slate-700">
      <legend className="sr-only">Show on the arena</legend>
      {LABELS.map(([key, label]) => (
        <label key={key} className="flex items-center gap-1.5">
          <input
            type="checkbox"
            className="accent-slate-800"
            checked={view[key]}
            onChange={(e) => onChange({ ...view, [key]: e.target.checked })}
          />
          {label}
        </label>
      ))}
    </fieldset>
  );
}
