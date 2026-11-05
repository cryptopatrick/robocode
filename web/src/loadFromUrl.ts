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
