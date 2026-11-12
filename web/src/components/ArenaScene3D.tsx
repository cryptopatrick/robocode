import { useEffect, useRef } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { ROBOT_COLORS } from '../draw';
import type { Recording } from '../recording';
import { BUILDING_HEIGHT, HULL_HEIGHT, terrainBoxes, tankPose } from '../scene';

const BACKGROUND = 0x0b1624;
const GROUND = 0x22395a;
const ROAD = 0x2e4260;
const BUILDING = 0x6d86a3;
const WRECK = 0x4a5866;
const STEEL = 0xe9eef2;
const BULLET = 0xffd166;

interface Props {
  recording: Recording;
  turn: number;
}

interface Built {
  tanks: THREE.Group[];
  bodies: THREE.Mesh[];
  bullets: THREE.Group;
  bulletGeometry: THREE.SphereGeometry;
  bulletMaterial: THREE.MeshLambertMaterial;
  render: () => void;
  dispose: () => void;
}

const material = (color: number) => new THREE.MeshLambertMaterial({ color });

/** A robot's colour as a number, from the palette used everywhere in the viewer. */
const tankColor = (i: number): number => parseInt(ROBOT_COLORS[i % ROBOT_COLORS.length].slice(1), 16);

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

  const bodies: THREE.Mesh[] = [];
  const tanks = rec.robots.map((_, i) => {
    const tank = new THREE.Group();
    const hull = new THREE.Mesh(new THREE.BoxGeometry(36, HULL_HEIGHT, 36), material(tankColor(i)));
    hull.position.y = HULL_HEIGHT / 2;
    tank.add(hull);
    bodies.push(hull);

    const turret = new THREE.Group();
    turret.position.y = HULL_HEIGHT;
    const turretBlock = new THREE.Mesh(new THREE.BoxGeometry(22, 6, 22), material(STEEL));
    turretBlock.position.y = 3;
    turret.add(turretBlock);
    const gun = new THREE.Mesh(new THREE.BoxGeometry(4, 4, 28), material(STEEL));
    gun.position.set(0, 6, 14);
    turret.add(gun);
    tank.add(turret);

    const radar = new THREE.Mesh(new THREE.CylinderGeometry(5, 5, 4, 12), material(STEEL));
    radar.position.set(0, HULL_HEIGHT + 2, -10);
    tank.add(radar);

    tank.userData = { turret, radar };
    scene.add(tank);
    return tank;
  });

  const bullets = new THREE.Group();
  scene.add(bullets);
  const bulletGeometry = new THREE.SphereGeometry(3, 8, 8);
  const bulletMaterial = material(BULLET);

  const camera = new THREE.PerspectiveCamera(40, container.clientWidth / container.clientHeight, 1, 5000);
  camera.position.set(0, Math.max(width, height) * 0.9, Math.max(width, height) * 0.55);
  camera.lookAt(0, 0, 0);

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
    bodies,
    bullets,
    bulletGeometry,
    bulletMaterial,
    render,
    dispose: () => {
      window.removeEventListener('resize', onResize);
      controls.dispose();
      bulletGeometry.dispose();
      bulletMaterial.dispose();
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
  }, [recording]);

  useEffect(() => {
    const b = built.current;
    if (!b) return;
    const { width, height } = recording.battlefield;
    const snap = recording.turns[turn];
    snap.robots.forEach((r, i) => {
      const pose = tankPose(r, width, height);
      const tank = b.tanks[i];
      tank.position.set(pose.x, 0, pose.z);
      tank.rotation.y = pose.hull;
      tank.visible = true;
      const { turret, radar } = tank.userData as { turret: THREE.Group; radar: THREE.Mesh };
      turret.rotation.y = pose.gun;
      radar.rotation.y = pose.radar;
      const body = b.bodies[i];
      (body.material as THREE.MeshLambertMaterial).color.set(pose.alive ? tankColor(i) : WRECK);
      tank.scale.y = pose.alive ? 1 : 0.4;
    });

    b.bullets.clear();
    for (const bullet of snap.bullets) {
      const mesh = new THREE.Mesh(b.bulletGeometry, b.bulletMaterial);
      mesh.position.set(bullet.position.x - width / 2, HULL_HEIGHT, bullet.position.y - height / 2);
      b.bullets.add(mesh);
    }
    b.render();
  }, [recording, turn]);

  return <div ref={container} className="aspect-[4/3] w-full overflow-hidden rounded-lg bg-[#0f1e2e]" role="img" aria-label="Battle in 3D" />;
}
