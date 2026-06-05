import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

function adminSlashRedirect() {
  return {
    name: 'admin-slash-redirect',
    configureServer(server) {
      server.middlewares.use((req, res, next) => {
        if (req.url === '/admin' || req.url?.startsWith('/admin?')) {
          const query = req.url.slice('/admin'.length);
          res.statusCode = 302;
          res.setHeader('Location', `/admin/${query}`);
          res.end();
          return;
        }
        next();
      });
    },
  };
}

export default defineConfig({
  base: '/admin/',
  plugins: [adminSlashRedirect(), react()],
  server: {
    host: '0.0.0.0',
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:3000',
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: '../public/admin',
    emptyOutDir: true,
  },
});
