import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
const script = fileURLToPath(new URL("./release-manifest.mjs", import.meta.url));
const root = fs.mkdtempSync(path.join(os.tmpdir(), "pinboard-manifest-"));
const env = { ...process.env };
delete env.PINBOARD_RELEASE_TAG;
const run = (...args) => spawnSync(process.execPath, [script, ...args], { encoding: "utf8", env });
try {
  fs.writeFileSync(path.join(root, "payload.bin"), "tested bytes");
  let result = run("write", root, "payload.bin");
  assert.equal(result.status, 0, result.stderr);
  const receipt = fs.readFileSync(path.join(root, "release-manifest.json"));
  assert.equal(run("verify", root, "--source").status, 0);
  fs.appendFileSync(path.join(root, "payload.bin"), "tampered");
  assert.notEqual(run("verify", root).status, 0);
  fs.writeFileSync(path.join(root, "payload.bin"), "tested bytes");
  for (const mutate of [
    value => value.artifacts.push(value.artifacts[0]),
    value => value.artifacts[0].path = "../outside.bin",
    value => value.artifacts[0].path = "payload.bin:stream",
    value => value.artifacts[0].path = "missing.bin",
    value => value.sourceInputs[0].sha256 = "0".repeat(64),
    value => value.version = "9.9.9",
  ]) {
    const value = JSON.parse(receipt);
    mutate(value);
    fs.writeFileSync(path.join(root, "release-manifest.json"), JSON.stringify(value));
    assert.notEqual(run("verify", root, "--source").status, 0);
  }
  fs.writeFileSync(path.join(root, "release-manifest.json"), receipt);
  assert.equal(run("verify", root, "--source").status, 0);
  console.log("Manifest positive verification and artifact tamper, duplicate, traversal, ADS, missing file, source and version rejection passed.");
} finally {
  assert.ok(path.resolve(root).startsWith(path.resolve(os.tmpdir()) + path.sep + "pinboard-manifest-"));
  fs.rmSync(root, { recursive: true, force: true });
}
