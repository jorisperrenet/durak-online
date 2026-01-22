import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

// https://vite.dev/config/
export default defineConfig(({ mode }) => ({
  plugins: [svelte()],
  // Set base path for GitHub Pages only in production
  base: mode === 'production' ? '/durak-online/' : '/',
}))
