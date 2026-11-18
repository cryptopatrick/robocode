// Shared definitions for the five tanks. Decoration only: physics and
// collision sizes never read this table.

export type TankId = 'leopard2a8' | 'abrams' | 'k2' | 'leclerc' | 'merkava'

export const TANK_IDS: TankId[] = ['leopard2a8', 'abrams', 'k2', 'leclerc', 'merkava']

export const DEFAULT_TANKS: TankId[] = [...TANK_IDS]

export interface TankDef {
  id: TankId
  name: string
  /** Hull length in px (tracks included). */
  hullLength: number
  /** Hull width in px (tracks included); 28 for every tank. */
  hullWidth: number
  /** Turret pivot offset from the robot centre along the hull, positive forward. */
  pivotY: number
  /** Muzzle distance from the robot centre in px. */
  barrelReach: number
  colors: { base: number; secondary: number; armor: number }
}

// hullLength, pivotY and barrelReach are seeds from the proportions in
// _wb/tankmodels.md; the 2D art task replaces them with measured values.
export const TANKS: Record<TankId, TankDef> = {
  leopard2a8: {
    id: 'leopard2a8',
    name: 'Leopard 2A8',
    hullLength: 49.7,
    hullWidth: 28,
    pivotY: 1.1,
    barrelReach: 24.9,
    colors: { base: 0x77775b, secondary: 0x535746, armor: 0x96947b },
  },
  abrams: {
    id: 'abrams',
    name: 'M1A2 Abrams SEPv3',
    hullLength: 51.8,
    hullWidth: 28,
    pivotY: -0.5,
    barrelReach: 24.1,
    colors: { base: 0x777354, secondary: 0x4f5140, armor: 0x999477 },
  },
  k2: {
    id: 'k2',
    name: 'K2 Black Panther',
    hullLength: 48.6,
    hullWidth: 28,
    pivotY: 0.5,
    barrelReach: 25.6,
    colors: { base: 0x626b4c, secondary: 0x454c3c, armor: 0x89916a },
  },
  leclerc: {
    id: 'leclerc',
    name: 'Leclerc XLR',
    hullLength: 48.1,
    hullWidth: 28,
    pivotY: 0.5,
    barrelReach: 23.9,
    colors: { base: 0x777a5b, secondary: 0x4d5540, armor: 0x989879 },
  },
  merkava: {
    id: 'merkava',
    name: 'Merkava (inspired)',
    hullLength: 54,
    hullWidth: 28,
    pivotY: -3.2,
    barrelReach: 20.5,
    colors: { base: 0x9b9270, secondary: 0x625d4c, armor: 0xb1a587 },
  },
}

export function defaultTank(i: number): TankId {
  return DEFAULT_TANKS[((i % DEFAULT_TANKS.length) + DEFAULT_TANKS.length) % DEFAULT_TANKS.length]
}

export function isTankId(s: string): s is TankId {
  return (TANK_IDS as string[]).includes(s)
}
