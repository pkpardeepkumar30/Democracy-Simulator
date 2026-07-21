import { defineConfig } from 'vite';

export default defineConfig({
  build: {
    emptyOutDir: true,
    lib: {
      entry: 'city-src/main.ts',
      name: 'CivicCityBundle',
      formats: ['iife'],
      fileName: () => 'city.bundle.js',
    },
    outDir: 'dist',
    sourcemap: false,
    minify: 'esbuild',
  },
});
