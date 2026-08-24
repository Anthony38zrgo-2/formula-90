import tailwindcss from '@tailwindcss/vite'
import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'

export default defineConfig({
  plugins: [vue(), tailwindcss()],
  server: {
    proxy: {
      '/api': `http://127.0.0.1:${process.env.VEHICLE_STUDIO_API_PORT ?? '8765'}`,
    },
  },
  test: {
    environment: 'jsdom',
  },
})
