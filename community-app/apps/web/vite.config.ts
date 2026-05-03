import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      "/auth": "http://localhost:8080",
      "/orgs": "http://localhost:8080",
      "/channels": "http://localhost:8080",
      "/messages": "http://localhost:8080",
      "/media": "http://localhost:8080",
      "/public": "http://localhost:8080",
      "/realtime": {
        target: "http://localhost:8081",
        ws: true,
      },
    },
  },
});

