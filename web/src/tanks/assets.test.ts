// @vitest-environment node
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { PNG } from 'pngjs'
import { TANKS, TANK_IDS } from './defs'

function load(id: string, part: 'hull' | 'turret'): PNG {
  const p = fileURLToPath(new URL(`../assets/tanks/${id}-${part}.png`, import.meta.url))
  return PNG.sync.read(readFileSync(p))
}

type Rule = (r: number, g: number, b: number) => boolean
const red: Rule = (r, g, b) => r > g + 60 && r > b + 60
const yellowGreen: Rule = (r, g, b) => g > r + 20 && g > b + 60

/** Count semi-transparent pixels matching `rule` within 3 px of full transparency. */
function fringe(png: PNG, rule: Rule): number {
  const { width: w, height: h, data } = png
  let n = 0
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const i = (y * w + x) * 4
      if (data[i + 3] === 255 || !rule(data[i], data[i + 1], data[i + 2])) continue
      let edge = false
      for (let dy = -3; dy <= 3 && !edge; dy++) {
        for (let dx = -3; dx <= 3; dx++) {
          const xx = x + dx
          const yy = y + dy
          if (xx < 0 || yy < 0 || xx >= w || yy >= h || data[(yy * w + xx) * 4 + 3] === 0) {
            edge = true
            break
          }
        }
      }
      if (edge) n++
    }
  }
  return n
}

describe('tank art', () => {
  it('all ten images exist', () => {
    for (const id of TANK_IDS) {
      expect(load(id, 'hull').width).toBeGreaterThan(0)
      expect(load(id, 'turret').width).toBeGreaterThan(0)
    }
  })
  it('hulls point up and are taller than wide', () => {
    for (const id of TANK_IDS) {
      const p = load(id, 'hull')
      expect(p.height).toBeGreaterThan(p.width * 1.5)
    }
  })
  it('no red fringe on the Abrams turret', () => {
    expect(fringe(load('abrams', 'turret'), red)).toBe(0)
  })
  it('no yellow-green fringe', () => {
    for (const id of TANK_IDS) {
      for (const part of ['hull', 'turret'] as const) {
        expect(fringe(load(id, part), yellowGreen), `${id} ${part}`).toBe(0)
      }
    }
  })
  it('art metadata matches the files', () => {
    for (const id of TANK_IDS) {
      const { hull, turret } = TANKS[id].art
      const h = load(id, 'hull')
      const t = load(id, 'turret')
      expect([hull.w, hull.h]).toEqual([h.width, h.height])
      expect([turret.w, turret.h]).toEqual([t.width, t.height])
    }
  })
  it('scale makes hulls 28 px wide', () => {
    for (const id of TANK_IDS) {
      expect(Math.abs(TANKS[id].art.hull.w * TANKS[id].scale - 28)).toBeLessThanOrEqual(0.5)
    }
  })
})
