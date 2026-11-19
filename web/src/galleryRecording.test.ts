import { describe, expect, it } from 'vitest';
import { galleryRecording } from './galleryRecording';
import { tankFor } from './tanks/defs';

describe('galleryRecording', () => {
  it('has an alive and a dead copy of the tank, turret 30 degrees off', () => {
    const rec = galleryRecording('k2');
    const [a, d] = rec.turns[0].robots;
    expect(a.alive).toBe(true);
    expect(d.alive).toBe(false);
    expect(a.gun_heading - a.heading).toBe(30);
    expect(tankFor(rec, 0)).toBe('k2');
    expect(tankFor(rec, 1)).toBe('k2');
  });
});
