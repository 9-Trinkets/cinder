import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

const apiProxyTarget = process.env.VITE_API_PROXY_TARGET ?? 'http://localhost:3000'

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    host: '0.0.0.0',
    proxy: {
      '/api': {
        target: apiProxyTarget,
        ws: true,
        xfwd: true,
        configure: (proxy) => {
          proxy.on('error', (err, _req, res) => {
            if (err.message?.includes('ECONNREFUSED')) {
              console.error(
                `\n  ⚠️  Backend server not running at ${apiProxyTarget}\n` +
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
