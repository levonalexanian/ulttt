import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  // Relative asset paths, so the build works from any directory.
  base: "./",
  worker: { format: "es" },
  server: { host: true, port: 5173 },
});
