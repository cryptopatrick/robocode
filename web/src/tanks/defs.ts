// Shared definitions for the five tanks. Decoration only: physics and
// collision sizes never read this table.

export type TankId = 'leopard2a8' | 'abrams' | 'k2' | 'leclerc' | 'merkava'

export const TANK_IDS: TankId[] = ['leopard2a8', 'abrams', 'k2', 'leclerc', 'merkava']

export const DEFAULT_TANKS: TankId[] = [...TANK_IDS]

/** A cut sprite: PNG size and the pixel it rotates about. */
export interface ArtPart {
  w: number
  h: number
  pivotX: number
  pivotY: number
}

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
  /** px per art pixel: makes the hull 28 px wide. */
  scale: number
  /** Cut sprite sizes and rotation pivots, in art pixels (from scripts/cut-tanks.mjs). */
  art: { hull: ArtPart; turret: ArtPart }
  colors: { base: number; secondary: number; armor: number }
}

// hullLength, pivotY, barrelReach and art are measured from the cut sprites
// (npm run cut-tanks); hullLength = art.hull.h * scale.
export const TANKS: Record<TankId, TankDef> = {
  leopard2a8: {
    id: 'leopard2a8',
    name: 'Leopard 2A8',
    hullLength: 46.4,
    hullWidth: 28,
    pivotY: -0.1,
    barrelReach: 28.3,
    scale: 0.1393,
    art: {
      hull: { w: 201, h: 333, pivotX: 97.5, pivotY: 167.5 },
      turret: { w: 171, h: 323, pivotX: 85, pivotY: 209.5 },
    },
    colors: { base: 0x77775b, secondary: 0x535746, armor: 0x96947b },
  },
  abrams: {
    id: 'abrams',
    name: 'M1A2 Abrams SEPv3',
    hullLength: 47,
    hullWidth: 28,
    pivotY: 0.4,
    barrelReach: 29.1,
    scale: 0.1407,
    art: {
      hull: { w: 199, h: 334, pivotX: 92.5, pivotY: 164.5 },
      turret: { w: 186, h: 312, pivotX: 93, pivotY: 208.5 },
    },
    colors: { base: 0x777354, secondary: 0x4f5140, armor: 0x999477 },
  },
  k2: {
    id: 'k2',
    name: 'K2 Black Panther',
    hullLength: 45.8,
    hullWidth: 28,
    pivotY: 0.2,
    barrelReach: 28.9,
    scale: 0.1379,
    art: {
      hull: { w: 203, h: 332, pivotX: 101.5, pivotY: 164.5 },
      turret: { w: 174, h: 313, pivotX: 83, pivotY: 212 },
    },
    colors: { base: 0x626b4c, secondary: 0x454c3c, armor: 0x89916a },
  },
  leclerc: {
    id: 'leclerc',
    name: 'Leclerc XLR',
    hullLength: 45.1,
    hullWidth: 28,
    pivotY: 0.3,
    barrelReach: 28.8,
    scale: 0.1366,
    art: {
      hull: { w: 205, h: 330, pivotX: 102.5, pivotY: 162.5 },
      turret: { w: 175, h: 323, pivotX: 83.5, pivotY: 212 },
    },
    colors: { base: 0x777a5b, secondary: 0x4d5540, armor: 0x989879 },
  },
  merkava: {
    id: 'merkava',
    name: 'Merkava (inspired)',
    hullLength: 46.8,
    hullWidth: 28,
    pivotY: 0.4,
    barrelReach: 29.1,
    scale: 0.14,
    art: {
      hull: { w: 200, h: 334, pivotX: 99.5, pivotY: 164.5 },
      turret: { w: 182, h: 323, pivotX: 89, pivotY: 209 },
    },
    colors: { base: 0x9b9270, secondary: 0x625d4c, armor: 0xb1a587 },
  },
}

export function defaultTank(i: number): TankId {
  return DEFAULT_TANKS[((i % DEFAULT_TANKS.length) + DEFAULT_TANKS.length) % DEFAULT_TANKS.length]
}

/** The tank robot `i` drives in a recording; falls back to the slot default. */
export function tankFor(rec: { tanks?: string[] }, i: number): TankId {
  const id = rec.tanks?.[i]
  return id !== undefined && isTankId(id) ? id : defaultTank(i)
}

export function isTankId(s: string): s is TankId {
  return (TANK_IDS as string[]).includes(s)
}
