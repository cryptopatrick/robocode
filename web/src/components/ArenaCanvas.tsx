import { useEffect, useRef, useState } from 'react';
import { drawArena, type ViewOptions } from '../draw';
import type { Recording } from '../recording';
import type { Sprites } from '../sprites';
import { loadTankImages, type TankImages } from '../tanks/images';

interface Props {
  recording: Recording;
  turn: number;
  view: ViewOptions;
  sprites: Sprites;
}

export default function ArenaCanvas({ recording, turn, view, sprites }: Props) {
  const ref = useRef<HTMLCanvasElement>(null);
  const [tanks, setTanks] = useState<TankImages>({});
  useEffect(() => {
    let live = true;
    loadTankImages().then((t) => live && setTanks(t));
    return () => {
      live = false;
    };
  }, []);
  useEffect(() => {
    const ctx = ref.current?.getContext('2d');
    if (ctx) drawArena(ctx, recording, turn, view, sprites, tanks);
  }, [recording, turn, view, sprites, tanks]);
  return (
    <canvas
      ref={ref}
      width={recording.battlefield.width}
      height={recording.battlefield.height}
      className="block h-auto w-full rounded-lg shadow-[0_1px_0_#0f1e2e,0_12px_30px_-12px_rgba(15,30,46,0.6)]"
      aria-label={`Battle arena, turn ${turn}`}
      role="img"
    />
  );
}
