import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { svelteTesting } from '@testing-library/svelte/vite';
import tailwindcss from '@tailwindcss/vite';
import { resolve } from 'path';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [
    paraglideVitePlugin({
      project: './project.inlang',
      outdir: './src/lib/paraglide',
      strategy: ['globalVariable', 'baseLocale'],
    }),
    svelte(),
    svelteTesting(),
    tailwindcss(),
  ],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    rolldownOptions: {
      checks: {
        pluginTimings: false,
      },
    },
  },
  resolve: {
    alias: {
      $lib: resolve(__dirname, 'src/lib'),
    },
  },
  test: {
    setupFiles: ['./src/test-setup.ts'],
    exclude: ['**/node_modules/**', '**/dist/**', 'tests/e2e/**'],
    // Theme transition suites exercise a document-global singleton; separate
    // test files must not mutate that shared DOM state concurrently.
    fileParallelism: false,
  },
});
