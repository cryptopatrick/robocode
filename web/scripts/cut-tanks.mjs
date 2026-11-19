// Cuts the five tanks (hull + turret) out of the supplied sprite sheet.
// Usage: node scripts/cut-tanks.mjs <sheet.png> <out-dir>
// Layout: five columns in id order; row 1 hulls, row 2 turrets, rows below
// are pre-rotated turrets and are ignored.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs'
import { PNG } from 'pngjs'

const IDS = ['leopard2a8', 'abrams', 'k2', 'leclerc', 'merkava']
const PAD = 2
const HULL_WIDTH = 28

const [, , sheetPath, outDir] = process.argv
if (!sheetPath || !outDir) {
  console.error('usage: cut-tanks.mjs <sheet.png> <out-dir>')
  process.exit(1)
}
mkdirSync(outDir, { recursive: true })
const sheet = PNG.sync.read(readFileSync(sheetPath))
const W = sheet.width
const H = sheet.height
const A = (x, y) => sheet.data[(y * W + x) * 4 + 3]

// Row bands: hulls in the top 40%, turrets from 40% to 76%.
const BANDS = { hull: [0, Math.round(H * 0.4)], turret: [Math.round(H * 0.4), Math.round(H * 0.76)] }

/** Largest 4-connected component of alpha > 0 inside the box. Returns its pixel list. */
function largestComponent(x0, x1, y0, y1) {
  const seen = new Uint8Array(W * H)
  let best = []
  for (let sy = y0; sy < y1; sy++) {
    for (let sx = x0; sx < x1; sx++) {
      if (seen[sy * W + sx] || A(sx, sy) === 0) continue
      const comp = []
      const stack = [[sx, sy]]
      seen[sy * W + sx] = 1
      while (stack.length) {
        const [x, y] = stack.pop()
        comp.push([x, y])
        for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
          const nx = x + dx
          const ny = y + dy
          if (nx < x0 || nx >= x1 || ny < y0 || ny >= y1) continue
          if (seen[ny * W + nx] || A(nx, ny) === 0) continue
          seen[ny * W + nx] = 1
          stack.push([nx, ny])
        }
      }
      if (comp.length > best.length) best = comp
    }
  }
  return best
}

const isRed = (r, g, b) => r > g + 60 && r > b + 60
const isYellowGreen = (r, g, b) => g > r + 20 && g > b + 60

/** True when a fully transparent pixel (or the image border) lies within 3 px. */
function nearTransparent(png, x, y) {
  const { width: w, height: h, data } = png
  for (let dy = -3; dy <= 3; dy++) {
    for (let dx = -3; dx <= 3; dx++) {
      const xx = x + dx
      const yy = y + dy
      if (xx < 0 || yy < 0 || xx >= w || yy >= h || data[(yy * w + xx) * 4 + 3] === 0) return true
    }
  }
  return false
}

/**
 * Clear semi-transparent edge pixels whose colour is red or yellow-green
 * (the sheet's halo). Repeated until stable, so a halo thicker than 3 px is
 * peeled from the outside in; opaque pixels are never touched.
 */
function clean(png) {
  const { width: w, height: h, data } = png
  let cleared = 0
  for (let changed = true; changed; ) {
    changed = false
    const kill = []
    for (let y = 0; y < h; y++) {
      for (let x = 0; x < w; x++) {
        const i = (y * w + x) * 4
        const a = data[i + 3]
        if (a === 0 || a === 255) continue
        const [r, g, b] = [data[i], data[i + 1], data[i + 2]]
        if ((isRed(r, g, b) || isYellowGreen(r, g, b)) && nearTransparent(png, x, y)) kill.push(i)
      }
    }
    for (const i of kill) {
      data[i] = data[i + 1] = data[i + 2] = data[i + 3] = 0
      changed = true
      cleared++
    }
  }
  return cleared
}

function crop(comp) {
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity
  for (const [x, y] of comp) {
    if (x < minX) minX = x
    if (x > maxX) maxX = x
    if (y < minY) minY = y
    if (y > maxY) maxY = y
  }
  const w = maxX - minX + 1 + PAD * 2
  const h = maxY - minY + 1 + PAD * 2
  const png = new PNG({ width: w, height: h })
  for (const [x, y] of comp) {
    const s = (y * W + x) * 4
    const d = ((y - minY + PAD) * w + (x - minX + PAD)) * 4
    png.data[d] = sheet.data[s]
    png.data[d + 1] = sheet.data[s + 1]
    png.data[d + 2] = sheet.data[s + 2]
    png.data[d + 3] = sheet.data[s + 3]
  }
  return png
}

