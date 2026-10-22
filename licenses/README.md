# Third-party notice sources

The About screen does not need a promotional list of dependencies. Required
copyright and license notices are retained in `THIRD-PARTY-NOTICES.txt`, bundled
with the installer, copied beside the portable release, and embedded in the
executable. At startup, the embedded text is written to the app data folder,
which users can open from About. Free distribution does not waive notice terms.

`npm run notices` collects license, copyright and NOTICE files from installed
npm production dependencies (plus the bundled Svelte runtime) and Cargo's locked
host-platform dependency metadata. Rust build dependencies are included conservatively.
Exact-version source archive URLs are listed for every crate, including MPL-2.0
components. This is a notice collection, not a claim that every listed build
dependency is part of the executable or a complete legal audit of future releases.

Linux uses a [reviewed local GLib backport](../src-tauri/vendor/README.md). Its
original MIT license and copyright remain intact, and its entry identifies the
upstream fix commit and local modification. The generator includes this reviewed
path dependency instead of skipping all packages without a registry source, and
rejects unexpected local dependencies. Original-source archive URLs describe the
upstream source; the modification line distinguishes the code actually shipped.

Some published packages omit their repository license file. The retained
supplemental texts and collection rules are:

- Pretendard 1.3.9: `upstream/pretendard-1.3.9.txt`, from
  https://github.com/orioncactus/pretendard/blob/v1.3.9/LICENSE.
- alloc-stdlib 0.3.0: `upstream/alloc-stdlib.txt`, from its Cargo-recorded source
  commit 0a81fd6928ea3b33c8cd484aa4575d50ffb98012, repository root LICENSE:
  https://github.com/dropbox/rust-alloc-no-stdlib/blob/0a81fd6928ea3b33c8cd484aa4575d50ffb98012/LICENSE.
- webview2-com 0.39.1 and related macros/bindings: `upstream/webview2.txt`, from
  https://github.com/wravery/webview2-rs/blob/edc2caf886175ccaebe86078c9cfe1ae2a187328/LICENSE.
- tauri-plugin: use the license files distributed by the corresponding Tauri
  package from the same project, including Tauri copyright notices.
- selectors: use the MPL-2.0 full text distributed by cssparser. The selectors
  source headers identify MPL-2.0; its exact source archive is also linked.
- dlopen2 and dlopen2_derive (Linux): `upstream/dlopen2.txt`, from the
  Cargo-recorded source commit shared by both packages:
  https://github.com/OpenByteDev/dlopen2/blob/cc80e4a0a90d499b677fdf7743699b4b3a43a989/LICENSE.
- libappindicator-sys 0.9.0 (Linux): use LICENSE-APACHE and LICENSE-MIT from
  libappindicator 0.9.0. Both packages record source commit
  eafd1e3682a1247f595410266091e9684021cb6f.
- is-reference 3.0.3 and locate-character 3.0.0: neither the installed tarballs
  nor their upstream repositories contain a separate license text. Their package
  metadata declares MIT and identifies Rich Harris as author. The supplemental
  `upstream/rich-harris-mit.txt` reproduces the standard MIT text with that
  attribution, without inventing a copyright year.

Authoritative license texts: https://opensource.org/license/mit and
https://openfontlicense.org/open-font-license-official-text/.

Recheck notices and source availability whenever dependencies or distribution
targets change. This process does not select a license for Pinboard's own code.
