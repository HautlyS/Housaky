import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  base: '/Housaky/A2A/',
  server: {
    port: 3333,
    host: '0.0.0.0'
  },
  build: {
    // Output to dist directory within the hub folder
    outDir: 'dist',
    emptyOutDir: true,
    assetsDir: 'assets'
  }
})
