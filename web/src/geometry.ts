// Converting engine angles to canvas angles. The engine's 0° points up and
// angles grow clockwise; its y axis grows downward like the canvas, so only
// the angle needs converting.

/** Canvas rotation (radians) for an engine heading in degrees. */
export function headingToCanvas(deg: number): number {
  return ((deg - 90) * Math.PI) / 180;
}

/** Signed shortest turn from `from` to `to` in degrees, within (-180, 180]. */
export function shortestTurn(to: number, from: number): number {
  let d = (to - from) % 360;
  if (d > 180) d -= 360;
  if (d <= -180) d += 360;
  return d;
}

/** The radar sweep from the previous heading to the current one, as a canvas arc. */
export function scanArc(prevDeg: number, curDeg: number): { start: number; sweep: number } {
  return {
    start: headingToCanvas(prevDeg),
    sweep: (shortestTurn(curDeg, prevDeg) * Math.PI) / 180,
  };
}
