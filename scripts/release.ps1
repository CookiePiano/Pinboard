# Build and validate a release in isolated directories. Never replace installed apps.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$OutputDirectory,
    [Parameter(Mandatory)][string]$TargetDirectory,
    [string]$AuditTool = 'cargo-audit',
    [ValidateSet('Signed','UnsignedInitial')][string]$SigningPolicy = 'Signed',
    [string]$ExpectedSignerThumbprint,
    [switch]$RequireSignedArtifacts
)
$ErrorActionPreference = 'Stop'
$source = Split-Path $PSScriptRoot -Parent
$target = [IO.Path]::GetFullPath($TargetDirectory)
$out = [IO.Path]::GetFullPath($OutputDirectory)
foreach ($directory in @($target, $out)) {
    if ($directory -eq [IO.Path]::GetPathRoot($directory) -or $directory -eq $source -or $source.StartsWith($directory + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Release paths must be isolated from the checkout and its parents' }
}
if ($out -eq $target -or $target.StartsWith($out + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Output cannot contain the build directory' }
if ((Test-Path -LiteralPath $out) -and @(Get-ChildItem -LiteralPath $out -Force).Count) { throw 'Output directory must be new or empty; releases are never overwritten' }
$version = (Get-Content -Raw -LiteralPath (Join-Path $source 'src-tauri\tauri.conf.json') | ConvertFrom-Json).version
if ($RequireSignedArtifacts) { $SigningPolicy = 'Signed' }
if ($SigningPolicy -eq 'UnsignedInitial' -and $version -ne '0.1.0') { throw 'The unsigned exception is restricted to the initial 0.1.0 release' }
if ($SigningPolicy -eq 'Signed' -and $ExpectedSignerThumbprint -notmatch '^[0-9a-fA-F]{40}$') { throw 'Signed releases require the approved publisher certificate thumbprint' }
$null = New-Item -ItemType Directory -Path $target -Force
$stage = Join-Path $target ('staging-' + [guid]::NewGuid().ToString('N'))
$null = New-Item -ItemType Directory -Path $stage -Force
$saved = @{}
foreach ($name in @('Path','CARGO_TARGET_DIR','CARGO_ENCODED_RUSTFLAGS','PINBOARD_UI_PLUGIN_DIR','PINBOARD_NSIS_DIR','PINBOARD_SIGNING_POLICY')) { $saved[$name] = [Environment]::GetEnvironmentVariable($name, 'Process') }
Push-Location $source
try {
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
    $env:CARGO_TARGET_DIR = $target
    $env:PINBOARD_UI_PLUGIN_DIR = Join-Path $target 'installer-ui'
    $env:PINBOARD_SIGNING_POLICY = $SigningPolicy
    & (Join-Path $PSScriptRoot 'prepare-tools.ps1') -Directory (Join-Path $target 'tools')
    $localNsis = Join-Path $target '.tauri\NSIS'
    $null = New-Item -ItemType Directory -Path $localNsis -Force
    Copy-Item -Path (Join-Path $env:PINBOARD_NSIS_DIR '*') -Destination $localNsis -Recurse -Force
    $env:PINBOARD_NSIS_DIR = $localNsis
    & (Join-Path $PSScriptRoot 'security-check.ps1') -AuditTool $AuditTool
    $cargoCache = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { "$env:USERPROFILE\.cargo" }
    $existing = if ($saved.CARGO_ENCODED_RUSTFLAGS) { @($saved.CARGO_ENCODED_RUSTFLAGS -split [char]0x1f) } elseif ($env:RUSTFLAGS) { @($env:RUSTFLAGS -split '\s+' | Where-Object { $_ }) } else { @() }
    $env:CARGO_ENCODED_RUSTFLAGS = (@($existing) + @("--remap-path-prefix=$env:USERPROFILE=~", "--remap-path-prefix=$cargoCache=/cargo", "--remap-path-prefix=$source=/pinboard")) -join [char]0x1f
    npm run tauri -- build --ci -- --locked -j 24
    if ($LASTEXITCODE -ne 0) { throw 'Tauri build failed' }
    & (Join-Path $PSScriptRoot 'build-uninstaller.ps1') -OutputDirectory $stage -UiDirectory $env:PINBOARD_UI_PLUGIN_DIR
    $installerName = "Pinboard_${version}_x64-setup.exe"
    Copy-Item -LiteralPath (Join-Path $target "release\bundle\nsis\$installerName") -Destination (Join-Path $stage $installerName)
    Copy-Item -LiteralPath (Join-Path $target 'release\pinboard.exe') -Destination (Join-Path $stage 'Pinboard.exe')
    foreach ($name in @('LICENSE','THIRD-PARTY-NOTICES.txt')) { Copy-Item -LiteralPath (Join-Path $source $name) -Destination $stage }
    Copy-Item -LiteralPath (Join-Path $source 'docs\portable-readme.txt') -Destination (Join-Path $stage 'README.txt')
    & (Join-Path $PSScriptRoot 'verify-artifacts.ps1') -Directory $stage -Version $version -SigningPolicy $SigningPolicy -ExpectedSignerThumbprint $ExpectedSignerThumbprint
    & (Join-Path $PSScriptRoot 'test-bootstrapper.ps1') -WorkDirectory (Join-Path $target 'bootstrapper-test') -UiDirectory $env:PINBOARD_UI_PLUGIN_DIR
    & (Join-Path $PSScriptRoot 'test-installer-paths.ps1') -WorkDirectory (Join-Path $target ('path-test-' + [guid]::NewGuid().ToString('N')))
    & (Join-Path $PSScriptRoot 'test-installer.ps1') -WorkDirectory (Join-Path $target ('installer-test-' + [guid]::NewGuid().ToString('N'))) -RenderedDirectory (Join-Path $target 'release\nsis\x64') -UiDirectory $env:PINBOARD_UI_PLUGIN_DIR
    node (Join-Path $PSScriptRoot 'test-packaged.mjs') (Join-Path $stage 'Pinboard.exe') (Join-Path $target 'app-smoke')
    if ($LASTEXITCODE -ne 0) { throw 'Packaged application smoke failed' }
    $zip = "Pinboard_${version}_windows-x64-portable.zip"
    $portableFiles = @('Pinboard.exe','Pinboard_Uninstall.exe','LICENSE','THIRD-PARTY-NOTICES.txt','README.txt')
    Compress-Archive -LiteralPath ($portableFiles | ForEach-Object { Join-Path $stage $_ }) -DestinationPath (Join-Path $stage $zip) -CompressionLevel Optimal
    syft scan "dir:$source" --base-path $source --source-name Pinboard --source-version $version --exclude '**/node_modules/**' --exclude '**/target/**' --exclude '**/dist/**' --exclude '**/.git/**' --exclude '**/.cache/**' --exclude '**/installer/build/**' -o "spdx-json=$(Join-Path $stage 'sbom.spdx.json')" -q
    if ($LASTEXITCODE -ne 0) { throw 'SBOM generation failed' }
    & (Join-Path $PSScriptRoot 'scan-artifacts.ps1') -Directory $stage -EvidenceDirectory (Join-Path $target 'malware-scan')
    node (Join-Path $PSScriptRoot 'release-manifest.mjs') write $stage @portableFiles $installerName $zip 'sbom.spdx.json'
    if ($LASTEXITCODE -ne 0) { throw 'Manifest generation failed' }
    node (Join-Path $PSScriptRoot 'release-manifest.mjs') verify $stage --source
    if ($LASTEXITCODE -ne 0) { throw 'Manifest verification failed' }
    $assets = @($installerName, $zip, 'release-manifest.json', 'sbom.spdx.json')
    $checksums = $assets | ForEach-Object { (Get-FileHash -LiteralPath (Join-Path $stage $_) -Algorithm SHA256).Hash.ToLowerInvariant() + '  ' + $_ }
    [IO.File]::WriteAllLines((Join-Path $stage 'SHA256SUMS.txt'), $checksums, [Text.UTF8Encoding]::new($false))
    if (!(Test-Path -LiteralPath $out)) { $null = New-Item -ItemType Directory -Path $out }
    Copy-Item -Path (Join-Path $stage '*') -Destination $out
    Write-Output "Validated $version artifacts: $out"
} finally {
    Pop-Location
    foreach ($name in $saved.Keys) { [Environment]::SetEnvironmentVariable($name, $saved[$name], 'Process') }
}
