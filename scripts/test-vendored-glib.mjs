// Verify the complete vendor copy while allowing exactly the reviewed upstream fix.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const vendor = path.join(root, "src-tauri/vendor/glib");
const files = JSON.parse(fs.readFileSync(path.join(vendor, "../glib-upstream-sha256.json"), "utf8"));
const actual = fs.readdirSync(vendor, { recursive: true, withFileTypes: true })
  .filter(entry => entry.isFile())
  .map(entry => path.relative(vendor, path.join(entry.parentPath, entry.name)).replaceAll("\\", "/"));
assert.deepEqual(actual.sort(), files.map(file => file.path).sort(), "Unexpected vendor files");
for (const file of files) {
  let bytes = fs.readFileSync(path.join(vendor, file.path));
  if (file.path === "src/variant_iter.rs") {
    const fixed = bytes.toString("utf8");
    const declaration = "let mut p: *mut libc::c_char = std::ptr::null_mut();";
    assert.equal(fixed.split(declaration).length, 2);
    assert.equal(fixed.split("                &mut p,").length, 2);
    bytes = Buffer.from(fixed.replace(declaration, "let p: *mut libc::c_char = std::ptr::null_mut();")
      .replace("                &mut p,", "                &p,"));
  }
  assert.equal(createHash("sha256").update(bytes).digest("hex"), file.sha256, file.path);
}
assert.match(fs.readFileSync(path.join(root, "src-tauri/Cargo.toml"), "utf8"),
  /\[patch\.crates-io\]\s+glib = \{ path = "vendor\/glib" \}/);
const packages = fs.readFileSync(path.join(root, "src-tauri/Cargo.lock"), "utf8")
  .split("[[package]]").filter(block => /^\s*name = "glib"\r?$/m.test(block));
assert.equal(packages.length, 1, "Review every resolved GLib version");
assert.match(packages[0], /^version = "0\.18\.5"\r?$/m);
assert.doesNotMatch(packages[0], /^source\s*=/m, "GLib must resolve to the reviewed local copy");
console.log(`Vendored GLib: ${files.length} upstream files verified; only the two-line soundness patch differs.`);
