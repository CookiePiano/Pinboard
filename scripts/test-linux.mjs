// Run under `xvfb-run -a dbus-run-session -- node scripts/test-linux.mjs <binary> <output>`.
// Requires tauri-driver on PATH, WebKitWebDriver, Xvfb and Openbox.
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { spawn } from "node:child_process";

const binary = path.resolve(process.argv[2] || "src-tauri/target/debug/pinboard");
const output = path.resolve(process.argv[3] || "linux-test-output");
fs.mkdirSync(output, { recursive: true });
const data = fs.mkdtempSync(path.join(output, "data-"));
const port = process.env.PINBOARD_TEST_PORT || "4466";
const backend = process.env.PINBOARD_TEST_BACKEND || "x11";
const log = fs.openSync(path.join(output, "webdriver.log"), "w");
const wm = backend === "x11" ? spawn("openbox", [], { stdio: ["ignore", log, log] }) : undefined;
const driverEnv = { ...process.env, PINBOARD_DATA_DIR: data, GDK_BACKEND: backend };
if (backend === "wayland") delete driverEnv.DISPLAY;
const driver = spawn("tauri-driver", ["--port", port, "--native-port", String(Number(port) + 1)], {
  stdio: ["ignore", log, log],
  env: driverEnv,
});
driver.on("error", error => { console.error(error); process.exitCode = 1; });
let session;
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
async function request(route, body, method = body === undefined ? "GET" : "POST") {
  const response = await fetch(`http://127.0.0.1:${port}${route}`, {
    method, headers: { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(45000),
  });
  const result = await response.json();
  if (!response.ok || result.value?.error) throw new Error(JSON.stringify(result));
  return result.value;
}
const api = (route, body, method) => request(`/session/${session}${route}`, body, method);
async function waitFor(check, description) {
  const deadline = Date.now() + 30000;
  while (Date.now() < deadline) {
    try { const value = await check(); if (value) return value; } catch {}
    await sleep(200);
  }
  throw new Error(`Timed out: ${description}`);
}
async function element(selector) {
  const result = await api("/element", { using: "css selector", value: selector });
  return result["element-6066-11e4-a52e-4f735466cecf"];
}
const click = async selector => api(`/element/${await element(selector)}/click`, {});
const execute = (script, args = []) => api("/execute/sync", { script, args });
async function selectWindow(selector) {
  return waitFor(async () => {
    for (const handle of await api("/window/handles")) {
      await api("/window", { handle });
      if (await execute("return !!document.querySelector(arguments[0])", [selector])) return handle;
    }
  }, `window containing ${selector}`);
}
async function screenshot(name) {
  fs.writeFileSync(path.join(output, `${name}.png`), Buffer.from(await api("/screenshot"), "base64"));
}
function notes() {
  return fs.readdirSync(path.join(data, "notes")).filter(name => name.endsWith(".json"))
    .map(name => JSON.parse(fs.readFileSync(path.join(data, "notes", name), "utf8")));
}
try {
  await waitFor(() => request("/status"), "WebDriver startup");
  const capabilities = { "tauri:options": { application: binary } };
  session = (await request("/session", { capabilities: { alwaysMatch: capabilities } })).sessionId;
  await selectWindow(".cm-content");
  assert.match(await execute("return document.body.innerText"), /Pinboard|메모/);
  await screenshot("welcome");
  const initialHandles = new Set(await api("/window/handles"));
  await click('button[aria-label="새 메모"]');
  await waitFor(() => notes().length === 2, "new note persisted");
  await waitFor(async () => {
    for (const handle of await api("/window/handles")) {
      if (initialHandles.has(handle)) continue;
      await api("/window", { handle });
      if (await execute('return !!document.querySelector(".cm-content")')) return true;
    }
  }, "new note editor");
  const content = "Ubuntu 26.04 Rust Linux 검증\n한글 저장 및 재실행 테스트";
  const editor = await element(".cm-content");
  await api(`/element/${editor}/value`, { text: content.replaceAll("\n", "\uE007") });
  await waitFor(() => notes().some(note => note.body === content), "editor text saved by Rust");
  await screenshot("edited-note");
  const savedNote = notes().find(note => note.body === content);
  await api("/window/rect", { width: 420, height: 600 });
  await click('button[aria-label="색과 투명도"]');
  await waitFor(async () => await execute("return document.querySelectorAll('#note-font-family option').length > 2 && !document.querySelector('#note-font-family').disabled"), "per-note font list");
  await click(".size-controls button:last-child");
  await waitFor(() => notes().find(note => note.id === savedNote.id)?.font?.size > 14.5, "per-note font increment");
  await execute("const input = document.querySelector('#note-font-size'); input.value = '18'; input.dispatchEvent(new Event('change', { bubbles: true }));");
  await waitFor(() => notes().find(note => note.id === savedNote.id)?.font?.size === 24, "per-note 18pt font saved as 24px");
  await screenshot("note-font");
  await click('button[aria-label="색과 투명도"]');
  await click('button[aria-label$="일반"]');
  await click('[role="radiogroup"] [role="radio"]:nth-child(3)');
  await waitFor(() => JSON.parse(fs.readFileSync(path.join(data, "local-state.json"), "utf8")).windows[savedNote.id].pin === "desktop", "desktop pin persisted");
  await click('button[aria-label="설정 열기"]');
  await selectWindow(".shell nav.side");
  await waitFor(async () => /일반|디자인/.test(await execute("return document.body.innerText")), "settings visible");
  await screenshot("settings");
  for (const lang of ["en", "ja", "zh", "fr", "es", "ko"]) {
    await click(`select.note-select option[value="${lang}"]`);
    await waitFor(async () => (await execute("return document.documentElement.lang")) === lang && JSON.parse(fs.readFileSync(path.join(data, "settings.json"), "utf8")).language === lang, `live language ${lang}`);
    await screenshot(`settings-${lang}`);
  }
  await click("nav.side li:nth-child(2) button");
  await waitFor(async () => await execute("return document.querySelectorAll('#font-family option').length > 2 && !document.querySelector('#font-family').disabled"), "Linux font list");
  await screenshot("fonts");
  assert.equal(fs.existsSync(path.join(data, "mcp")), false, "MCP stays unprovisioned");
  await api("", undefined, "DELETE");
  session = undefined;
  session = (await request("/session", { capabilities: { alwaysMatch: capabilities } })).sessionId;
  await waitFor(async () => {
    for (const handle of await api("/window/handles")) {
      await api("/window", { handle });
      if (await execute("return document.querySelector('.cm-content')?.innerText.includes(arguments[0])", ["Ubuntu 26.04 Rust Linux 검증"])) return true;
    }
  }, "saved note restored after restarting the Linux process");
  await screenshot("restored-note");
  const report = { passed: true, binary, backend, data, checks: ["WebKitGTK renders Korean UI", "create note", "edit Korean text and newline", "Rust persistence", "per-note font size persists", "desktop pin persisted", "settings window", "all six live languages", "fontconfig enumeration", "MCP remains opt-in", "restart restores note"] };
  fs.writeFileSync(path.join(output, "result.json"), JSON.stringify(report, null, 2) + "\n");
  console.log(JSON.stringify(report, null, 2));
} catch (error) {
  if (session) {
    const states = [];
    for (const handle of await api("/window/handles").catch(() => [])) {
      try {
        await api("/window", { handle });
        states.push({ handle, state: await execute("return {url: location.href, title: document.title, text: document.body?.innerText, html: document.documentElement.outerHTML}") });
        await screenshot(`failure-${states.length}`);
      } catch (captureError) { states.push({ handle, error: String(captureError) }); }
    }
    fs.writeFileSync(path.join(output, "failure-state.json"), JSON.stringify({ error: String(error), states }, null, 2) + "\n");
  }
  throw error;
} finally {
  if (session) await api("", undefined, "DELETE").catch(() => {});
  driver.kill();
  wm?.kill();
  fs.closeSync(log);
}
