import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'

const base = process.env.GITHUB_PAGES === 'true' ? '/Housaky/' : '/'

export default defineConfig({
  plugins: [vue()],
  base: base,
  resolve: {
    alias: {
      '@': resolve(__dirname, './src'),
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
})
