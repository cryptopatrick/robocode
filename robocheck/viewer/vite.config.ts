/// <reference types="vitest/config" />
// Builds dist/robocheck-viewer.js, one ES module with React and three inside. The drawing, 3D scene,
// recording and playback code are imported unchanged from ../../web/src; their bare imports (react,
// three) resolve from this folder's node_modules, since web/ may not have its own installed.
import { resolve } from 'node:path';
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

const mod = (name: string) => resolve(import.meta.dirname, 'node_modules', name);

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: { react: mod('react'), 'react-dom': mod('react-dom'), three: mod('three') },
    dedupe: ['react', 'react-dom', 'three'],
  },
  define: { 'process.env.NODE_ENV': JSON.stringify('production') },
  build: {
    lib: { entry: resolve(import.meta.dirname, 'src/viewer.ts'), formats: ['es'], fileName: () => 'robocheck-viewer.js' },
    rollupOptions: { output: { inlineDynamicImports: true } },
    emptyOutDir: true,
    minify: true,
  },
  test: { environment: 'jsdom', include: ['src/**/*.test.ts'] },
});
