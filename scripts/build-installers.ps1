# Repackage the released app without rebuilding or replacing the portable executable.
[CmdletBinding()]
param([string]$WorkDirectory,[string]$AppExecutable)
$ErrorActionPreference = 'Stop'
$source = Split-Path $PSScriptRoot -Parent
if (!$WorkDirectory) { $WorkDirectory = Join-Path 'C:\build\PinboardInstaller' ('build-' + [guid]::NewGuid().ToString('N')) }
if (!$AppExecutable) { $AppExecutable = Join-Path (Split-Path $source -Parent) 'Pinboard.exe' }
$app = (Resolve-Path -LiteralPath $AppExecutable).Path
$config = Get-Content -Raw -LiteralPath (Join-Path $source 'src-tauri\tauri.conf.json') | ConvertFrom-Json
if ((Get-Item -LiteralPath $app).VersionInfo.ProductVersion -ne $config.version) { throw 'The app payload version must match tauri.conf.json.' }
$null = New-Item -ItemType Directory -Path $WorkDirectory -Force
$work = (Resolve-Path -LiteralPath $WorkDirectory).Path
$target = Join-Path $work 'target'
$output = Join-Path $work 'artifacts'
$ui = Join-Path $work 'ui'
$null = New-Item -ItemType Directory -Path "$target\release",$output -Force
$payload = Join-Path $target 'release\pinboard.exe'
Copy-Item -LiteralPath $app -Destination $payload -Force
$savedTarget = $env:CARGO_TARGET_DIR
$savedPlugin = $env:PINBOARD_UI_PLUGIN_DIR
try {
    $env:CARGO_TARGET_DIR = $target
    $env:PINBOARD_UI_PLUGIN_DIR = $ui
    Push-Location $source
    try {
        npm run tauri -- bundle --bundles nsis --ci --no-binary-patching
        if ($LASTEXITCODE -ne 0) { throw "NSIS bundling failed ($LASTEXITCODE)." }
    } finally { Pop-Location }
    & (Join-Path $PSScriptRoot 'build-uninstaller.ps1') -OutputDirectory $output -UiDirectory $ui
} finally {
    $env:CARGO_TARGET_DIR = $savedTarget
    $env:PINBOARD_UI_PLUGIN_DIR = $savedPlugin
}
$name = "Pinboard_$($config.version)_x64-setup.exe"
Copy-Item -LiteralPath (Join-Path $target "release\bundle\nsis\$name") -Destination (Join-Path $output $name) -Force
if ((Get-FileHash -LiteralPath $app).Hash -ne (Get-FileHash -LiteralPath $payload).Hash) { throw 'The packaging step modified the app payload.' }
Get-ChildItem -LiteralPath $output -File | Get-FileHash | Select-Object Path,Hash | ConvertTo-Json -Compress
