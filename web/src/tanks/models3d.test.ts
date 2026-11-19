import { describe, expect, it } from 'vitest'
import * as THREE from 'three'
import { TANKS, TANK_IDS } from './defs'
import { createTank3D } from './models3d'

const WRECK = 0x4a5866

function meshes(root: THREE.Object3D): THREE.Mesh[] {
  const out: THREE.Mesh[] = []
  root.traverse((o) => {
    if ((o as THREE.Mesh).isMesh) out.push(o as THREE.Mesh)
  })
  return out
}

/** Footprint of the hull and tracks alone: the turret group is detached while measuring. */
function hullBox(id: (typeof TANK_IDS)[number]): THREE.Vector3 {
  const t = createTank3D(id, 0xef5d79)
  const parent = t.turret.parent!
  parent.remove(t.turret)
  t.root.updateMatrixWorld(true)
  return new THREE.Box3().setFromObject(t.root).getSize(new THREE.Vector3())
}

describe('procedural 3D tanks', () => {
  it('footprint matches the definitions', () => {
    for (const id of TANK_IDS) {
      const size = hullBox(id)
      expect(Math.abs(size.x - 28) / 28, `${id} x ${size.x}`).toBeLessThan(0.1)
      const len = TANKS[id].hullLength
      expect(Math.abs(size.z - len) / len, `${id} z ${size.z}`).toBeLessThan(0.1)
    }
  })

  it('the muzzle tip sits at barrelReach and the turret pivots at pivotY', () => {
    for (const id of TANK_IDS) {
      const t = createTank3D(id, 0xef5d79)
      t.root.updateMatrixWorld(true)
      const tip = new THREE.Box3().setFromObject(t.muzzle).max.z
      expect(tip).toBeCloseTo(TANKS[id].barrelReach, 0)
      const p = new THREE.Vector3()
      t.turret.getWorldPosition(p)
      expect(p.z).toBeCloseTo(TANKS[id].pivotY, 1)
      expect(p.x).toBeCloseTo(0, 5)
      expect(t.radar.parent).toBe(t.turret)
    }
  })

  it('muzzle is team-coloured', () => {
    const m = createTank3D('k2', 0xef5d79).muzzle.material as THREE.MeshLambertMaterial
    expect(m.color.getHex()).toBe(0xef5d79)
  })

  it('track distance moves pads deterministically', () => {
    const t = createTank3D('abrams', 0xef5d79)
    const snap = () => meshes(t.root).map((m) => m.position.toArray().concat(m.rotation.toArray() as number[]))
    t.setTrackDistance(10)
    const a = snap()
    t.setTrackDistance(10)
    expect(snap()).toEqual(a)
    t.setTrackDistance(0)
    expect(snap()).not.toEqual(a)
  })

  it('dead tank is wreck-grey and flattened, and revives', () => {
    const t = createTank3D('merkava', 0xef5d79)
    const before = meshes(t.root).map((m) => (m.material as THREE.MeshLambertMaterial).color.getHex())
    t.setDead(true)
    for (const m of meshes(t.root)) {
      expect((m.material as THREE.MeshLambertMaterial).color.getHex()).toBe(WRECK)
    }
    expect(t.root.scale.y).toBe(0.4)
    t.setDead(false)
    expect(meshes(t.root).map((m) => (m.material as THREE.MeshLambertMaterial).color.getHex())).toEqual(before)
    expect(t.root.scale.y).toBe(1)
  })

  it('a dead tank never recolours another tank', () => {
    const a = createTank3D('k2', 0x111111)
    const b = createTank3D('k2', 0x222222)
    a.setDead(true)
    const c = (b.muzzle.material as THREE.MeshLambertMaterial).color.getHex()
    expect(c).toBe(0x222222)
    expect((meshes(b.root)[0].material as THREE.MeshLambertMaterial).color.getHex()).not.toBe(WRECK)
  })

  it('no gun elevation', () => {
    const t = createTank3D('leclerc', 0xef5d79)
    expect('gunSpeed' in t).toBe(false)
    expect('gun' in t).toBe(false)
  })
})
