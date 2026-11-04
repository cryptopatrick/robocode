import { describe, expect, it } from 'vitest';
import { matchSpriteFiles } from './sprites';

describe('matchSpriteFiles', () => {
  it('matches body, gun and radar by name, case-insensitively', () => {
    expect(matchSpriteFiles(['tank-body.png', 'Gun.PNG', 'x.txt'])).toEqual({ body: 0, gun: 1 });
    expect(matchSpriteFiles(['a.png', 'RADAR.png', 'body2.png', 'body1.png'])).toEqual({ radar: 1, body: 2 });
  });
  it('returns nothing for no files', () => {
    expect(matchSpriteFiles([])).toEqual({});
  });
});
