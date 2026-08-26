import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // `maqueta.html` es la app corriendo en un navegador común, con datos falsos.
  // Existe para poder MIRAR la interfaz sin Windows y sin permisos de captura de
  // pantalla — ver `dev/maqueta.ts`.
  //
  // **Se arma solo con `XTAL_MAQUETA=1`**, y no en el build normal: si no, viaja
  // adentro del instalador una página que reemplaza el backend por datos inventados.
  // No es peligrosa —sin `invoke` de verdad no puede hacer nada— pero es basura en la
  // computadora de otro, y de las que confunden si alguien la encuentra.
  build: {
    rollupOptions: {
      input: process.env.XTAL_MAQUETA
        ? { index: "index.html", maqueta: "maqueta.html" }
        : { index: "index.html" },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
