// Run under xvfb-run and dbus-run-session. Tests the actual single-instance path.
import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { spawn, execFileSync } from "node:child_process";
const binary = path.resolve(process.argv[2]);
const output = path.resolve(process.argv[3]);
fs.mkdirSync(output, { recursive: true });
const root = fs.mkdtempSync(path.join(output, "xdg-"));
const env = { ...process.env, XDG_DATA_HOME: path.join(root, "data"), XDG_CONFIG_HOME: path.join(root, "config"), XDG_CACHE_HOME: path.join(root, "cache") };
delete env.PINBOARD_DATA_DIR;
delete env.GDK_BACKEND;
// auto-launch currently writes ~/.config/autostart rather than XDG_CONFIG_HOME.
// Pre-initialize this fixture to avoid changing the real login session.
const dataDir = path.join(root, "data", "dev.pinboard.notes");
fs.mkdirSync(dataDir, { recursive: true });
fs.writeFileSync(path.join(dataDir, "local-state.json"), JSON.stringify({ autostart_initialized: true }));
const log = fs.openSync(path.join(output, "startup.log"), "w");
const wm = spawn("openbox", [], { stdio: ["ignore", log, log] });
const app = spawn(binary, [], { env, stdio: ["ignore", log, log] });
const sleep = ms => new Promise(resolve => setTimeout(resolve, ms));
let second;
async function windows() {
  try {
    return execFileSync("xdotool", ["search", "--onlyvisible", "--pid", String(app.pid)], { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] })
      .trim().split("\n").map(id => ({ id, title: execFileSync("xdotool", ["getwindowname", id], { encoding: "utf8" }).trim() }));
  } catch { return []; }
}
async function waitFor(check) {
  for (let i = 0; i < 100; i++) { const value = await check(); if (value) return value; await sleep(200); }
  throw new Error("Timed out waiting for Linux startup");
}
try {
  await waitFor(async () => (await windows()).some(window => window.title === "Pinboard"));
  second = spawn(binary, ["--settings"], { env, stdio: ["ignore", log, log] });
  await waitFor(() => second.exitCode !== null);
  assert.equal(second.exitCode, 0);
  const settings = await waitFor(async () => (await windows()).find(window => window.title.includes("설정")));
  assert.equal(app.exitCode, null);
  const noteCount = () => fs.readdirSync(path.join(dataDir, "notes")).filter(name => name.endsWith(".json")).length;
  const before = noteCount();
  second = spawn(binary, [], { env, stdio: ["ignore", log, log] });
  await waitFor(() => second.exitCode !== null && noteCount() === before + 1);
  assert.equal(second.exitCode, 0);
  second = spawn(binary, ["--autostart"], { env, stdio: ["ignore", log, log] });
  await waitFor(() => second.exitCode !== null);
  assert.equal(second.exitCode, 0);
  await sleep(500);
  assert.equal(noteCount(), before + 1, "autostart must not create another note");
  const report = { passed: true, binary, checks: ["default backend uses available X11", "second launch opens settings in first process", "normal relaunch creates one note", "autostart relaunch creates no note", "second process exits successfully", "XDG data is isolated and autostart initialization skipped"], settingsTitle: settings.title };
  fs.writeFileSync(path.join(output, "result.json"), JSON.stringify(report, null, 2) + "\n");
  console.log(JSON.stringify(report, null, 2));
} finally {
  second?.kill();
  app.kill();
  wm.kill();
  fs.closeSync(log);
}
