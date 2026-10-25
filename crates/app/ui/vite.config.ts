import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import solid from 'vite-plugin-solid';

// Tauri loads the dev server in development and `dist/` in release builds.
export default defineConfig({
  plugins: [solid(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    // Oldest engine we support: Safari 16.4's WebKit (what Tailwind v4 needs),
    // which macOS 13+ has (minimumSystemVersion in tauri.conf.json). Windows'
    // WebView2 is a current Chromium.
    target: 'safari16.4',
    outDir: 'dist',
    emptyOutDir: true,
  },
});
