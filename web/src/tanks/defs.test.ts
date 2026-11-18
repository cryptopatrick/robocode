import { describe, expect, it } from 'vitest'
import { DEFAULT_TANKS, TANKS, TANK_IDS, defaultTank, isTankId } from './defs'

describe('tank defs', () => {
  it('ids are the five tanks', () => {
    expect(TANK_IDS).toEqual(['leopard2a8', 'abrams', 'k2', 'leclerc', 'merkava'])
    expect(DEFAULT_TANKS).toEqual(TANK_IDS)
  })
  it('defaults follow slot order', () => {
    expect(defaultTank(0)).toBe('leopard2a8')
    expect(defaultTank(3)).toBe('leclerc')
    expect(defaultTank(5)).toBe('leopard2a8')
  })
  it('every hull is 28 px wide and fits the art proportions', () => {
    for (const id of TANK_IDS) {
      expect(TANKS[id].hullWidth).toBe(28)
      expect(TANKS[id].hullLength).toBeGreaterThanOrEqual(40)
      expect(TANKS[id].hullLength).toBeLessThanOrEqual(56)
    }
  })
  it('barrels reach 20 to 28 px past the centre, K2 longest', () => {
    const reaches = TANK_IDS.map((id) => TANKS[id].barrelReach)
    for (const r of reaches) {
      expect(r).toBeGreaterThanOrEqual(20)
      expect(r).toBeLessThanOrEqual(28)
    }
    expect(TANKS.k2.barrelReach).toBe(Math.max(...reaches))
  })
  it('merkava turret is set back', () => {
    expect(TANKS.merkava.pivotY).toBeLessThan(0)
  })
  it('names have no country', () => {
    for (const id of TANK_IDS) {
      expect(TANKS[id].name).not.toMatch(/Germany|USA|Korea|France|Israel/)
    }
  })
  it('isTankId rejects unknown', () => {
    expect(isTankId('t34')).toBe(false)
    expect(isTankId('k2')).toBe(true)
  })
})
