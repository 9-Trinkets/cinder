import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    proxy: {
      '/api': {
        target: 'http://localhost:3000',
        ws: true,
        configure: (proxy) => {
          proxy.on('error', (err, _req, res) => {
            if (err.message?.includes('ECONNREFUSED')) {
              console.error(
                '\n  ⚠️  Backend server not running at localhost:3000\n' +
                '     Run: cargo run -p cinder-srv\n'
              )
            }
            if (typeof res === 'object' && 'writeHead' in res) {
              ;(res as import('http').ServerResponse).writeHead(502, { 'Content-Type': 'text/plain' })
              ;(res as import('http').ServerResponse).end('Backend unreachable')
            }
          })
        },
      },
    },
  },
})
