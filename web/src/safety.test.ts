import { describe, expect, it } from 'vitest';
import { safetyChecks } from './safety';
import type { Decision } from './recording';

const ready: Decision = {
  energy: 50_000, gun_heat: 0, gun_heading: 0,
  target: { bearing: 0, distance: 300, age: 0 },
  fire: 1000, ahead: 100, turn_body: null, turn_gun: 0, turn_radar: 450,
};

const oks = (d: Decision, power = 1000) => safetyChecks(d, power).map((c) => c.ok);

describe('safetyChecks', () => {
  it('lists the four conditions of safe_to_fire in order', () => {
    expect(safetyChecks(ready, 1000).map((c) => c.label)).toEqual([
      'power is 0.1 to 3.0', 'energy > power', 'gun is cool', 'target seen this turn', 'aimed within 3.0°',
    ]);
    expect(oks(ready)).toEqual([true, true, true, true, true]);
  });

  it('flags an illegal power', () => {
    expect(oks(ready, 99)[0]).toBe(false);
    expect(oks(ready, 100)[0]).toBe(true);
    expect(oks(ready, 3000)[0]).toBe(true);
    expect(oks(ready, 3001)[0]).toBe(false);
  });

  it('flags a hot gun', () => {
    expect(oks({ ...ready, gun_heat: 1 })).toEqual([true, true, false, true, true]);
  });

  it('needs energy strictly above the power', () => {
    expect(oks({ ...ready, energy: 1000 })[1]).toBe(false);
    expect(oks({ ...ready, energy: 1001 })[1]).toBe(true);
  });

  it('wraps angles through 0', () => {
    expect(oks({ ...ready, gun_heading: 3595, target: { bearing: 5, distance: 300, age: 0 } })[4]).toBe(true);
    expect(oks({ ...ready, target: { bearing: 31, distance: 300, age: 0 } })[4]).toBe(false);
  });

  it('fails the target checks with no target or a stale one', () => {
    expect(oks({ ...ready, target: null })).toEqual([true, true, true, false, false]);
    expect(oks({ ...ready, target: { bearing: 0, distance: 300, age: 1 } })[3]).toBe(false);
  });
});
