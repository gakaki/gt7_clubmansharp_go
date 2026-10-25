import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import { voidPlugin } from 'void'
import { fileURLToPath, URL } from 'node:url'

export default defineConfig({
  plugins: [
    voidPlugin(),
    react({ compiler: true }),
    tailwindcss(),
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  base: '/',
})
