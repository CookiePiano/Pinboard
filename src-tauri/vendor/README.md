# Reviewed dependency backports

## GLib 0.18.5: RUSTSEC-2024-0429

The GTK3/WebKitGTK dependency graph requires GLib 0.18. Pinboard vendors the
published 0.18.5 crate and applies only the upstream two-line correction to
`VariantStrIter::impl_get`: the C out-pointer is declared mutable and passed as
`&mut p`. This preserves the original API and ABI. It is a local backport, not a
new upstream release and not an advisory suppression.

| Provenance | Value |
| --- | --- |
| Published crate | [glib 0.18.5](https://crates.io/api/v1/crates/glib/0.18.5/download) |
| Crate archive SHA-256 | `233daaf6e83ae6a12a52055f568f9d7cf4671dabb78ff9560ab6da230ce00ee5` |
| Original source commit | `42b9caf98e03ded086362d9653ca58fe94dc8658` |
| Merged upstream fix | [gtk-rs-core #1343](https://github.com/gtk-rs/gtk-rs-core/pull/1343), commit `b5a4071e439bef2b5eea76c3aa25e5ae84839e34` |
| 0.18 backport status checked 2026-10-22 KST | [#2009](https://github.com/gtk-rs/gtk-rs-core/pull/2009) closed without merging; [release request #2010](https://github.com/gtk-rs/gtk-rs-core/issues/2010) closed |
| Advisory | [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429.html) |

`glib/` contains the 121 original published files except Cargo's local
`.cargo-ok` download marker. Copyright and license files are unmodified.
`glib-upstream-sha256.json` records their original hashes. Run
`node scripts/test-vendored-glib.mjs` from the repository root: it reverses the
two reviewed lines in memory and verifies every original file, the file set and
the Cargo patch entry. Both security checks and notice generation run this guard.
Do not edit vendored files opportunistically or add generated files there.

The application's Linux integration test exercises string iteration in both
directions, `nth`, `nth_back`, `last`, Unicode and empty strings. Run it with
optimization, because debug compilation can hide the original undefined behavior:

```text
cargo test --release --locked --manifest-path src-tauri/Cargo.toml --test glib_variant
```

`scripts/build-linux.sh` runs that check. Verify `cargo tree -i glib` and Cargo
metadata point to this local copy before packaging. Scanners can omit path
dependencies; fewer warnings alone do not establish that this backport is safe.
Retain the upstream reproduction and patched optimized-test evidence. Replace
this backport with an upstream-compatible fixed release when one is available,
then update the lockfile, notices, integrity guard and Linux tests together.
