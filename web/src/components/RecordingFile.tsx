import { parseRecording, type Recording } from '../recording';

interface Props {
  recording: Recording | null;
  onOpen: (rec: Recording) => void;
  onError: (message: string) => void;
}

const quiet =
  'rounded-md border border-slate-300 bg-white px-3 py-1 text-sm text-slate-700 hover:bg-slate-50 focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-600 focus-within:ring-2 focus-within:ring-sky-600';

export function recordingFileName(rec: Recording): string {
  return `roboverus-${rec.robots.map((r) => r.name).join('-')}-${rec.outcome.turns}.json`;
}

export default function RecordingFile({ recording, onOpen, onError }: Props) {
  const save = () => {
    if (!recording) return;
    const url = URL.createObjectURL(new Blob([JSON.stringify(recording)], { type: 'application/json' }));
    const a = document.createElement('a');
    a.href = url;
    a.download = recordingFileName(recording);
    a.click();
    URL.revokeObjectURL(url);
  };
  return (
    <div className="flex flex-wrap gap-2">
      <button type="button" className={quiet} disabled={!recording} onClick={save}>
        Save recording
      </button>
      <label className={`${quiet} cursor-pointer`}>
        Open recording…
        <input
          type="file"
          accept="application/json,.json"
          className="sr-only"
          onChange={async (e) => {
            const file = e.target.files?.[0];
            e.target.value = '';
            if (!file) return;
            try {
              onOpen(parseRecording(await file.text()));
            } catch (err) {
              onError(`${file.name}: ${err instanceof Error ? err.message : String(err)}`);
            }
          }}
        />
      </label>
    </div>
  );
}
