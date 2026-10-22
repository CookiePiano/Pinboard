# Verify an official download

Use only the release page under https://github.com/CookiePiano/Pinboard/releases .
MIT permits forks and modified redistributions; those are not official builds.

Download the installer or portable ZIP together with SHA256SUMS.txt and
release-manifest.json. Compare its SHA-256 against the entry in SHA256SUMS.txt
before running it. A checksum obtained from an untrusted mirror is insufficient.

With the official GitHub CLI installed, check both the published release and
the build provenance. Replace the filename with the actual downloaded asset:

```powershell
gh release verify-asset v0.1.0 .\Pinboard_0.1.0_x64-setup.exe --repo CookiePiano/Pinboard
gh attestation verify .\Pinboard_0.1.0_x64-setup.exe --repo CookiePiano/Pinboard --signer-workflow CookiePiano/Pinboard/.github/workflows/release.yml
Get-FileHash .\Pinboard_0.1.0_x64-setup.exe -Algorithm SHA256
```

The manifest must identify the v0.1.0 tag and its exact source commit. Check the
workflow run linked by that manifest and the successful security/build jobs.
For a full local receipt check, download all five assets, extract the portable ZIP
into that same directory, and use `scripts/release-manifest.mjs verify <directory>`
from the matching source checkout. Add `--source` to compare the source inputs too.

The 0.1.0 application and installer are not Authenticode-signed. Windows may warn
about an unknown publisher. Do not turn off antivirus, SmartScreen or reputation
protection. A future signed version will document the approved signer separately.
Neither a signature nor an antivirus scan guarantees that software is harmless.

If verification fails, do not execute the file. Use the repository's private
vulnerability reporting channel to share the version, filename and observed
verification failure without exposing private notes or credentials.
