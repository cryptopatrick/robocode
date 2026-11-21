import { useEffect, useMemo, useRef } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { ROBOT_COLORS } from '../draw';
import type { Recording } from '../recording';
import { roadsFor, sceneryFor } from '../scenery';
import { BUILDING_HEIGHT, bulletSegment, worldPoint, cameraFor, drivenDistances, impactFlashes, sceneKey, HULL_HEIGHT, streakPose, terrainBoxes, tankPose } from '../scene';
import { isCity, roofColor, tokyoPalette } from '../tokyo';
import { hatchCanvas, mapPalette, meadowsFor, plotsFor } from '../maplook';
import { tankFor } from '../tanks/defs';
import { createTank3D, type Tank3D } from '../tanks/models3d';

const BACKGROUND = 0x0b1624;
const GROUND = 0x22395a;
const ROAD = 0x2e4260;
const BUILDING = 0x6d86a3;
/** Daylight: a pale sky, light ground and roofs, a stronger sun. */
const DAY = { background: 0xa9d0f0, ground: 0x668f4c, road: 0x6b727c, building: 0xcdd1d8, ambient: 0.95, sun: 1.8, tracer: 0xd9480f };
const TRACER = 0xfff6c8;
const FLASH = 0xfff0be;
/** Cross-section of a tracer streak, in battlefield pixels. */
const TRACER_THICKNESS = 1.5;
const IMPACT_TURNS = 4;

interface Props {
  recording: Recording;
  turn: number;
  /** Procedural ground, road, roof and facade textures instead of flat colours (default off). */
  textures?: boolean;
  /** Daylight instead of night (default off). */
  daylight?: boolean;
}

interface Built {
  tanks: Tank3D[];
  bullets: THREE.Group;
  bulletGeometry: THREE.BoxGeometry;
  bulletMaterial: THREE.MeshBasicMaterial;
  flashes: THREE.Group;
  flashTexture: THREE.Texture;
  render: () => void;
  dispose: () => void;
}

const material = (color: number) => new THREE.MeshLambertMaterial({ color });

/** A robot's colour as a number, from the palette used everywhere in the viewer. */
const tankColor = (i: number): number => parseInt(ROBOT_COLORS[i % ROBOT_COLORS.length].slice(1), 16);

/** A soft round glow, white in the middle, for impact flash sprites. */
function glowTexture(): THREE.Texture {
  const c = document.createElement('canvas');
  c.width = c.height = 64;
  const g = c.getContext('2d');
  if (g) {
    const grad = g.createRadialGradient(32, 32, 0, 32, 32, 32);
    grad.addColorStop(0, 'rgba(255,255,255,1)');
    grad.addColorStop(0.35, 'rgba(255,230,150,0.8)');
    grad.addColorStop(1, 'rgba(255,200,80,0)');
    g.fillStyle = grad;
    g.fillRect(0, 0, 64, 64);
  }
  return new THREE.CanvasTexture(c);
}

/** Removes the flash sprites and frees their per-flash materials. */
function clearFlashes(flashes: THREE.Group) {
  for (const o of flashes.children) if (o instanceof THREE.Sprite) o.material.dispose();
  flashes.clear();
}

