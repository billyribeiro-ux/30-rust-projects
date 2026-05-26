import adapter from '@sveltejs/adapter-node';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  compilerOptions: {
    runes: ({ filename }) => (filename.split(/[/\\]/).includes('node_modules') ? undefined : true)
  },
  kit: {
    adapter: adapter(),
    serviceWorker: { register: true },
    experimental: {
      // Project 10 required lesson: remote functions ($app/server).
      // We use them in src/routes/recipes/[slug]/edit/photos.remote.ts
      // to demonstrate the query / form / command pattern.
      remoteFunctions: true
    }
  }
};
export default config;
