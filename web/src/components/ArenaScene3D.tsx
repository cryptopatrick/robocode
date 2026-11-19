import { useEffect, useMemo, useRef } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { ROBOT_COLORS } from '../draw';
import type { Recording } from '../recording';
import { BUILDING_HEIGHT, bulletSegment, cameraFor, drivenDistances, impactFlashes, sceneKey, HULL_HEIGHT, streakPose, terrainBoxes, tankPose } from '../scene';
import { tankFor } from '../tanks/defs';
import { createTank3D, type Tank3D } from '../tanks/models3d';

const BACKGROUND = 0x0b1624;
const GROUND = 0x22395a;
const ROAD = 0x2e4260;
const BUILDING = 0x6d86a3;
const TRACER = 0xfff6c8;
const FLASH = 0xfff0be;
/** Cross-section of a tracer streak, in battlefield pixels. */
const TRACER_THICKNESS = 1.5;
const IMPACT_TURNS = 4;

interface Props {
  recording: Recording;
  turn: number;
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
function build(container: HTMLElement, rec: Recording): Built {
  const { width, height } = rec.battlefield;
  const renderer = new THREE.WebGLRenderer({ antialias: true });
  renderer.setPixelRatio(window.devicePixelRatio);
  renderer.setSize(container.clientWidth, container.clientHeight);
  container.appendChild(renderer.domElement);

  const scene = new THREE.Scene();
  scene.background = new THREE.Color(BACKGROUND);
  scene.add(new THREE.AmbientLight(0xffffff, 1.1));
  const sun = new THREE.DirectionalLight(0xffffff, 1.2);
  sun.position.set(-300, 600, 400);
  scene.add(sun);

  const ground = new THREE.Mesh(new THREE.PlaneGeometry(width, height), material(GROUND));
  ground.rotation.x = -Math.PI / 2;
  scene.add(ground);

  const terrain = terrainBoxes(rec);
  for (const [x, z, w, d] of terrain.roads) {
    const road = new THREE.Mesh(new THREE.BoxGeometry(w, 0.5, d), material(ROAD));
    road.position.set(x, 0.25, z);
    scene.add(road);
  }
  for (const [x, z, w, d] of terrain.buildings) {
    const building = new THREE.Mesh(new THREE.BoxGeometry(w, BUILDING_HEIGHT, d), material(BUILDING));
    building.position.set(x, BUILDING_HEIGHT / 2, z);
    scene.add(building);
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
  const bulletMaterial = new THREE.MeshBasicMaterial({ color: TRACER });
  const flashes = new THREE.Group();
  scene.add(flashes);
  const flashTexture = glowTexture();

  const camera = new THREE.PerspectiveCamera(40, container.clientWidth / container.clientHeight, 1, 5000);
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
          (o.material as THREE.Material).dispose();
        }
      });
      renderer.dispose();
      renderer.domElement.remove();
    },
  };
}

/** The battle drawn in 3D. The scene is built once per recording; each turn moves the tanks and bullets. */
export default function ArenaScene3D({ recording, turn }: Props) {
  const container = useRef<HTMLDivElement>(null);
  const built = useRef<Built | null>(null);
  const key = sceneKey(recording);
  const dist = useMemo(() => drivenDistances(recording), [recording]);
  const latest = useRef({ recording, turn });
  latest.current = { recording, turn };

  useEffect(() => {
    const el = container.current;
    if (!el) return;
    const scene = build(el, latest.current.recording);
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
  }, [recording, turn, dist]);

  return <div ref={container} className="aspect-[4/3] w-full overflow-hidden rounded-lg bg-[#0f1e2e]" role="img" aria-label="Battle in 3D" />;
}
