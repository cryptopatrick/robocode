// Battle sounds, made in the browser with Web Audio: no audio files. `cuesForTurn` and `engineLevels` read what
// happened in a recording (pure, tested); `createBattleSound` turns that into noise bursts and tones.
import type { Recording } from './recording';

export type CueKind = 'fire' | 'hit' | 'obstacle' | 'wall' | 'collision' | 'death';

export interface Cue {
  kind: CueKind;
  /** Stereo position, -0.8 (left edge of the arena) to 0.8 (right edge). */
  pan: number;
  /** Shot power (0.1 to 3) for fire and hit; 1 otherwise. */
  power: number;
}

const panFor = (x: number, width: number) => Math.max(-0.8, Math.min(0.8, (x / width) * 1.6 - 0.8));

/** What is heard on a turn: shots fired, tanks hit, buildings and walls struck, tanks colliding, tanks destroyed. */
export function cuesForTurn(rec: Recording, turn: number): Cue[] {
  const snap = rec.turns[turn];
  if (!snap) return [];
  const w = rec.battlefield.width;
  const at = (id: number) => snap.robots.find((r) => r.id === id)?.position;
  const cues: Cue[] = [];
  const add = (kind: CueKind, x: number | undefined, power = 1) => {
    if (x !== undefined) cues.push({ kind, pan: panFor(x, w), power });
  };
  const e = snap.events;
  for (const f of e.bullet_fired) add('fire', at(f.shooter)?.x, f.power);
  for (const h of e.bullet_hits) add('hit', at(h.victim)?.x, h.power);
  for (const o of e.bullet_hit_obstacles ?? []) add('obstacle', o.position.x, o.power);
  for (const c of e.wall_collisions) add('wall', c.position.x);
  // Both tanks of a collision are listed; the one at fault stands for the pair.
  for (const c of e.robot_collisions) if (c.at_fault) add('collision', at(c.robot_id)?.x);
  for (const id of e.robot_deaths) add('death', at(id)?.x);
  return cues;
}

/** How loud each tank's engine is: its speed over the top speed (8 px a turn), silent once it is destroyed. */
export function engineLevels(rec: Recording, turn: number): number[] {
  const snap = rec.turns[turn];
  return snap ? snap.robots.map((r) => (r.alive ? Math.min(1, Math.abs(r.velocity) / 8) : 0)) : [];
}

/** Each tank's stereo position at a turn. */
export function enginePans(rec: Recording, turn: number): number[] {
  const snap = rec.turns[turn];
  return snap ? snap.robots.map((r) => panFor(r.position.x, rec.battlefield.width)) : [];
}

export interface BattleSound {
  /** Switching on needs a user gesture in most browsers (the button press is one). */
  enable(on: boolean): void;
  enabled(): boolean;
  /** Call from a user gesture (Play): a context the browser left suspended is resumed. */
  unlock(): void;
  play(cues: Cue[]): void;
  engines(levels: number[], pans: number[]): void;
  silence(): void;
}

const MAX_CUES = 10;

