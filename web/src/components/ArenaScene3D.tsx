import { useEffect, useMemo, useRef } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { ROBOT_COLORS } from '../draw';
import type { Recording } from '../recording';
import { roadsFor, sceneryFor } from '../scenery';
import { BUILDING_HEIGHT, bulletSegment, worldPoint, cameraFor, drivenDistances, impactFlashes, sceneKey, HULL_HEIGHT, streakPose, terrainBoxes, tankPose } from '../scene';
import { houseStyle, roadDashes, TILE, textureCanvas, type TextureKind } from '../textures';
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

  // Textures are tiled canvases (see ../textures); `owned` is what dispose() must free besides the materials.
  const owned: THREE.Texture[] = [];
  const surface = (hex: number, kind: TextureKind, repeatX: number, repeatY: number): THREE.MeshLambertMaterial => {
    const canvas = textures ? textureCanvas(kind, `#${hex.toString(16).padStart(6, '0')}`) : null;
    if (!canvas) return material(hex);
    const map = new THREE.CanvasTexture(canvas);
    map.wrapS = map.wrapT = THREE.RepeatWrapping;
    map.repeat.set(repeatX, repeatY);
    map.colorSpace = THREE.SRGBColorSpace;
    map.anisotropy = 4;
    owned.push(map);
    return new THREE.MeshLambertMaterial({ map });
  };

  const ground = new THREE.Mesh(new THREE.PlaneGeometry(width, height), surface(c.ground, 'ground', width / TILE, height / TILE));
  ground.rotation.x = -Math.PI / 2;
  scene.add(ground);
  if (textures) {
    // The grass runs on past the walls to the horizon, fading into the sky with distance.
    const far = 60000;
    const horizon = new THREE.Mesh(new THREE.PlaneGeometry(far, far), surface(c.ground, 'ground', far / TILE / 2, far / TILE / 2));
    horizon.rotation.x = -Math.PI / 2;
    horizon.position.y = -0.3;
    scene.add(horizon);
    scene.fog = new THREE.Fog(c.background, 2500, 16000);
  }

  const terrain = terrainBoxes(rec);
  const dashMaterial = new THREE.MeshBasicMaterial({ color: 0xe9eef2 });
  // With scenery on, the roads are the map's own or the generated large road and small paths (see ../scenery).
  const roads: { box: [number, number, number, number]; big: boolean }[] = textures
    ? roadsFor(rec).map(({ rect: [x, y, w, h], big }) => {
        const p = worldPoint(x + w / 2, y + h / 2, width, height);
        return { box: [p.x, p.z, w, h], big };
      })
    : terrain.roads.map((box) => ({ box, big: true }));
  for (const { box: [x, z, w, d], big } of roads) {
    const road = new THREE.Mesh(
      new THREE.BoxGeometry(w, 0.5, d),
      big ? surface(c.road, 'road', w / TILE, d / TILE) : surface(daylight ? 0x9a8c70 : 0x4d4639, 'path', w / TILE, d / TILE),
    );
    road.position.set(x, 0.25, z);
    scene.add(road);
    if (textures && big) {
      for (const [dx, dz, dw, dd] of roadDashes([x - w / 2, z - d / 2, w, d])) {
        const dash = new THREE.Mesh(new THREE.BoxGeometry(dw, 0.1, dd), dashMaterial);
        dash.position.set(dx + dw / 2, 0.56, dz + dd / 2);
        scene.add(dash);
      }
    }
  }
  terrain.buildings.forEach(([x, z, w, d], i) => {
    // Faces of a box: +x, -x, top, bottom, +z, -z. Each building has its own roof and facade (brick, stucco, glass).
    const style = houseStyle(i, daylight);
    const hex = (css: string) => parseInt(css.slice(1), 16);
    const side = (along: number) => surface(hex(style.wallColor), style.wall, along / 96, 0.25);
    const materials = textures
      ? [side(d), side(d), surface(hex(style.roofColor), style.roof, w / TILE, d / TILE), material(hex(style.wallColor)), side(w), side(w)]
      : material(c.building);
    const building = new THREE.Mesh(new THREE.BoxGeometry(w, BUILDING_HEIGHT, d), materials);
    building.position.set(x, BUILDING_HEIGHT / 2, z);
    scene.add(building);
  });

  if (textures) {
    // Scenery: low-poly trees, bushes and rocks, the same places the 2D drawing uses.
    const flat = (hex: number) => new THREE.MeshLambertMaterial({ color: hex, flatShading: true });
    const leaf = [flat(daylight ? 0x3a8a3a : 0x1d4a33), flat(daylight ? 0x5cad4c : 0x2b6845)];
    const bushMat = flat(daylight ? 0x5a9d48 : 0x24583b);
    const rockMat = flat(daylight ? 0x9a9ea6 : 0x555e6c);
    const trunkMat = flat(daylight ? 0x7a5a3a : 0x3d2f24);
    for (const p of sceneryFor(rec)) {
      if (p.kind === 'tuft') continue;
      const { x, z } = worldPoint(p.x, p.y, width, height);
      let mesh: THREE.Object3D;
      if (p.kind === 'tree') {
        const s = p.r / 12;
        const tree = new THREE.Group();
        const trunk = new THREE.Mesh(new THREE.CylinderGeometry(1.3 * s, 1.9 * s, 7 * s, 6), trunkMat);
        trunk.position.y = 3.5 * s;
        const pine = p.v < 0.5;
        const crown = new THREE.Mesh(
          pine ? new THREE.ConeGeometry(p.r * 0.95, p.r * 2.6, 7) : new THREE.IcosahedronGeometry(p.r * 0.95, 0),
          leaf[p.v < 0.25 || p.v > 0.75 ? 0 : 1],
        );
        crown.position.y = 7 * s + (pine ? p.r * 1.2 : p.r * 0.7);
        tree.add(trunk, crown);
        mesh = tree;
      } else if (p.kind === 'bush') {
        const bush = new THREE.Mesh(new THREE.SphereGeometry(p.r, 6, 4), bushMat);
        bush.scale.y = 0.7;
        bush.position.y = p.r * 0.45;
        mesh = bush;
      } else {
        const rock = new THREE.Mesh(new THREE.DodecahedronGeometry(p.r * 0.9, 0), rockMat);
        rock.position.y = p.r * 0.35;
        rock.rotation.set(p.v * 3, p.v * 7, 0);
        mesh = rock;
      }
      mesh.position.x = x;
      mesh.position.z = z;
      scene.add(mesh);
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

  const camera = new THREE.PerspectiveCamera(40, container.clientWidth / container.clientHeight, 2, 80000);
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
