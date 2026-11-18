// Loads the ten tank PNGs. A tank whose images fail to load is left out of the result.
import { TANK_IDS, type TankId } from './defs';
import leopard2a8Hull from '../assets/tanks/leopard2a8-hull.png';
import leopard2a8Turret from '../assets/tanks/leopard2a8-turret.png';
import abramsHull from '../assets/tanks/abrams-hull.png';
import abramsTurret from '../assets/tanks/abrams-turret.png';
import k2Hull from '../assets/tanks/k2-hull.png';
import k2Turret from '../assets/tanks/k2-turret.png';
import leclercHull from '../assets/tanks/leclerc-hull.png';
import leclercTurret from '../assets/tanks/leclerc-turret.png';
import merkavaHull from '../assets/tanks/merkava-hull.png';
import merkavaTurret from '../assets/tanks/merkava-turret.png';

export type TankImages = Partial<Record<TankId, { hull: HTMLImageElement; turret: HTMLImageElement }>>;

const URLS: Record<TankId, { hull: string; turret: string }> = {
  leopard2a8: { hull: leopard2a8Hull, turret: leopard2a8Turret },
  abrams: { hull: abramsHull, turret: abramsTurret },
  k2: { hull: k2Hull, turret: k2Turret },
  leclerc: { hull: leclercHull, turret: leclercTurret },
  merkava: { hull: merkavaHull, turret: merkavaTurret },
};

function load(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.onload = () => resolve(img);
    img.onerror = () => reject(new Error(`failed to load ${url}`));
    img.src = url;
  });
}

/** Resolves once every image has settled; tanks with a failed image are omitted. */
export async function loadTankImages(): Promise<TankImages> {
  const out: TankImages = {};
  await Promise.all(
    TANK_IDS.map(async (id) => {
      try {
        const [hull, turret] = await Promise.all([load(URLS[id].hull), load(URLS[id].turret)]);
        out[id] = { hull, turret };
      } catch {
        // Leave this tank out; the plain tank is drawn instead.
      }
    }),
  );
  return out;
}
