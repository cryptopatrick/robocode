// Procedural low-poly 3D tanks, ported from the supplied createTank factory.
// Built in the file's frame (x forward, y up, z width) at file units, inside a
// `model` group that is scaled to battlefield px and turned so the nose is the
// root's +z. Hull footprint, turret pivot and barrel reach come from TANKS.
import * as THREE from 'three'
import { TANKS, type TankId } from './defs'

const WRECK = 0x4a5866
const PAD_COUNT = 40

export interface Tank3D {
  root: THREE.Group
  turret: THREE.Group
  radar: THREE.Object3D
  muzzle: THREE.Mesh
  setTrackDistance(d: number): void
  setDead(dead: boolean): void
}

/** Per-tank look in file units (length is the hull length; width comes from the defs). */
interface Look {
  length: number
  hullHeight: number
  turretLength: number
  turretWidth: number
  turretHeight: number
  wheels: number
  trackRadius: number
  angular: boolean
  armorBlocks: boolean
}

const LOOKS: Record<TankId, Look> = {
  leopard2a8: { length: 4.6, hullHeight: 0.85, turretLength: 2.15, turretWidth: 1.95, turretHeight: 0.68, wheels: 7, trackRadius: 0.57, angular: true, armorBlocks: true },
  abrams: { length: 4.8, hullHeight: 0.9, turretLength: 2.5, turretWidth: 2.05, turretHeight: 0.78, wheels: 7, trackRadius: 0.58, angular: false, armorBlocks: true },
  k2: { length: 4.5, hullHeight: 0.77, turretLength: 2.1, turretWidth: 1.85, turretHeight: 0.57, wheels: 6, trackRadius: 0.55, angular: true, armorBlocks: false },
  leclerc: { length: 4.45, hullHeight: 0.76, turretLength: 2.0, turretWidth: 1.85, turretHeight: 0.59, wheels: 6, trackRadius: 0.53, angular: true, armorBlocks: true },
  merkava: { length: 5.0, hullHeight: 0.98, turretLength: 2.0, turretWidth: 1.85, turretHeight: 0.66, wheels: 6, trackRadius: 0.57, angular: false, armorBlocks: true },
}

type V3 = [number, number, number]

/** Point on the rounded-rectangle track loop in the x/y plane. */
function trackPoint(distance: number, straight: number, radius: number): [number, number] {
  const arc = Math.PI * radius
  const total = 2 * straight + 2 * arc
  let s = ((distance % total) + total) % total
  if (s < straight) return [-straight / 2 + s, -radius]
  s -= straight
  if (s < arc) {
    const a = -Math.PI / 2 + s / radius
    return [straight / 2 + radius * Math.cos(a), radius * Math.sin(a)]
  }
  s -= arc
  if (s < straight) return [straight / 2 - s, radius]
  s -= straight
  const a = Math.PI / 2 + s / radius
  return [-straight / 2 + radius * Math.cos(a), radius * Math.sin(a)]
}

