import { useEffect, useMemo, useRef } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls.js';
import { ROBOT_COLORS } from '../draw';
import type { Recording } from '../recording';
import { BUILDING_HEIGHT, cameraFor, drivenDistances, sceneKey, HULL_HEIGHT, terrainBoxes, tankPose } from '../scene';
import { tankFor } from '../tanks/defs';
import { createTank3D, type Tank3D } from '../tanks/models3d';

const BACKGROUND = 0x0b1624;
const GROUND = 0x22395a;
const ROAD = 0x2e4260;
const BUILDING = 0x6d86a3;
const BULLET = 0xffd166;

interface Props {
  recording: Recording;
  turn: number;
}

interface Built {
  tanks: Tank3D[];
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

  const tanks = rec.robots.map((_, i) => {
    const tank = createTank3D(tankFor(rec, i), tankColor(i));
    scene.add(tank.root);
    return tank;
  });

  const bullets = new THREE.Group();
  scene.add(bullets);
  const bulletGeometry = new THREE.SphereGeometry(3, 8, 8);
  const bulletMaterial = material(BULLET);

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
    for (const bullet of snap.bullets) {
      const mesh = new THREE.Mesh(b.bulletGeometry, b.bulletMaterial);
      mesh.position.set(bullet.position.x - width / 2, HULL_HEIGHT, bullet.position.y - height / 2);
      b.bullets.add(mesh);
    }
    b.render();
  }, [recording, turn, dist]);

  return <div ref={container} className="aspect-[4/3] w-full overflow-hidden rounded-lg bg-[#0f1e2e]" role="img" aria-label="Battle in 3D" />;
}
