# Compile the native NSIS presentation and signature-verification plugins with MSVC.
[CmdletBinding()]
param([string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
$source = Split-Path $PSScriptRoot -Parent
if (!$OutputDirectory) { $OutputDirectory = if ($env:PINBOARD_UI_PLUGIN_DIR) { $env:PINBOARD_UI_PLUGIN_DIR } else { Join-Path $source 'installer\build' } }
$null = New-Item -ItemType Directory -Path $OutputDirectory -Force
$output = (Resolve-Path -LiteralPath $OutputDirectory).Path
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (!(Test-Path -LiteralPath $vswhere)) { throw 'MSVC discovery tool vswhere.exe is missing.' }
$vs = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (!$vs) { throw 'The MSVC x86/x64 C++ build tools are required.' }
$nsis = $env:PINBOARD_NSIS_DIR
if (!$nsis) {
    & (Join-Path $PSScriptRoot 'prepare-tools.ps1') -Tools nsis -Directory (Join-Path $output 'tools')
    $nsis = $env:PINBOARD_NSIS_DIR
}
$sdk = Join-Path $nsis 'Examples\Plugin\nsis'
if (!(Test-Path -LiteralPath (Join-Path $sdk 'pluginapi.h'))) { throw 'NSIS plugin SDK is missing from the Tauri cache.' }
$saved = @{}
foreach ($name in @('Path','INCLUDE','LIB','LIBPATH')) { $saved[$name] = [Environment]::GetEnvironmentVariable($name, 'Process') }
try {
    $devshell = Join-Path $vs 'Common7\Tools\Microsoft.VisualStudio.DevShell.dll'
    Import-Module $devshell
    Enter-VsDevShell -VsInstallPath $vs -SkipAutomaticLocation -DevCmdArguments '-arch=x86 -host_arch=x64' | Out-Null
    Push-Location $output
    try {
        & cl.exe /nologo /std:c++17 /EHsc /W3 /O2 /MT /LD /utf-8 /DUNICODE /D_UNICODE "/I$sdk" "/Fo$output\PinboardUI.obj" (Join-Path $source 'installer\native\PinboardUI.cpp') /link "/OUT:$output\PinboardUI.dll" user32.lib gdi32.lib comctl32.lib dwmapi.lib uxtheme.lib
        if ($LASTEXITCODE -ne 0) { throw "PinboardUI compilation failed ($LASTEXITCODE)." }
        & cl.exe /nologo /std:c++17 /EHsc /W4 /O2 /MT /LD /utf-8 /DUNICODE /D_UNICODE /guard:cf "/I$sdk" "/Fo$output\PinboardVerify.obj" (Join-Path $source 'installer\native\PinboardVerify.cpp') /link "/OUT:$output\PinboardVerify.dll" /DYNAMICBASE /NXCOMPAT /GUARD:CF wintrust.lib crypt32.lib
        if ($LASTEXITCODE -ne 0) { throw "PinboardVerify compilation failed ($LASTEXITCODE)." }
    } finally { Pop-Location }
} finally {
    foreach ($name in $saved.Keys) { [Environment]::SetEnvironmentVariable($name,$saved[$name],'Process') }
}
Get-Item -LiteralPath (Join-Path $output 'PinboardUI.dll') | Select-Object FullName,Length
