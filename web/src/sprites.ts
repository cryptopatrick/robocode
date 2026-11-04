// Optional user-supplied robot images. No images are bundled: the viewer
// draws its own robots unless you pick image files whose names contain
// "body", "gun" or "radar". Images should point up (heading 0°).

export type SpritePart = 'body' | 'gun' | 'radar';
export type Sprites = Partial<Record<SpritePart, HTMLImageElement>>;

const PARTS: SpritePart[] = ['body', 'gun', 'radar'];

/** Index of the first file name matching each part, case-insensitively. */
export function matchSpriteFiles(names: string[]): Partial<Record<SpritePart, number>> {
  const found: Partial<Record<SpritePart, number>> = {};
  names.forEach((name, i) => {
    const lower = name.toLowerCase();
    for (const part of PARTS) {
      if (found[part] === undefined && lower.includes(part)) found[part] = i;
    }
  });
  return found;
}

function loadImage(file: File): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error(`Could not read ${file.name} as an image`));
    img.src = URL.createObjectURL(file);
  });
}

/** Loads the matching images from the picked files. */
export async function loadSprites(files: File[]): Promise<Sprites> {
  const matches = matchSpriteFiles(files.map((f) => f.name));
  const sprites: Sprites = {};
  for (const part of PARTS) {
    const i = matches[part];
    if (i !== undefined) sprites[part] = await loadImage(files[i]);
  }
  if (Object.keys(sprites).length === 0) {
    throw new Error('No image names contain "body", "gun" or "radar"');
  }
  return sprites;
}
