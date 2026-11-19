// Dev-only gallery of the five tanks (served at /gallery.html by `npm run dev`;
// not an input of the production build).
import { StrictMode, useEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import * as THREE from 'three';
import { drawArena, ROBOT_COLORS } from './draw';
import { GALLERY_HEIGHT, GALLERY_WIDTH, galleryRecording } from './galleryRecording';
import { TANK_IDS, TANKS, type TankId } from './tanks/defs';
import { loadTankImages, type TankImages } from './tanks/images';
import { createTank3D } from './tanks/models3d';
import './index.css';

const VIEW = { names: false, energy: false, scanArcs: false, effects: false };

function Canvas2D({ tank, images, zoom }: { tank: TankId; images: TankImages; zoom: number }) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const ctx = ref.current?.getContext('2d');
    if (!ctx) return;
    ctx.setTransform(zoom, 0, 0, zoom, 0, 0);
    drawArena(ctx, galleryRecording(tank), 0, VIEW, {}, images);
  }, [tank, images, zoom]);
  return <canvas ref={ref} width={GALLERY_WIDTH * zoom} height={GALLERY_HEIGHT * zoom} className="max-w-full rounded" />;
}

function Turntable({ tank }: { tank: TankId }) {
  const box = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const el = box.current;
    if (!el) return;
    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setSize(el.clientWidth, el.clientHeight);
    el.appendChild(renderer.domElement);
    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x0b1624);
    scene.add(new THREE.AmbientLight(0xffffff, 1.1));
    const sun = new THREE.DirectionalLight(0xffffff, 1.2);
    sun.position.set(-300, 600, 400);
    scene.add(sun);
    const model = createTank3D(tank, parseInt(ROBOT_COLORS[0].slice(1), 16));
    model.turret.rotation.y = (30 * Math.PI) / 180;
    scene.add(model.root);
    const camera = new THREE.PerspectiveCamera(40, el.clientWidth / el.clientHeight, 1, 1000);
    camera.position.set(0, 70, 110);
    camera.lookAt(0, 8, 0);
    let frame = 0;
    const tick = () => {
      model.root.rotation.y += 0.01;
      renderer.render(scene, camera);
      frame = requestAnimationFrame(tick);
    };
    tick();
    return () => {
      cancelAnimationFrame(frame);
      scene.traverse((o) => {
        if (o instanceof THREE.Mesh) {
          o.geometry.dispose();
          (o.material as THREE.Material).dispose();
        }
      });
      renderer.dispose();
      renderer.domElement.remove();
    };
  }, [tank]);
  return <div ref={box} className="aspect-[4/3] w-full max-w-xs overflow-hidden rounded bg-[#0f1e2e]" role="img" aria-label={`${TANKS[tank].name} in 3D`} />;
}

function Gallery() {
  const [images, setImages] = useState<TankImages | null>(null);
  useEffect(() => {
    void loadTankImages().then(setImages);
  }, []);
  return (
    <main className="mx-auto max-w-4xl space-y-8 p-6">
      <h1 className="text-2xl font-bold">Tank gallery</h1>
      <p className="text-sm text-slate-600">Turret turned 30 degrees. Each row: 2D at 1x and 4x (alive, then dead), and 3D on a turntable.</p>
      {!images ? (
        <p>Loading...</p>
      ) : (
        TANK_IDS.map((id) => (
          <section key={id} className="space-y-3">
            <h2 className="text-lg font-semibold">{TANKS[id].name}</h2>
            <div className="flex flex-wrap items-start gap-4">
              <Canvas2D tank={id} images={images} zoom={1} />
              <Turntable tank={id} />
            </div>
            <Canvas2D tank={id} images={images} zoom={4} />
          </section>
        ))
      )}
    </main>
  );
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Gallery />
  </StrictMode>,
);
