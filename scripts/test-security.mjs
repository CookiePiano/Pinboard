// Guard against accidentally restoring global privileges when adding an IPC command.
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import "./test-vendored-glib.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const read = (file) => fs.readFileSync(path.join(root, file), "utf8");
const lib = read("src-tauri/src/lib.rs");
const handler = lib.split("tauri::generate_handler![")[1].split("])")[0];
const registered = [...handler.matchAll(/\w+::(\w+),/g)].map((match) => match[1]);
const manifest = [...read("src-tauri/build.rs").matchAll(/"([a-z][a-z_]+)"/g)].map((match) => match[1]);
assert.deepEqual([...registered].sort(), [...manifest].sort(), "Every IPC command must opt into the ACL");

const caps = fs.readdirSync(path.join(root, "src-tauri/capabilities"))
  .filter((file) => file.endsWith(".json"))
  .map((file) => JSON.parse(read(`src-tauri/capabilities/${file}`)));
const ids = (cap) => cap.permissions.map((p) => typeof p === "string" ? p : p.identifier);
for (const cap of caps) {
  assert.ok(!cap.remote, "Remote pages must never receive native permissions");
  assert.ok(!cap.windows.includes("*"), "Avoid granting permissions to arbitrary new windows");
}
const sensitive = ["save-settings", "gdrive-connect", "gdrive-disconnect", "open-data-dir",
  "export-archive", "import-archive", "provision-mcp", "set-mcp-enabled", "connect-mcp-clients"];
for (const cap of caps) {
  if (ids(cap).some((id) => sensitive.some((name) => id === `allow-${name}`)
    || id.startsWith("autostart:") || id === "dialog:allow-open" || id === "core:window:allow-destroy")) {
    assert.deepEqual(cap.windows, ["settings"], `${cap.identifier} grants a sensitive action outside settings`);
  }
  assert.ok(!ids(cap).includes("opener:default"));
  assert.ok(!ids(cap).includes("opener:allow-open-path"));
}
const granted = new Set(caps.flatMap(ids));
// Tauri onCloseRequested completes an accepted close with destroy(), after our
// settings handler flushes pending design saves. allow-close alone is not enough.
assert.ok(caps.some((cap) => cap.windows.includes("settings")
  && ids(cap).includes("core:window:allow-destroy")), "Settings must be able to finish a saved close request");
for (const command of registered) assert.ok(granted.has(`allow-${command.replaceAll("_", "-")}`), `${command} has no UI grant`);
const opener = caps.flatMap((cap) => cap.permissions).find((p) => p.identifier === "opener:allow-open-url");
assert.deepEqual(opener.allow, [{ url: "https://*" }, { url: "http://*" }]);
const csp = JSON.parse(read("src-tauri/tauri.conf.json")).app.security.csp;
for (const directive of ["object-src 'none'", "frame-src 'none'", "base-uri 'none'", "form-action 'none'"]) {
  assert.ok(csp.includes(directive));
}
assert.ok(!csp.includes("unsafe-eval"));
console.log(`Security contract passed: ${registered.length} IPC commands, settings-only sensitive grants, scoped URLs and CSP.`);
