import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
// Development only: rewrite Host and Origin as one proxy boundary. Production is served same-origin by the backend.
const proxy = {
  target: 'http://127.0.0.1:43180',
  changeOrigin: true,
  headers: { Origin: 'http://127.0.0.1:43180' },
};
export default defineConfig({
  plugins: [vue()],
  server: { host: '127.0.0.1', port: 5173, strictPort: true, proxy: { '/api': proxy, '/health': proxy } },
  build: { target: 'es2022' },
});
