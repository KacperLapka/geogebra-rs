import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  base: './',
  // SPA fallback: serve `index.html` for the primary `/w/<slug>` view so a
  // direct open reloads the app; the `?s=<slug>` fallback needs no server help.
  appType: 'spa',
  server: {
    port: 5173,
    proxy: {
      '/api': 'http://127.0.0.1:3000'
    }
  }
});
