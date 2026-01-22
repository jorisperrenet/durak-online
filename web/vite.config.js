import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

// https://vite.dev/config/
export default defineConfig({
  plugins: [svelte()],
  // Set base path for GitHub Pages (e.g., '/repo-name/' or '/' for username.github.io)
  base: '/durak-online/',
})
