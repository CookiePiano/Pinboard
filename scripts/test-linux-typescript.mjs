// Ubuntu's Node build may omit native TypeScript stripping. Reuse Vite's resolver.
import { createServer } from "vite";
import { tmpdir } from "node:os";
import { join } from "node:path";

const server = await createServer({
  configFile: false,
  cacheDir: join(tmpdir(), `pinboard-linux-check-${process.pid}`),
  optimizeDeps: { noDiscovery: true, include: [] },
  server: { middlewareMode: true, hmr: false, ws: false },
  appType: "custom",
});
try {
  await server.ssrLoadModule("/scripts/test-links.mjs");
  await server.ssrLoadModule("/scripts/test-formatting.mjs");
  await server.ssrLoadModule("/scripts/test-calendar-dates.mjs");
  await server.ssrLoadModule("/scripts/test-save-queue.mjs");
  await server.ssrLoadModule("/scripts/test-shortcut-capture.mjs");
  await server.ssrLoadModule("/scripts/test-performance.mjs");
} finally {
  await server.close();
}