const solid = (png, x, y) => png.data[(y * png.width + x) * 4 + 3] > 128

/**
 * Turret ring: the hull deck has a darker circular hub at the ring centre.
 * Find the centre whose hub disc is darkest against the ring around it.
 */
function hullPivot(png) {
  const { width: w, height: h, data } = png
  const lum = (x, y) => {
    const i = (y * w + x) * 4
    return 0.3 * data[i] + 0.59 * data[i + 1] + 0.11 * data[i + 2]
  }
  const R_IN = 14, R_OUT = 26
  let best = -Infinity, bx = 0, by = 0
  for (let cy = Math.round(h * 0.35); cy < Math.round(h * 0.65); cy++) {
    for (let cx = Math.round(w * 0.3); cx < Math.round(w * 0.7); cx++) {
      let di = 0, dn = 0, ao = 0, an = 0
      for (let dy = -R_OUT; dy <= R_OUT; dy++) {
        for (let dx = -R_OUT; dx <= R_OUT; dx++) {
          const d2 = dx * dx + dy * dy
          if (d2 <= R_IN * R_IN) { di += lum(cx + dx, cy + dy); dn++ }
          else if (d2 <= R_OUT * R_OUT) { ao += lum(cx + dx, cy + dy); an++ }
        }
      }
      const score = ao / an - di / dn
      if (score > best) { best = score; bx = cx; by = cy }
    }
  }
  return { x: bx + 0.5, y: by + 0.5 }
}

/** Turret body centre: bbox of the rows wider than 40% of the widest row (excludes the barrel). */
function turretPivot(png) {
  const { width: w, height: h } = png
  const widths = []
  const left = [], right = []
  for (let y = 0; y < h; y++) {
    let l = -1, r = -1
    for (let x = 0; x < w; x++) if (solid(png, x, y)) { if (l < 0) l = x; r = x }
    left.push(l); right.push(r); widths.push(l < 0 ? 0 : r - l + 1)
  }
  const max = Math.max(...widths)
  const rows = widths.map((v, y) => (v > max * 0.4 ? y : -1)).filter((y) => y >= 0)
  const top = rows[0], bottom = rows[rows.length - 1]
  let minL = Infinity, maxR = -Infinity
  for (let y = top; y <= bottom; y++) { minL = Math.min(minL, left[y]); maxR = Math.max(maxR, right[y]) }
  // Muzzle tip: first solid row of the barrel.
  const tip = widths.findIndex((v) => v > 0)
  return { x: (minL + maxR) / 2 + 0.5, y: (top + bottom) / 2 + 0.5, tip, top }
}

const r1 = (v) => Math.round(v * 10) / 10
const colW = W / IDS.length
const out = {}
for (let c = 0; c < IDS.length; c++) {
  const id = IDS[c]
  const x0 = Math.round(c * colW), x1 = Math.round((c + 1) * colW)
  const res = {}
  for (const part of ['hull', 'turret']) {
    const comp = largestComponent(x0, x1, BANDS[part][0], BANDS[part][1])
    const png = crop(comp)
    const cleared = clean(png)
    writeFileSync(`${outDir}/${id}-${part}.png`, PNG.sync.write(png))
    const p = part === 'hull' ? hullPivot(png) : turretPivot(png)
    res[part] = { w: png.width, h: png.height, pivotX: r1(p.x), pivotY: r1(p.y), cleared, tip: p.tip }
  }
  const scale = HULL_WIDTH / res.hull.w
  const hullLength = res.hull.h * scale
  // World offsets (px, +forward) of the turret pivot and muzzle from the hull centre.
  const pivotY = (res.hull.h / 2 - res.hull.pivotY) * scale
  const barrelReach = pivotY + (res.turret.pivotY - res.turret.tip) * scale
  out[id] = { ...res, scale: r1(scale * 1000) / 1000, hullLength: r1(hullLength), pivotY: r1(pivotY), barrelReach: r1(barrelReach) }
  console.log(id, JSON.stringify(out[id]))
}
