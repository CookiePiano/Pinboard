// Collect the installed, locked dependencies' unmodified license and notice texts.
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import "./test-vendored-glib.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const read = (file) => fs.readFileSync(file, "utf8").replace(/\r\n/g, "\n").trimEnd();
const local = (name) => path.join(root, "licenses", "upstream", `${name}.txt`);
const cargo = process.platform === "win32"
  ? path.join(process.env.USERPROFILE, ".cargo", "bin", "cargo.exe") : "cargo";
const host = execFileSync(cargo, ["-vV"], { encoding: "utf8" }).match(/^host: (.+)$/m)?.[1]?.trim();
if (!host) throw new Error("Could not determine the Cargo host platform.");
const metadata = JSON.parse(execFileSync(cargo, ["metadata", "--manifest-path", "src-tauri/Cargo.toml", "--locked", "--offline", "--format-version", "1", "--filter-platform", host], { cwd: root, maxBuffer: 32 * 1024 * 1024, encoding: "utf8" }));
const licensePattern = /^(licen[sc]e|copying|copyright|notice|ofl)([._-]|$)/i;

function licenseFiles(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    if (!licensePattern.test(entry.name)) return [];
    const filename = path.join(dir, entry.name);
    if (entry.isFile()) return [filename];
    if (entry.isDirectory()) return fs.readdirSync(filename, { recursive: true, withFileTypes: true })
      .filter((child) => child.isFile()).map((child) => path.join(child.parentPath, child.name));
    return [];
  }).sort();
}

const entries = [];
function add(name, version, license, source, files, base) {
  if (!files.length) throw new Error(`Missing license text: ${name}@${version}`);
  entries.push(`\n${"=".repeat(72)}\n${name} ${version}\nLicense: ${license}\nSource: ${source}\n\n`
    + files.map((file) => `--- ${path.relative(base, file).replaceAll("\\", "/")} ---\n${read(file)}\n`).join("\n"));
}

const lock = JSON.parse(read(path.join(root, "package-lock.json")));
for (const [directory, entry] of Object.entries(lock.packages)) {
  // Svelte is declared as a development dependency, but its runtime is bundled.
  if (!directory || (entry.dev && directory !== "node_modules/svelte")) continue;
  const dir = path.join(root, directory);
  if (!fs.existsSync(dir)) continue; // Optional packages for other platforms are not shipped.
  const pkg = JSON.parse(read(path.join(dir, "package.json")));
  let files = licenseFiles(dir);
  if (pkg.name === "pretendard") files = [local(`pretendard-${pkg.version}`)];
  // These packages declare MIT and Rich Harris as author, but ship no license file.
  if (["is-reference", "locate-character"].includes(pkg.name)) files = [local("rich-harris-mit")];
  add(pkg.name, pkg.version, pkg.license, `https://www.npmjs.com/package/${pkg.name}/v/${pkg.version}`, files, files[0]?.startsWith(path.join(root, "licenses")) ? root : dir);
}

for (const pkg of [...metadata.packages].sort((a, b) => `${a.name}@${a.version}`.localeCompare(`${b.name}@${b.version}`))) {
  if (metadata.workspace_members.includes(pkg.id)) continue;
  const vendoredGlib = !pkg.source && pkg.name === "glib" && pkg.version === "0.18.5"
    && path.resolve(pkg.manifest_path) === path.join(root, "src-tauri/vendor/glib/Cargo.toml");
  if (!pkg.source && !vendoredGlib) throw new Error(`Unreviewed local dependency: ${pkg.name}`);
  const dir = path.dirname(pkg.manifest_path);
  let files = licenseFiles(dir);
  if (pkg.license_file) files.push(path.resolve(dir, pkg.license_file));
  if (!files.length) {
    if (pkg.name.startsWith("webview2-com")) files = [local("webview2")];
    if (pkg.name === "alloc-stdlib") files = [local("alloc-stdlib")];
    // These crates omit the repository-wide license in their published tarballs.
    if (pkg.name === "tauri-plugin") {
      const owner = metadata.packages.find((p) => p.name === "tauri");
      files = licenseFiles(path.dirname(owner.manifest_path));
    }
    if (pkg.name === "selectors") {
      const owner = metadata.packages.find((p) => p.name === "cssparser");
      files = licenseFiles(path.dirname(owner.manifest_path));
    }
    // Both crates omit the repository-wide MIT license from the published tarballs.
    if (["dlopen2", "dlopen2_derive"].includes(pkg.name)) files = [local("dlopen2")];
    if (pkg.name === "libappindicator-sys") {
      const owner = metadata.packages.find((p) => p.name === "libappindicator");
      files = licenseFiles(path.dirname(owner.manifest_path));
    }
  }
  // Use stable labels, never publish the developer's local Cargo paths.
  const body = [...new Set(files)].map((file) => `--- ${path.basename(file)} ---\n${read(file)}\n`).join("\n");
  if (!body) throw new Error(`Missing license text: ${pkg.name}@${pkg.version}`);
  const modification = vendoredGlib
    ? "Pinboard modification: backported upstream commit b5a4071e439bef2b5eea76c3aa25e5ae84839e34 (RUSTSEC-2024-0429). VariantStrIter::impl_get uses a mutable out-pointer. See src-tauri/vendor/README.md in the source tree.\n"
    : "";
  entries.push(`\n${"=".repeat(72)}\n${pkg.name} ${pkg.version}\nLicense: ${pkg.license}\nSource: https://crates.io/api/v1/crates/${pkg.name}/${pkg.version}/download\n${modification}\n${body}`);
}

const header = `Pinboard — Third-party notices\n\nThis file preserves copyright, permission, license and notice texts from the\nlocked dependency versions. It includes Rust build dependencies conservatively;\nnot every listed component is linked into every executable. Any local dependency\nmodification is identified in its entry. Multiple licenses are reproduced as provided.\n\nThe corresponding unmodified source code is available at each exact-version\nSource URL below, including components under MPL-2.0. The Pinboard repository\nalso retains its dependency lockfiles and the upstream font license.\n\nRegenerate with: npm run notices\nDistribute this file with portable releases. The executable also writes its\nembedded copy to the app data folder, and installers bundle the file.\n`;
fs.writeFileSync(path.join(root, "THIRD-PARTY-NOTICES.txt"), header + entries.join(""), "utf8");
console.log(`Collected ${entries.length} third-party notice entries.`);
