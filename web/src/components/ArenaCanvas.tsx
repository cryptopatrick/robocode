import { useEffect, useRef } from 'react';
import { drawArena, type ViewOptions } from '../draw';
import type { Recording } from '../recording';
import type { Sprites } from '../sprites';

interface Props {
  recording: Recording;
  turn: number;
  view: ViewOptions;
  sprites: Sprites;
}

export default function ArenaCanvas({ recording, turn, view, sprites }: Props) {
  const ref = useRef<HTMLCanvasElement>(null);
  useEffect(() => {
    const ctx = ref.current?.getContext('2d');
    if (ctx) drawArena(ctx, recording, turn, view, sprites);
  }, [recording, turn, view, sprites]);
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
