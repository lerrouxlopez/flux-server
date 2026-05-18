import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    globals: true,
  },
  server: {
    port: 5173,
    proxy: {
      "/auth": process.env.VITE_API_TARGET ?? "http://localhost:8080",
      "/orgs": process.env.VITE_API_TARGET ?? "http://localhost:8080",
      "/channels": process.env.VITE_API_TARGET ?? "http://localhost:8080",
      "/messages": process.env.VITE_API_TARGET ?? "http://localhost:8080",
      "/threads": process.env.VITE_API_TARGET ?? "http://localhost:8080",
      "/attachments": process.env.VITE_API_TARGET ?? "http://localhost:8080",
      "/media": process.env.VITE_API_TARGET ?? "http://localhost:8080",
      "/experience": process.env.VITE_API_TARGET ?? "http://localhost:8080",
      "/public": process.env.VITE_API_TARGET ?? "http://localhost:8080",
      "/realtime": {
        target: process.env.VITE_REALTIME_TARGET ?? "http://localhost:8081",
        ws: true,
      },
    },
  },
});