/** Builds the static scene for a recording: ground, map objects and one tank per robot. */
function build(container: HTMLElement, rec: Recording, textures: boolean, daylight: boolean): Built {
  const c = daylight
    ? DAY
    : { background: BACKGROUND, ground: GROUND, road: ROAD, building: BUILDING, ambient: 1.1, sun: 1.2, tracer: TRACER };
  const { width, height } = rec.battlefield;
  const renderer = new THREE.WebGLRenderer({ antialias: true });
  renderer.setPixelRatio(window.devicePixelRatio);
  renderer.setSize(container.clientWidth, container.clientHeight);
  container.appendChild(renderer.domElement);

  const scene = new THREE.Scene();
  scene.background = new THREE.Color(c.background);
  scene.add(new THREE.AmbientLight(0xffffff, c.ambient));
  const sun = new THREE.DirectionalLight(0xffffff, c.sun);
  sun.position.set(-300, 600, 400);
  scene.add(sun);

  // `owned` is what dispose() must free besides the materials: the textures made here.
  const owned: THREE.Texture[] = [];
  const hex = (css: string) => parseInt(css.slice(1), 16);
  const terrain = terrainBoxes(rec);

  if (!textures) {
    const ground = new THREE.Mesh(new THREE.PlaneGeometry(width, height), material(c.ground));
    ground.rotation.x = -Math.PI / 2;
    scene.add(ground);
    for (const [x, z, w, d] of terrain.roads) {
      const road = new THREE.Mesh(new THREE.BoxGeometry(w, 0.5, d), material(c.road));
      road.position.set(x, 0.25, z);
      scene.add(road);
    }
    for (const [x, z, w, d] of terrain.buildings) {
      const building = new THREE.Mesh(new THREE.BoxGeometry(w, BUILDING_HEIGHT, d), material(c.building));
      building.position.set(x, BUILDING_HEIGHT / 2, z);
      scene.add(building);
    }
  } else if (isCity(rec)) {
    // A real-city map: charcoal ground running to the horizon, street surfaces and linework, extruded cream buildings
    // at their real heights (at 0.6 of true scale so tanks stay visible between them).
    const pal = tokyoPalette(daylight);
    const flat = (css: string) => new THREE.MeshLambertMaterial({ color: new THREE.Color(css) });
    const sky = daylight ? 0xdfdcd4 : 0x17161a;
    scene.background = new THREE.Color(sky);
    scene.fog = new THREE.Fog(sky, 2500, 16000);
    const horizon = new THREE.Mesh(new THREE.PlaneGeometry(60000, 60000), flat(pal.ground));
    horizon.rotation.x = -Math.PI / 2;
    horizon.position.y = -0.3;
    scene.add(horizon);
    const ground = new THREE.Mesh(new THREE.PlaneGeometry(width, height), flat(pal.ground));
    ground.rotation.x = -Math.PI / 2;
    scene.add(ground);

    // Streets as flat ribbons: one mesh for the surfaces, one for the thin linework.
    const ribbons = (lines: { points: [number, number][]; width: number }[], y: number, css: string) => {
      const pos: number[] = [];
      for (const { points, width: lw } of lines) {
        for (let k = 0; k + 1 < points.length; k++) {
          const a = worldPoint(points[k][0], points[k][1], width, height), b = worldPoint(points[k + 1][0], points[k + 1][1], width, height);
          const dx = b.x - a.x, dz = b.z - a.z, len = Math.hypot(dx, dz);
          if (len < 0.01) continue;
          const nx = (-dz / len) * (lw / 2), nz = (dx / len) * (lw / 2);
          pos.push(a.x + nx, y, a.z + nz, a.x - nx, y, a.z - nz, b.x + nx, y, b.z + nz,
            b.x + nx, y, b.z + nz, a.x - nx, y, a.z - nz, b.x - nx, y, b.z - nz);
        }
      }
      const g = new THREE.BufferGeometry();
      g.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3));
      g.computeVertexNormals();
      const m = new THREE.Mesh(g, new THREE.MeshBasicMaterial({ color: new THREE.Color(css), side: THREE.DoubleSide }));
      scene.add(m);
    };
    const all = rec.terrain!.road_lines!;
    ribbons(all.filter((l) => l.width > 6), 0.2, pal.street);
    ribbons(all.map((l) => ({ points: l.points, width: l.width > 6 ? 1.4 : 0.9 })), 0.35, pal.line);

    const ppm = rec.terrain!.px_per_m ?? 4;
    terrain.buildings.forEach(([x, z, w, d], i) => {
      const m = rec.terrain!.heights?.[i] ?? 25;
      const hgt = Math.max(8, m * ppm * 0.6);
      const walls = flat(pal.wall);
      const roof = flat(roofColor(pal, m));
      const geometry = new THREE.BoxGeometry(w, hgt, d);
      const building = new THREE.Mesh(geometry, [walls, walls, roof, walls, walls, walls]);
      building.position.set(x, hgt / 2, z);
      scene.add(building);
      const outline = new THREE.LineSegments(new THREE.EdgesGeometry(geometry), new THREE.LineBasicMaterial({ color: daylight ? 0x6f695c : 0x2a2823 }));
      outline.position.copy(building.position);
      scene.add(outline);
    });
  } else {
    // The planning-map look (see ../maplook): pale ground running to the horizon, darker meadows, peach and pink
    // plots, white roads, cross-hatched buildings with dark outlines, pale round-crowned trees.
    const pal = mapPalette(daylight);
    const flat = (css: string) => new THREE.MeshLambertMaterial({ color: hex(css) });
    scene.background = new THREE.Color(daylight ? 0xcfe3f2 : c.background);
    scene.fog = new THREE.Fog(daylight ? 0xcfe3f2 : c.background, 2500, 16000);
    const horizon = new THREE.Mesh(new THREE.PlaneGeometry(60000, 60000), flat(pal.ground));
    horizon.rotation.x = -Math.PI / 2;
    horizon.position.y = -0.3;
    scene.add(horizon);
    const ground = new THREE.Mesh(new THREE.PlaneGeometry(width, height), flat(pal.ground));
    ground.rotation.x = -Math.PI / 2;
    scene.add(ground);

    const flatShape = (points: [number, number][], css: string, y: number) => {
      const shape = new THREE.Shape(points.map(([px, py]) => {
        const p = worldPoint(px, py, width, height);
        return new THREE.Vector2(p.x, -p.z);
      }));
      const mesh = new THREE.Mesh(new THREE.ShapeGeometry(shape), flat(css));
      mesh.rotation.x = -Math.PI / 2;
      mesh.position.y = y;
      scene.add(mesh);
    };
    // Meadows are smoothed blobs: a closed Catmull-Rom curve through the points.
    for (const pts of meadowsFor(rec)) {
      const curve = new THREE.CatmullRomCurve3(pts.map(([px, py]) => { const p = worldPoint(px, py, width, height); return new THREE.Vector3(p.x, 0, p.z); }), true);
      const smooth = curve.getPoints(60).map((v): [number, number] => [v.x, v.z]);
      const shape = new THREE.Shape(smooth.map(([wx, wz]) => new THREE.Vector2(wx, -wz)));
      const mesh = new THREE.Mesh(new THREE.ShapeGeometry(shape), flat(pal.meadow));
      mesh.rotation.x = -Math.PI / 2;
      mesh.position.y = 0.12;
      scene.add(mesh);
    }
    for (const { rect: [x, y, pw, ph], color } of plotsFor(rec, pal)) {
      flatShape([[x, y], [x + pw, y], [x + pw, y + ph], [x, y + ph]], color, 0.25);
    }
    // Roads sit above the plots, white and flat.
    for (const { rect: [x, y, rw, rh] } of roadsFor(rec)) {
      flatShape([[x, y], [x + rw, y], [x + rw, y + rh], [x, y + rh]], pal.road, 0.4);
    }

    const edge = new THREE.LineBasicMaterial({ color: hex(pal.ink) });
    terrain.buildings.forEach(([x, z, w, d], i) => {
      const look = pal.buildings[i % pal.buildings.length];
      const tile = hatchCanvas(look.fill, look.hatch);
      const roof = new THREE.MeshLambertMaterial({ color: 0xffffff });
      if (tile) {
        const map = new THREE.CanvasTexture(tile);
        map.wrapS = map.wrapT = THREE.RepeatWrapping;
        map.repeat.set(w / 8, d / 8);
        map.colorSpace = THREE.SRGBColorSpace;
        map.magFilter = THREE.NearestFilter;
        owned.push(map);
        roof.map = map;
      } else {
        roof.color.set(hex(look.fill));
      }
      // Faces of a box: +x, -x, top, bottom, +z, -z.
      const wall = flat(look.fill);
      const geometry = new THREE.BoxGeometry(w, BUILDING_HEIGHT, d);
      const building = new THREE.Mesh(geometry, [wall, wall, roof, wall, wall, wall]);
      building.position.set(x, BUILDING_HEIGHT / 2, z);
      scene.add(building);
      const outline = new THREE.LineSegments(new THREE.EdgesGeometry(geometry), edge);
      outline.position.copy(building.position);
      scene.add(outline);
    });

    // Trees: a short trunk under a pale round crown (an icosahedron, so it stays light on the triangles).
    const crown = flat(pal.treeFill), trunk = flat(pal.treeEdge);
    for (const p of sceneryFor(rec)) {
      if (p.kind !== 'tree') continue;
      const { x, z } = worldPoint(p.x, p.y, width, height);
      const stem = new THREE.Mesh(new THREE.CylinderGeometry(1.2, 1.6, p.r * 0.9, 6), trunk);
      stem.position.set(x, p.r * 0.45, z);
      const top = new THREE.Mesh(new THREE.IcosahedronGeometry(p.r * 0.95, 1), crown);
      top.position.set(x, p.r * 1.4, z);
      scene.add(stem, top);
    }
  }

  const tanks = rec.robots.map((_, i) => {
    const tank = createTank3D(tankFor(rec, i), tankColor(i));
    scene.add(tank.root);
    return tank;
  });

  const bullets = new THREE.Group();
  scene.add(bullets);
  // Unlit, so the streak glows whatever the lighting: a unit box scaled per bullet.
  const bulletGeometry = new THREE.BoxGeometry(TRACER_THICKNESS, TRACER_THICKNESS, 1);
  const bulletMaterial = new THREE.MeshBasicMaterial({ color: c.tracer });
  const flashes = new THREE.Group();
  scene.add(flashes);
  const flashTexture = glowTexture();

  const camera = new THREE.PerspectiveCamera(40, container.clientWidth / container.clientHeight, 10, 80000);
  {
    const view = cameraFor(width, height, camera.aspect);
    camera.fov = view.fov;
    camera.position.set(...view.position);
    camera.lookAt(...view.target);
    camera.updateProjectionMatrix();
  }

  const controls = new OrbitControls(camera, renderer.domElement);
  controls.enableDamping = true;
  controls.maxPolarAngle = Math.PI / 2 - 0.05;
  const render = () => renderer.render(scene, camera);
  controls.addEventListener('change', render);

  const onResize = () => {
    renderer.setSize(container.clientWidth, container.clientHeight);
    camera.aspect = container.clientWidth / container.clientHeight;
    camera.updateProjectionMatrix();
    render();
  };
  window.addEventListener('resize', onResize);

  return {
    tanks,
    bullets,
    bulletGeometry,
    bulletMaterial,
    flashes,
    flashTexture,
    render,
    dispose: () => {
      window.removeEventListener('resize', onResize);
      controls.dispose();
      bulletGeometry.dispose();
      bulletMaterial.dispose();
      clearFlashes(flashes);
      flashTexture.dispose();
      scene.traverse((o) => {
        if (o instanceof THREE.Mesh) {
          o.geometry.dispose();
          for (const m of Array.isArray(o.material) ? o.material : [o.material]) m.dispose();
        }
      });
      for (const t of owned) t.dispose();
      renderer.dispose();
      renderer.domElement.remove();
    },
  };
}

