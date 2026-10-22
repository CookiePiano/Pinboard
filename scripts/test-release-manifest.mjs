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

  // Exercise official tag validation in a disposable repository, never this checkout.
  const fixture = path.join(root, "source");
  fs.mkdirSync(fixture);
  for (const directory of ["src", "src-tauri", "public", "installer", "scripts", "licenses", ".github", "docs"]) {
    fs.mkdirSync(path.join(fixture, directory));
    fs.writeFileSync(path.join(fixture, directory, "fixture.txt"), "synthetic input\n");
  }
  for (const file of ["index.html", "vite.config.ts", "svelte.config.js", "tsconfig.json", ".gitattributes",
    "LICENSE", "SECURITY.md", "README.md", "THIRD-PARTY-NOTICES.txt", "docs/portable-readme.txt"]) {
    fs.writeFileSync(path.join(fixture, file), "synthetic input\n");
  }
  fs.copyFileSync(script, path.join(fixture, "scripts/release-manifest.mjs"));
  const setVersion = version => {
    fs.writeFileSync(path.join(fixture, "package.json"), JSON.stringify({ version }));
    fs.writeFileSync(path.join(fixture, "package-lock.json"), JSON.stringify({ version, packages: { "": { version } } }));
    fs.writeFileSync(path.join(fixture, "src-tauri/tauri.conf.json"), JSON.stringify({ version }));
    for (const name of ["Cargo.toml", "Cargo.lock"]) fs.writeFileSync(path.join(fixture, "src-tauri", name), `name = "pinboard"\nversion = "${version}"\n`);
  };
  const git = (...args) => {
    const result = spawnSync("git", ["-c", "user.name=Release Test", "-c", "user.email=release-test@example.invalid", ...args], { cwd: fixture, encoding: "utf8" });
    assert.equal(result.status, 0, result.stderr);
  };
  const official = tag => spawnSync(process.execPath, [path.join(fixture, "scripts/release-manifest.mjs"), "write", root, "payload.bin"],
    { encoding: "utf8", env: { ...env, PINBOARD_RELEASE_TAG: tag } });
  setVersion("0.1.0");
  git("init"); git("add", "."); git("commit", "-m", "Synthetic release inputs");
  assert.notEqual(official("v0.1.0-r2").status, 0, "A missing tag must fail");
  git("tag", "v0.1.0"); git("tag", "-a", "v0.1.0-r2", "-m", "Correction");
  for (const tag of ["v0.1.0", "v0.1.0-r2"]) {
    const result = official(tag);
    assert.equal(result.status, 0, result.stderr);
    assert.equal(JSON.parse(fs.readFileSync(path.join(root, "release-manifest.json"))).sourceGit.tag, tag);
  }
  git("tag", "v0.1.0-r3");
  assert.notEqual(official("v0.1.0-r3").status, 0, "Unapproved revision must fail");
  fs.appendFileSync(path.join(fixture, "README.md"), "changed\n");
  assert.notEqual(official("v0.1.0-r2").status, 0, "Dirty source must fail");
  git("add", "."); git("commit", "-m", "Different source");
  assert.notEqual(official("v0.1.0-r2").status, 0, "Tag pointing to another commit must fail");
  setVersion("0.1.1"); git("add", "."); git("commit", "-m", "Different app version");
  git("tag", "-f", "v0.1.0-r2");
  assert.notEqual(official("v0.1.0-r2").status, 0, "Correction exception must not apply to another app version");
  console.log("Official tags: initial and r2 accepted; missing, unapproved, dirty, mismatched commit and different version rejected.");
} finally {
  assert.ok(path.resolve(root).startsWith(path.resolve(os.tmpdir()) + path.sep + "pinboard-manifest-"));
  fs.rmSync(root, { recursive: true, force: true });
}
