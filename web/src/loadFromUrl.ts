// Loading a recording named in the page URL, e.g. `?recording=/battle.json`
// (how `cargo robocode battle` opens the viewer).
import { parseRecording, type Recording } from './recording';

/** The `recording` query parameter, or null if absent or empty. */
export function recordingUrl(search: string): string | null {
  const value = new URLSearchParams(search).get('recording');
  return value ? value : null;
}

/** Fetches and parses a recording; throws an Error with a readable message. */
export async function loadRecording(url: string, fetchFn: typeof fetch = fetch): Promise<Recording> {
  const response = await fetchFn(url);
  if (!response.ok) {
    throw new Error(`Could not load ${url}: HTTP ${response.status}`);
  }
  return parseRecording(await response.text());
}

/** The `live` query parameter (a Server-Sent Events URL), or null. */
export function liveUrl(search: string): string | null {
  const value = new URLSearchParams(search).get('live');
  return value ? value : null;
}

/** A viewer command as a line for the CLI server's `POST /control`, or null if it has none. */
export function controlLine(m: import('./live').ToWorker): string | null {
  switch (m.type) {
    case 'pause':
    case 'resume':
    case 'step':
      return m.type;
    case 'tps':
      return `tps ${m.tps ?? 'max'}`;
    default:
      return null;
  }
}
