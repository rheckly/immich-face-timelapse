import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// The login page is public/login.html; the server answers /login with it
const loginPage = {
  name: 'login-page',
  configureServer(server) {
    server.middlewares.use((req, res, next) => {
      if (req.url === '/login' || req.url.startsWith('/login?')) {
        req.url = '/login.html' + req.url.slice('/login'.length);
      }
      next();
    });
  },
};

export default defineConfig({
  plugins: [svelte(), loginPage],
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://localhost:5000',
        changeOrigin: true,
        ws: true, // Enable WebSocket proxying
      },
      // Tab icon of the login page (served by the backend)
      '/favicon.svg': 'http://localhost:5000',
    },
  },
  build: {
    outDir: 'dist',
    emptyOutDir: true,
  },
});
