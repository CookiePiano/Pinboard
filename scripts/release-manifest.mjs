// Record exact source/build inputs and artifact bytes. Hashes do not authenticate a publisher.
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";

const source = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const [action, directory, ...args] = process.argv.slice(2);
if (!["write", "verify"].includes(action) || !directory) {
  throw new Error("Usage: release-manifest.mjs write <directory> <artifact...> | verify <directory> [--source]");
}
const root = fs.realpathSync(directory);
const manifestPath = path.join(root, "release-manifest.json");
const skipped = new Set(["node_modules", "target", "dist", "build", "gen", ".git", ".cache"]);
const legacyInputRoots = ["package.json", "package-lock.json", "index.html", "vite.config.ts",
  "svelte.config.js", "tsconfig.json", "src", "src-tauri", "public", "installer", "scripts", "licenses", ".gitattributes"];
const inputRoots = [...legacyInputRoots, ".github", "LICENSE", "SECURITY.md", "README.md", "THIRD-PARTY-NOTICES.txt", "docs/portable-readme.txt"];

function command(executable, args) {
  try { return execFileSync(executable, args, { cwd: source, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim(); }
  catch { return null; }
}
function version() {
  const read = file => fs.readFileSync(path.join(source, file), "utf8");
  const expected = JSON.parse(read("src-tauri/tauri.conf.json")).version;
  const lock = JSON.parse(read("package-lock.json"));
  const versions = [JSON.parse(read("package.json")).version, lock.version, lock.packages[""].version,
    ...["src-tauri/Cargo.toml", "src-tauri/Cargo.lock"].map(file => read(file).match(/name = "pinboard"\s+version = "([^"]+)"/)?.[1])];
  if (!/^\d+\.\d+\.\d+$/.test(expected) || versions.some(value => value !== expected)) throw new Error("Release versions disagree");
  return expected;
}

function safeFile(base, relative) {
  if (typeof relative !== "string" || !relative || relative.includes("\\")
    || relative.split("/").some(part => !part || part === "." || part === ".." || part.includes(":"))
    || path.isAbsolute(relative)) throw new Error("Unsafe manifest path");
  let current = base;
  for (const part of relative.split("/")) {
    current = path.join(current, part);
    if (fs.lstatSync(current).isSymbolicLink()) throw new Error("Manifest paths cannot follow symbolic links");
  }
  if (!fs.statSync(current).isFile()) throw new Error("Manifest entry is not a regular file");
  return current;
}

function record(base, relative) {
  const file = safeFile(base, relative);
  const bytes = fs.readFileSync(file);
  return { path: relative, bytes: bytes.length, sha256: crypto.createHash("sha256").update(bytes).digest("hex") };
}

function sourceFiles(relative) {
  const file = path.join(source, relative);
  if (fs.lstatSync(file).isSymbolicLink()) throw new Error("Source inputs cannot be symbolic links");
  if (fs.statSync(file).isFile()) return [relative];
  return fs.readdirSync(file).sort().filter(name => !skipped.has(name))
    .flatMap(name => sourceFiles(`${relative}/${name}`));
}

function verifyEntries(base, entries) {
  if (!Array.isArray(entries) || !entries.length) throw new Error("Manifest has no entries");
  const seen = new Set();
  for (const entry of entries) {
    if (seen.has(entry.path)) throw new Error("Duplicate manifest path");
    seen.add(entry.path);
    const actual = record(base, entry.path);
    if (actual.bytes !== entry.bytes || actual.sha256 !== entry.sha256) {
      throw new Error(`Release verification failed: ${entry.path}`);
    }
  }
}

if (action === "write") {
  if (!args.length || args.some(file => file === "release-manifest.json")) throw new Error("Specify release artifacts");
  const releaseVersion = version();
  const sourceInputs = inputRoots.flatMap(sourceFiles).sort().map(file => record(source, file));
  const artifacts = args.map(file => record(root, file));
  if (new Set(args).size !== args.length) throw new Error("Duplicate artifact path");
  const sourceSha256 = crypto.createHash("sha256").update(JSON.stringify(sourceInputs)).digest("hex");
  const commit = command("git", ["rev-parse", "HEAD"]);
  const tag = process.env.PINBOARD_RELEASE_TAG || command("git", ["describe", "--tags", "--exact-match", "HEAD"]);
  const dirty = command("git", ["status", "--porcelain", "--untracked-files=normal"]) !== "";
  if (process.env.PINBOARD_RELEASE_TAG) {
    const allowed = tag === `v${releaseVersion}` || (releaseVersion === "0.1.0" && tag === "v0.1.0-r2");
    if (!allowed || !commit || dirty || command("git", ["rev-parse", `refs/tags/${tag}^{commit}`]) !== commit) {
      throw new Error("Official releases require a clean commit matching an approved release tag");
    }
  }
  const receipt = { format: "Pinboard release", schemaVersion: 2, version: releaseVersion,
    createdAt: new Date().toISOString(), buildId: `${releaseVersion}-${sourceSha256.slice(0, 16)}`,
    publisherAuthentication: "Not provided by SHA-256; verify publisher signatures separately.",
    signingPolicy: process.env.PINBOARD_SIGNING_POLICY || "Unspecified",
    sourceGit: { repository: "CookiePiano/Pinboard", commit, tag, dirty },
    build: { node: process.version, rust: command("rustc", ["--version"]), cargo: command("cargo", ["--version"]),
      platform: process.platform, arch: process.arch,
      workflowRun: process.env.GITHUB_REPOSITORY === "CookiePiano/Pinboard" && /^\d+$/.test(process.env.GITHUB_RUN_ID || "")
        ? `https://github.com/CookiePiano/Pinboard/actions/runs/${process.env.GITHUB_RUN_ID}` : null },
    sourceSha256, artifacts, sourceInputs };
  const temporary = `${manifestPath}.${crypto.randomUUID()}.tmp`;
  try {
    fs.writeFileSync(temporary, JSON.stringify(receipt, null, 2) + "\n", { flag: "wx" });
    fs.renameSync(temporary, manifestPath);
  } finally { if (fs.existsSync(temporary)) fs.unlinkSync(temporary); }
  console.log(`Recorded ${artifacts.length} artifacts and ${sourceInputs.length} source inputs: ${receipt.buildId}`);
} else {
  if (args.some(arg => arg !== "--source")) throw new Error("Unknown verification option");
  const receipt = JSON.parse(fs.readFileSync(safeFile(root, "release-manifest.json"), "utf8"));
  if (receipt.format !== "Pinboard release" || ![1, 2].includes(receipt.schemaVersion)) throw new Error("Unsupported release manifest");
  const sourceHash = crypto.createHash("sha256").update(JSON.stringify(receipt.sourceInputs)).digest("hex");
  if (sourceHash !== receipt.sourceSha256 || receipt.buildId !== `${receipt.version}-${sourceHash.slice(0, 16)}`) {
    throw new Error("Invalid source identity");
  }
  verifyEntries(root, receipt.artifacts);
  if (args.includes("--source")) {
    if (receipt.version !== version()) throw new Error("Release version does not match this source");
    verifyEntries(source, receipt.sourceInputs);
    const current = (receipt.schemaVersion === 1 ? legacyInputRoots : inputRoots).flatMap(sourceFiles).sort();
    if (JSON.stringify(current) !== JSON.stringify(receipt.sourceInputs.map(entry => entry.path))) {
      throw new Error("Source input file set has changed");
    }
  }
  console.log(`Verified ${receipt.artifacts.length} artifacts${args.includes("--source") ? " and source inputs" : ""}: ${receipt.buildId}`);
}
