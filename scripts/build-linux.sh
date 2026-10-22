#!/usr/bin/env bash
# Build with Linux toolchains, including when the host is Windows/WSL2.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."

[[ $(uname -s) == Linux ]] || { echo 'Run this script inside Linux or WSL2.' >&2; exit 1; }
for tool in cargo cargo-audit rustc node npm pkg-config; do
  command -v "$tool" >/dev/null || { echo "Missing tool: $tool" >&2; exit 1; }
done
node -e 'if (process.platform !== "linux") throw new Error("Use Linux Node.js, not Windows Node.js"); const [major, minor] = process.versions.node.split(".").map(Number); if (!(major > 22 || major === 22 && minor >= 12 || major === 20 && minor >= 19)) throw new Error("Vite requires Node 20.19+ or 22.12+")'
case "$(command -v npm)" in /mnt/*) echo 'Install Linux npm; Windows npm cannot build this package.' >&2; exit 1;; esac
pkg-config --exists gtk+-3.0 webkit2gtk-4.1 ayatana-appindicator3-0.1 dbus-1 openssl

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-8}"
host=$(rustc -vV | sed -n 's/^host: //p')
cargo fetch --locked --manifest-path src-tauri/Cargo.toml --target "$host"
npm audit --audit-level=low
cargo audit --file src-tauri/Cargo.lock --deny yanked
npm ci --no-audit --no-fund
npm run check
node scripts/test-security.mjs
node scripts/test-linux-typescript.mjs
node scripts/test-i18n.mjs
node scripts/test-window-drag.mjs
# Notices read Cargo metadata offline; fetch above must precede the frontend build.
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo test --release --locked --manifest-path src-tauri/Cargo.toml --test glib_variant
npm run tauri -- build --bundles deb -- --locked
release_dir="${CARGO_TARGET_DIR:-src-tauri/target}/release"
version=$(node -p 'JSON.parse(require("fs").readFileSync("src-tauri/tauri.conf.json", "utf8")).version')
cp THIRD-PARTY-NOTICES.txt "$release_dir/THIRD-PARTY-NOTICES.txt"
node scripts/release-manifest.mjs write "$release_dir" pinboard "bundle/deb/Pinboard_${version}_amd64.deb" THIRD-PARTY-NOTICES.txt
node scripts/release-manifest.mjs verify "$release_dir" --source
printf '\nLinux package: %s\n' "${CARGO_TARGET_DIR:-src-tauri/target}/release/bundle/deb/"
