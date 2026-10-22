import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { spawn } from "node:child_process";
const [executable, directory] = process.argv.slice(2).map(value => path.resolve(value));
fs.mkdirSync(directory, { recursive: true });
const root = fs.mkdtempSync(path.join(directory, "isolated-"));
const env = { ...process.env, PINBOARD_DATA_DIR: path.join(root, "data"), WEBVIEW2_USER_DATA_FOLDER: path.join(root, "webview") };
const expectedVersion = JSON.parse(fs.readFileSync(new URL("../package.json", import.meta.url), "utf8")).version;
const rpc = spawn(executable, ["--mcp", "--mcp-data-dir", env.PINBOARD_DATA_DIR], { env, windowsHide: true, stdio: ["pipe", "pipe", "pipe"] });
let stdout = "", stderr = "";
rpc.stdout.on("data", chunk => stdout += chunk);
rpc.stderr.on("data", chunk => stderr += chunk);
const completed = new Promise((resolve, reject) => { rpc.on("error", reject); rpc.on("exit", code => code === 0 ? resolve() : reject(new Error(`MCP exited ${code}: ${stderr}`))); });
const timeout = setTimeout(() => rpc.kill(), 20000);
try {
  rpc.stdin.end([
    { jsonrpc: "2.0", id: 1, method: "initialize", params: { protocolVersion: "2025-11-25", capabilities: {}, clientInfo: { name: "release-test", version: "1" } } },
    { jsonrpc: "2.0", method: "notifications/initialized" },
    { jsonrpc: "2.0", id: 2, method: "tools/list" },
    { jsonrpc: "2.0", id: 3, method: "tools/call", params: { name: "list_notes", arguments: {} } },
    { jsonrpc: "2.0", id: 4, method: "tools/call", params: { name: "read_note", arguments: { id: "../../settings" } } },
  ].map(value => JSON.stringify(value)).join("\n") + "\n");
  await completed;
  const replies = stdout.trim().split(/\r?\n/).map(line => JSON.parse(line));
  assert.equal(replies.find(value => value.id === 1)?.result?.serverInfo?.version, expectedVersion);
  assert.equal(replies.find(value => value.id === 2)?.result?.tools?.length, 5);
  const denied = replies.find(value => value.id === 3);
  assert.ok(denied?.error || denied?.result?.isError, "Unprovisioned note access must fail");
  const traversal = replies.find(value => value.id === 4);
  assert.ok(traversal?.error || traversal?.result?.isError, "Traversal must fail");
  assert.ok(!fs.existsSync(path.join(env.PINBOARD_DATA_DIR, "mcp", "settings.json")), "MCP must not provision itself");
  fs.writeFileSync(path.join(root, "result.json"), JSON.stringify({ version: expectedVersion, tools: 5, unprovisionedAccessDenied: true, traversalDenied: true }, null, 2));
  console.log(`Packaged MCP smoke passed for ${expectedVersion}: handshake, tools, consent and traversal.`);
} finally { clearTimeout(timeout); if (rpc.exitCode === null) rpc.kill(); }