export function createTank3D(id: TankId, teamColor: number): Tank3D {
  const def = TANKS[id]
  const look = LOOKS[id]

  // Uniform scale so the hull is hullLength long; the hull is widened (not the
  // scale) so the whole footprint, tracks and skirts included, is hullWidth.
  const k = def.hullLength / look.length
  const width = def.hullWidth / k - 0.32
  const h = look.hullHeight
  const r = look.trackRadius
  const pivotX = def.pivotY / k
  // The 2D turret ring sits off the hull axis by ringX px (+x = tank's right).
  // The model's +z (file frame) is the tank's right, so the offset is +z here.
  const ringZ = ((def.art.hull.pivotX - def.art.hull.w / 2) * def.scale) / k

  // One material per colour per tank, so a wreck never recolours another tank.
  const originals = new Map<THREE.MeshLambertMaterial, number>()
  const mat = (color: number) => {
    const m = new THREE.MeshLambertMaterial({ color, flatShading: true })
    originals.set(m, color)
    return m
  }
  const m = {
    hull: mat(def.colors.base),
    armor: mat(def.colors.armor),
    secondary: mat(def.colors.secondary),
    track: mat(0x222522),
    tread: mat(0x454844),
    wheel: mat(0x30342f),
    hub: mat(0x858575),
    glass: mat(0x263e42),
    dark: mat(0x242823),
    light: mat(0xd4bd80),
    team: mat(teamColor),
  }

  const root = new THREE.Group()
  // The file's +x (forward) becomes the root's +z; the model sits on the ground.
  const model = new THREE.Group()
  model.rotation.y = -Math.PI / 2
  model.scale.setScalar(k)
  model.position.y = (h / 2) * k
  root.add(model)

  const box = (parent: THREE.Object3D, size: V3, pos: V3, material: THREE.Material) => {
    const mesh = new THREE.Mesh(new THREE.BoxGeometry(...size), material)
    mesh.position.set(...pos)
    parent.add(mesh)
    return mesh
  }
  /** Cylinder whose axis is the width (z) by default, or the length (x) for barrel parts. */
  const cyl = (parent: THREE.Object3D, rad: number, len: number, pos: V3, material: THREE.Material, sides = 10, axis: 'x' | 'z' = 'z') => {
    const mesh = new THREE.Mesh(new THREE.CylinderGeometry(rad, rad, len, sides), material)
    if (axis === 'z') mesh.rotation.x = Math.PI / 2
    else mesh.rotation.z = Math.PI / 2
    mesh.position.set(...pos)
    parent.add(mesh)
    return mesh
  }

  // ---------------- hull ----------------
  const hull = new THREE.Group()
  model.add(hull)
  box(hull, [look.length, h, width], [0, 0, 0], m.hull)
  // Sloped glacis.
  const glacis = box(hull, [0.9, h * 0.8, width * 0.96], [look.length / 2 - 0.4, h * 0.12, 0], m.armor)
  glacis.rotation.z = -0.22
  // Rear engine deck and grilles.
  box(hull, [1.05, 0.22, width * 0.85], [-look.length / 2 + 0.48, h * 0.58, 0], m.secondary)
  for (let i = 0; i < 5; i++) {
    box(hull, [0.06, 0.025, width * 0.65], [-look.length / 2 + 0.2 + i * 0.14, h * 0.72, 0], m.dark)
  }
  for (const side of [-1, 1]) {
    box(hull, [0.12, 0.13, 0.16], [look.length / 2 - 0.12, h * 0.1, side * width * 0.37], m.light)
    // Side armour skirt.
    box(hull, [look.length * 0.78, 0.58, 0.12], [-0.05, -0.08, side * (width / 2 + 0.1)], m.armor)
  }
  if (id === 'merkava') {
    // Rear troop compartment and access hatch.
    box(hull, [1.5, 0.45, width * 0.92], [-look.length * 0.34, h * 0.73, 0], m.armor)
    box(hull, [0.12, 0.48, width * 0.7], [-look.length / 2 - 0.04, 0, 0], m.secondary)
  }

  // ---------------- turret ----------------
  const turretGroup = new THREE.Group()
  turretGroup.position.set(pivotX, h * 0.65, ringZ)
  model.add(turretGroup)
  const t = turretGroup
  const tl = look.turretLength
  const tw = look.turretWidth
  const th = look.turretHeight
  const roof = 0.42 + th / 2

  cyl(t, tw * 0.43, 0.24, [0, 0.1, 0], m.secondary, 8)
  box(t, [tl, th, tw], [0, 0.42, 0], m.armor)
  const tFront = box(t, [0.55, th * 0.9, tw * 0.9], [tl * 0.36, 0.42, 0], m.hull)
  tFront.rotation.z = -0.17
  cyl(t, 0.25, 0.09, [-0.45, roof, 0], m.secondary, 8)
  cyl(t, 0.27, 0.45, [tl * 0.5, 0.48, 0], m.secondary, 10)
  box(t, [0.24, 0.16, 0.3], [-0.48, roof + 0.22, 0], m.glass)

  // Barrel: its tip lands barrelReach from the robot centre.
  const gunX = tl * 0.5
  const barrelLength = def.barrelReach / k - pivotX - gunX
  const barrel = new THREE.Mesh(new THREE.CylinderGeometry(0.095, 0.14, barrelLength, 8), m.secondary)
  barrel.rotation.z = -Math.PI / 2
  barrel.position.set(gunX + barrelLength / 2, 0.48, 0)
  t.add(barrel)
  const muzzle = cyl(t, 0.15, 0.24, [gunX + barrelLength - 0.12, 0.48, 0], m.team, 8, 'x')

  if (look.armorBlocks) {
    for (const side of [-1, 1]) {
      for (let i = 0; i < 3; i++) {
        box(t, [0.48, 0.38, 0.22], [-0.45 + i * 0.48, 0.35, side * (tw / 2 + 0.08)], m.secondary)
      }
    }
  }
  if (look.angular) box(t, [0.7, 0.3, 0.35], [-0.45, 0.76, 0.5], m.hull)

  // Surveillance cameras, APS sensors, roof stowage and antenna.
  for (const p of [[0.55, 0.78, 0.78], [0.55, 0.78, -0.78], [-0.65, 0.78, 0.78], [-0.65, 0.78, -0.78]] as V3[]) {
    box(t, [0.18, 0.14, 0.18], p, m.glass)
  }
  for (const side of [-1, 1]) box(t, [0.25, 0.22, 0.3], [0.05, 0.75, side * 0.9], m.dark)
  box(t, [0.55, 0.08, 0.5], [-0.1, 0.83, -0.42], m.secondary)
  cyl(t, 0.018, 0.85, [-0.7, 1.1, -0.5], m.dark, 5, 'z').rotation.set(0, 0, 0)

  // Radar dish on the roof at the pivot (Task 7 spins it).
  const radar = new THREE.Group()
  radar.position.set(0, roof, 0)
  const mast = new THREE.Mesh(new THREE.CylinderGeometry(0.04, 0.04, 0.2, 5), m.dark)
  mast.position.y = 0.1
  const dish = new THREE.Mesh(new THREE.CylinderGeometry(0.34, 0.06, 0.1, 8), m.light)
  dish.position.y = 0.25
  radar.add(mast, dish)
  t.add(radar)

  // ---------------- tracks ----------------
  const straight = look.length - r * 2
  const perimeter = 2 * straight + 2 * Math.PI * r
  const trackY = -h / 2 + r + 0.09
  const padGeo = new THREE.BoxGeometry((perimeter / PAD_COUNT) * 0.88, 0.17, 0.15)
  const sides = [1, -1].map((side) => {
    const g = new THREE.Group()
    model.add(g)
    // Continuous dark belt behind the pads.
    const beltShape = new THREE.Shape()
    const half = straight / 2
    const outerR = r + 0.09
    const innerR = r - 0.09
    beltShape.moveTo(-half, -outerR)
    beltShape.lineTo(half, -outerR)
    beltShape.absarc(half, 0, outerR, -Math.PI / 2, Math.PI / 2, false)
    beltShape.lineTo(-half, outerR)
    beltShape.absarc(-half, 0, outerR, Math.PI / 2, Math.PI * 1.5, false)
    const hole = new THREE.Path()
    hole.moveTo(-half, -innerR)
    hole.lineTo(half, -innerR)
    hole.absarc(half, 0, innerR, -Math.PI / 2, Math.PI / 2, false)
    hole.lineTo(-half, innerR)
    hole.absarc(-half, 0, innerR, Math.PI / 2, Math.PI * 1.5, false)
    beltShape.holes.push(hole)
    const belt = new THREE.Mesh(new THREE.ExtrudeGeometry(beltShape, { depth: 0.12, bevelEnabled: false }), m.track)
    belt.position.set(0, trackY, side * (width / 2) - 0.06)
    g.add(belt)

    const pads: THREE.Mesh[] = []
    for (let i = 0; i < PAD_COUNT; i++) {
      const pad = new THREE.Mesh(padGeo, m.tread)
      g.add(pad)
      pads.push(pad)
    }
    const wheels: THREE.Mesh[] = []
    for (let i = 0; i < look.wheels; i++) {
      const x = -straight / 2 + (i / (look.wheels - 1)) * straight
      wheels.push(cyl(g, r * 0.49, 0.19, [x, trackY, side * (width / 2 + 0.035)], m.wheel, 12))
      wheels.push(cyl(g, r * 0.2, 0.21, [x, trackY, side * (width / 2 + 0.04)], m.hub, 8))
    }
    return { side, pads, wheels }
  })

  /** `distance` is battlefield px driven; the track is built in file units (px / k). */
  function setTrackDistance(distance: number) {
    const d = distance / k
    for (const { side, pads, wheels } of sides) {
      pads.forEach((pad, i) => {
        // Driving forward, the ground run moves backwards relative to the hull.
        const s = (i / PAD_COUNT) * perimeter - d
        const [x, y] = trackPoint(s, straight, r)
        const [nx, ny] = trackPoint(s - 0.01, straight, r)
        pad.position.set(x, y + trackY, side * (width / 2 + 0.075))
        pad.rotation.z = Math.atan2(ny - y, nx - x)
      })
      // Wheel axes lie along z after the x-rotation, so spin about the cylinder's own y.
      for (const w of wheels) w.rotation.y = -d / r
    }
  }
  setTrackDistance(0)

  function setDead(dead: boolean) {
    for (const [mt, color] of originals) mt.color.setHex(dead ? WRECK : color)
    root.scale.y = dead ? 0.4 : 1
  }

  return { root, turret: turretGroup, radar, muzzle, setTrackDistance, setDead }
}
