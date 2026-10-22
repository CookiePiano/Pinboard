import assert from "node:assert/strict";
import { createServer } from "vite";
import { tmpdir } from "node:os";
import { join } from "node:path";

// Load the real stores with the project's TypeScript resolver, without a browser or native IPC.
const server = await createServer({
  configFile: false,
  cacheDir: join(tmpdir(), `pinboard-i18n-check-${process.pid}`),
  optimizeDeps: { noDiscovery: true, include: [] },
  server: { middlewareMode: true, hmr: false, ws: false }, appType: "custom",
});
try {
  const { language, t, messages } = await server.ssrLoadModule("/src/lib/i18n.ts");
  const { fontText } = await server.ssrLoadModule("/src/lib/settings/font-messages.ts");
  const { calendarText } = await server.ssrLoadModule("/src/lib/calendar/messages.ts");
  const { aiText } = await server.ssrLoadModule("/src/lib/mcp/i18n.ts");
  const value = (store) => { let current; store.subscribe(v => { current = v; })(); return current; };
  assert.equal(value(language), "ko");
  const stores = [t, fontText, calendarText, aiText];
  const original = stores.map(value);
  const headings = { ko: "일반", en: "General", zh: "常规", ja: "一般", fr: "Général", es: "General" };
  for (const lang of ["zh", "ja", "fr", "es", "en", "ko"]) {
    language.set(lang);
    assert.equal(value(t).general, headings[lang]);
    stores.forEach((store, i) => {
      const catalog = value(store);
      assert.deepEqual(Object.keys(catalog).sort(), Object.keys(original[i]).sort());
      for (const [key, entry] of Object.entries(catalog)) {
        assert.equal(typeof entry, typeof original[i][key], `${lang}: ${key}`);
        assert.ok(typeof entry === "function" || entry.trim(), `${lang}: empty ${key}`);
        if (lang !== "ko" && lang !== "en") {
          const text = typeof entry === "function" ? entry(2, "test") : entry;
          assert.doesNotMatch(text, /[가-힣]|undefined/, `${lang}: untranslated ${key}`);
        }
      }
    });
    assert.ok(messages[lang].importDone(3, "C:/备份/メモ.zip").includes("C:/备份/メモ.zip"));
    assert.ok(messages[lang].saveError("test-error").includes("test-error"));
    assert.ok(messages[lang].minutesAgo(2).includes("2"));
  }
  assert.equal(value(t).general, "일반");
  console.log("Six-language catalogs, live store switching, default language, and interpolation passed.");
} finally {
  await server.close();
}
