import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  server: {
    port: 5173,
    proxy: {
      "/auth": process.env.VITE_API_TARGET ?? "http://localhost:8080",
      "/orgs": process.env.VITE_API_TARGET ?? "http://localhost:8080",
      "/channels": process.env.VITE_API_TARGET ?? "http://localhost:8080",
      "/messages": process.env.VITE_API_TARGET ?? "http://localhost:8080",
      "/media": process.env.VITE_API_TARGET ?? "http://localhost:8080",
      "/public": process.env.VITE_API_TARGET ?? "http://localhost:8080",
      "/realtime": {
        target: process.env.VITE_REALTIME_TARGET ?? "http://localhost:8081",
        ws: true,
      },
    },
  },
});
