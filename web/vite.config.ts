import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// PLENUM web (P2). Serving model = Vite dev server + proxy (architect
// resolution 4): the API stays untouched on loopback 127.0.0.1:5777 and this
// dev server proxies every /api/* call to it, so the browser talks to ONE
// origin — the session cookie "just works" and the API needs no CORS.
//
// Ports (D.'s call, 2026-07-19 — recorded amendment): the API owns 5777; the
// web page's usual port (5173) was held by another tenant of this machine, so
// D. moved PLENUM's dev server to 5177. strictPort: never silently drift to a
// neighbouring port — if 5177 is taken, fail loudly (constraint 9).
//
// VITE_API_TARGET overrides the proxy target (dev convenience, not a secret).
const apiTarget = process.env.VITE_API_TARGET ?? "http://127.0.0.1:5777";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    host: "127.0.0.1",
    port: 5177,
    strictPort: true,
    proxy: {
      "/api": {
        target: apiTarget,
        changeOrigin: true,
      },
    },
  },
});