export function createBattleSound(): BattleSound {
  let ctx: AudioContext | null = null;
  let master: GainNode | null = null;
  let noise: AudioBuffer | null = null;
  let on = false;
  const hums: { osc: OscillatorNode; gain: GainNode; pan: StereoPannerNode }[] = [];

  const ready = (): boolean => {
    if (!on) return false;
    if (!ctx) {
      const Ctor = typeof window !== 'undefined' ? (window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext) : undefined;
      if (!Ctor) return false;
      ctx = new Ctor();
      const compressor = ctx.createDynamicsCompressor();
      master = ctx.createGain();
      master.gain.value = 0.55;
      master.connect(compressor).connect(ctx.destination);
      noise = ctx.createBuffer(1, ctx.sampleRate * 2, ctx.sampleRate);
      const data = noise.getChannelData(0);
      for (let i = 0; i < data.length; i++) data[i] = Math.random() * 2 - 1;
    }
    return true;
  };

  /** One voice: filtered noise and/or a sweeping tone, panned, with a quick attack and a decay. */
  const voice = (
    pan: number,
    o: {
      noise?: { type: BiquadFilterType; f0: number; f1: number; q?: number; dur: number; gain: number };
      tone?: { type: OscillatorType; f0: number; f1: number; dur: number; gain: number };
    },
  ) => {
    if (!ctx || !master || !noise) return;
    const t0 = ctx.currentTime;
    const out = ctx.createStereoPanner();
    out.pan.value = pan;
    out.connect(master);
    const decay = (g: GainNode, peak: number, dur: number) => {
      g.gain.setValueAtTime(0.0001, t0);
      g.gain.exponentialRampToValueAtTime(peak, t0 + 0.005);
      g.gain.exponentialRampToValueAtTime(0.0001, t0 + dur);
    };
    if (o.noise) {
      const src = ctx.createBufferSource();
      src.buffer = noise;
      const f = ctx.createBiquadFilter();
      f.type = o.noise.type;
      f.Q.value = o.noise.q ?? 0.8;
      f.frequency.setValueAtTime(o.noise.f0, t0);
      f.frequency.exponentialRampToValueAtTime(o.noise.f1, t0 + o.noise.dur);
      const g = ctx.createGain();
      decay(g, o.noise.gain, o.noise.dur);
      src.connect(f).connect(g).connect(out);
      src.start(t0, Math.random());
      src.stop(t0 + o.noise.dur + 0.05);
    }
    if (o.tone) {
      const osc = ctx.createOscillator();
      osc.type = o.tone.type;
      osc.frequency.setValueAtTime(o.tone.f0, t0);
      osc.frequency.exponentialRampToValueAtTime(o.tone.f1, t0 + o.tone.dur);
      const g = ctx.createGain();
      decay(g, o.tone.gain, o.tone.dur);
      osc.connect(g).connect(out);
      osc.start(t0);
      osc.stop(t0 + o.tone.dur + 0.05);
    }
  };

  const sound: Record<CueKind, (c: Cue) => void> = {
    // A cannon: a bright crack that falls into a low thump; a heavier shell rings longer.
    fire: (c) => voice(c.pan, {
      noise: { type: 'lowpass', f0: 2600, f1: 260, dur: 0.22 + 0.07 * c.power, gain: 0.5 },
      tone: { type: 'sine', f0: 150, f1: 42, dur: 0.3 + 0.05 * c.power, gain: 0.6 },
    }),
    // A shell striking armour: a short metallic clank over a dull thud.
    hit: (c) => {
      voice(c.pan, {
        noise: { type: 'bandpass', f0: 1500, f1: 900, q: 1.5, dur: 0.12, gain: 0.45 },
        tone: { type: 'triangle', f0: 640, f1: 520, dur: 0.28, gain: 0.25 },
      });
      voice(c.pan, { tone: { type: 'square', f0: 930, f1: 870, dur: 0.18, gain: 0.08 } });
    },
    // A shell into a building: a muffled crumble.
    obstacle: (c) => voice(c.pan, {
      noise: { type: 'lowpass', f0: 700, f1: 180, dur: 0.25, gain: 0.45 },
      tone: { type: 'sine', f0: 85, f1: 50, dur: 0.22, gain: 0.5 },
    }),
    wall: (c) => voice(c.pan, {
      noise: { type: 'lowpass', f0: 300, f1: 120, dur: 0.14, gain: 0.25 },
      tone: { type: 'sine', f0: 62, f1: 40, dur: 0.18, gain: 0.5 },
    }),
    collision: (c) => voice(c.pan, {
      noise: { type: 'bandpass', f0: 650, f1: 400, q: 1, dur: 0.1, gain: 0.3 },
      tone: { type: 'triangle', f0: 125, f1: 70, dur: 0.2, gain: 0.5 },
    }),
    // A tank destroyed: a long roaring fall.
    death: (c) => voice(c.pan, {
      noise: { type: 'lowpass', f0: 1200, f1: 70, dur: 1.4, gain: 0.7 },
      tone: { type: 'sine', f0: 58, f1: 28, dur: 1.4, gain: 0.6 },
    }),
  };

  const stopHums = () => {
    for (const h of hums) {
      try {
        h.osc.stop();
      } catch {
        /* already stopped */
      }
      h.osc.disconnect();
    }
    hums.length = 0;
  };

  return {
    enable(v) {
      on = v;
      if (v) {
        if (ready()) void ctx?.resume();
      } else {
        stopHums();
      }
    },
    enabled: () => on,
    unlock() {
      if (ready() && ctx?.state === 'suspended') void ctx.resume();
    },
    play(cues) {
      if (!ready()) return;
      // A turn with many events keeps the loudest kinds: destruction first, then shots and hits.
      const rank: Record<CueKind, number> = { death: 0, fire: 1, hit: 2, obstacle: 3, collision: 4, wall: 5 };
      for (const c of [...cues].sort((a, b) => rank[a.kind] - rank[b.kind]).slice(0, MAX_CUES)) sound[c.kind](c);
    },
    engines(levels, pans) {
      if (!ready() || !ctx || !master) return;
      const now = ctx.currentTime;
      levels.forEach((level, i) => {
        if (!hums[i]) {
          // A low rumble: a sawtooth through a low-pass filter; louder and higher the faster the tank goes.
          const osc = ctx!.createOscillator();
          osc.type = 'sawtooth';
          osc.frequency.value = 40 + i * 3;
          const filter = ctx!.createBiquadFilter();
          filter.type = 'lowpass';
          filter.frequency.value = 170;
          const gain = ctx!.createGain();
          gain.gain.value = 0;
          const pan = ctx!.createStereoPanner();
          osc.connect(filter).connect(gain).connect(pan).connect(master!);
          osc.start();
          hums[i] = { osc, gain, pan };
        }
        const h = hums[i];
        h.gain.gain.setTargetAtTime(0.06 * level, now, 0.08);
        h.osc.frequency.setTargetAtTime(40 + i * 3 + 22 * level, now, 0.1);
        h.pan.pan.setTargetAtTime(pans[i] ?? 0, now, 0.1);
      });
    },
    silence() {
      if (!ctx) return;
      const now = ctx.currentTime;
      for (const h of hums) h.gain.gain.setTargetAtTime(0, now, 0.05);
    },
  };
}
