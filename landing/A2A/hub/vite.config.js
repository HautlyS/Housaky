import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import path from 'path'

export default defineConfig({
  plugins: [vue()],
  base: '/Housaky/A2A/',
  server: {
    port: 3333,
    host: '0.0.0.0'
  },
  build: {
    // Output directly to docs/A2A for GitHub Pages
    outDir: path.resolve(__dirname, '../../../docs/A2A'),
    emptyOutDir: true,
    assetsDir: 'assets'
  }
})
