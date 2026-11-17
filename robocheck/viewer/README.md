# robocheck-viewer

The battle replay on the RoboCheck page (cryptopatrick.com, `/robocheck/`): robocode `web/`'s 2D drawing
(`drawArena`), 3D scene (`ArenaScene3D`), recording parser and playback reducer, imported unchanged from
`../../web/src` and wrapped in a small API a plain page script can call. It only replays recordings; it never
runs a robot.

```js
import { mountViewer, loadRecording } from './robocheck-viewer.js';
const v = mountViewer(document.querySelector('#arena'));
v.load(await loadRecording('runs/s1/recording-urban.json.gz'), { rename: { Candidate: 'RoboCheck ✕' } });
v.setMode('3d');          // falls back to 2D, with a note, when WebGL is off
v.play(); v.pause(); v.step(); v.seek(40); v.reset();
v.onTurn((t) => …); v.mark(13, 'Turn 13: the robot breaks S2');
```

`npm ci && npm test && npm run build` writes `dist/robocheck-viewer.js` (one ES module, React and three inside,
about 200 kB gzipped). The site picks it up with `scripts/robocheck_sync.py`, which builds it from robocode's
`master` with `git archive`, so nothing here needs to be checked out.

Porting into robocode: nothing in `web/` was changed for this. If `web/` later wants an embeddable replay, this
file can move to `web/src/embed.ts` with a second Vite entry, and the aliases in `vite.config.ts` go away.
