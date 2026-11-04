import { describe, expect, it } from 'vitest';
import { headingToCanvas, scanArc } from './geometry';

const deg = (d: number) => (d * Math.PI) / 180;

describe('geometry', () => {
  it('heading 0 points up, 90 points right', () => {
    expect(headingToCanvas(0)).toBeCloseTo(-Math.PI / 2);
    expect(headingToCanvas(90)).toBeCloseTo(0);
    expect(headingToCanvas(180)).toBeCloseTo(Math.PI / 2);
  });

  it('scan arc wraps through 0', () => {
    expect(scanArc(350, 10).sweep).toBeCloseTo(deg(20));
    expect(scanArc(10, 350).sweep).toBeCloseTo(deg(-20));
    expect(scanArc(350, 10).start).toBeCloseTo(headingToCanvas(350));
  });

  it('scan arc is zero when the radar is still', () => {
    expect(scanArc(45, 45).sweep).toBeCloseTo(0);
  });
});
