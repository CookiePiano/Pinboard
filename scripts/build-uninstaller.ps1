# Build the standalone removal wizard with the same version as the app.
param(
    [string]$OutputDirectory,
    [string]$ProductName = 'Pinboard',
    [string]$UiDirectory
)
$ErrorActionPreference = 'Stop'
$source = Split-Path $PSScriptRoot -Parent
$buildUi = !$UiDirectory
if (!$UiDirectory) { $UiDirectory = Join-Path $source 'installer\build' }
if ($buildUi -or !(Test-Path -LiteralPath (Join-Path $UiDirectory 'PinboardUI.dll'))) {
    & (Join-Path $PSScriptRoot 'build-installer-ui.ps1') -OutputDirectory $UiDirectory
}
$config = Get-Content -Raw -LiteralPath (Join-Path $source 'src-tauri\tauri.conf.json') | ConvertFrom-Json
$compiler = if ($env:PINBOARD_NSIS_DIR) { Join-Path $env:PINBOARD_NSIS_DIR 'makensis.exe' } else { Join-Path $env:LOCALAPPDATA 'tauri\NSIS\makensis.exe' }
if (!(Test-Path -LiteralPath $compiler -PathType Leaf)) {
    throw 'NSIS is not available in the Tauri cache. Build the Tauri NSIS bundle first.'
}
if (!$OutputDirectory) { $OutputDirectory = Join-Path $source 'installer\build' }
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$output = Join-Path (Resolve-Path -LiteralPath $OutputDirectory).Path 'Pinboard_Uninstall.exe'
$previousPluginDirectory = $env:PINBOARD_UI_PLUGIN_DIR
try {
    $env:PINBOARD_UI_PLUGIN_DIR = (Resolve-Path -LiteralPath $UiDirectory).Path
    & $compiler '/V3' '/INPUTCHARSET' 'UTF8' "/DAPP_VERSION=$($config.version)" "/DPB_PRODUCT_NAME=$ProductName" "/DOUTPUT_FILE=$output" (Join-Path $source 'installer\uninstall-launcher.nsi')
    if ($LASTEXITCODE -ne 0) { throw "Uninstaller build failed ($LASTEXITCODE)" }
} finally { $env:PINBOARD_UI_PLUGIN_DIR = $previousPluginDirectory }
if (!(Test-Path -LiteralPath $output -PathType Leaf)) { throw 'Uninstaller artifact is missing.' }
Get-Item -LiteralPath $output | Select-Object FullName, Length | ConvertTo-Json -Compress
