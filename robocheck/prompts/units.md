## The robot's world (fixed-point units)

- Energy, gun heat and shot power are in thousandths: 1.0 = 1000. A shot's power is 100 (0.1) to 3000 (3.0).
- Angles are decidegrees, 0..3599: 0 is north (up), clockwise, so 900 is east.
- Distances and positions are whole pixels; the arena is 800 × 600 and y grows downward.
- Velocity is in decipixels per turn.
- `obs` (the observation at the start of the turn): `turn`, `energy`, `gun_heat`, `heading`, `gun_heading`,
  `radar_heading`, `velocity`, `x`, `y`, `field_width`, `field_height`, and `target`: the nearest robot scanned,
  if any, with `bearing` (absolute), `distance`, `energy`, `heading`, `velocity` and `age` (turns since it was
  scanned; 0 = this turn).
- `around`: `clearance_ahead` and `clearance_behind`, the free distance in pixels before the robot's body touches a
  wall or a building straight ahead or straight behind.
- The decision for the turn is an `Action`: `fire` (a power, or none), `ahead` (pixels to drive; negative drives
  backward; or none), `turn_body`, `turn_gun`, `turn_radar` (decidegrees, positive turns right; or none).
- The robot decides one turn at a time and remembers nothing between turns.