/** The battle drawn in 3D. The scene is built once per recording; each turn moves the tanks and bullets. */
export default function ArenaScene3D({ recording, turn, textures = false, daylight = false }: Props) {
  const container = useRef<HTMLDivElement>(null);
  const built = useRef<Built | null>(null);
  const key = sceneKey(recording) + (textures ? '|textures' : '') + (daylight ? '|day' : '');
  const dist = useMemo(() => drivenDistances(recording), [recording]);
  const latest = useRef({ recording, turn });
  latest.current = { recording, turn };

  useEffect(() => {
    const el = container.current;
    if (!el) return;
    const scene = build(el, latest.current.recording, textures, daylight);
    built.current = scene;
    return () => {
      scene.dispose();
      built.current = null;
    };
  // Rebuild only when the scene's contents change, not on every appended live frame.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);

  useEffect(() => {
    const b = built.current;
    if (!b) return;
    const { width, height } = recording.battlefield;
    const snap = recording.turns[turn];
    snap.robots.forEach((r, i) => {
      const pose = tankPose(r, width, height);
      const tank = b.tanks[i];
      tank.root.position.set(pose.x, 0, pose.z);
      tank.root.rotation.y = pose.hull;
      tank.root.visible = true;
      tank.turret.rotation.y = pose.gun;
      tank.radar.rotation.y = pose.radar - pose.gun;
      tank.setDead(!pose.alive);
      tank.setTrackDistance(dist[turn][i]);
    });

    b.bullets.clear();
    snap.bullets.forEach((bullet, j) => {
      if (!bullet.alive) return;
      const { from, to } = bulletSegment(recording, turn, j);
      const s = streakPose(from, to, width, height);
      const mesh = new THREE.Mesh(b.bulletGeometry, b.bulletMaterial);
      mesh.position.set(s.x, HULL_HEIGHT, s.z);
      mesh.rotation.y = s.rotation;
      mesh.scale.z = s.length;
      b.bullets.add(mesh);
    });

    clearFlashes(b.flashes);
    for (const f of impactFlashes(recording, turn, IMPACT_TURNS)) {
      const fade = 1 - f.age / IMPACT_TURNS;
      const sprite = new THREE.Sprite(
        new THREE.SpriteMaterial({
          map: b.flashTexture,
          color: FLASH,
          transparent: true,
          opacity: fade,
          blending: THREE.AdditiveBlending,
          depthWrite: false,
        }),
      );
      const size = 14 + f.power * 4 + f.age * 8;
      sprite.scale.set(size, size, 1);
      const p = { x: f.x - width / 2, z: f.y - height / 2 };
      sprite.position.set(p.x, HULL_HEIGHT, p.z);
      b.flashes.add(sprite);
    }
    b.render();
  // `key` too: a rebuilt scene (textures or daylight switched) has to be posed and drawn at once, not at the next turn.
  }, [recording, turn, dist, key]);

  return <div ref={container} className="aspect-[4/3] w-full overflow-hidden rounded-lg bg-[#0f1e2e]" role="img" aria-label="Battle in 3D" />;
}
