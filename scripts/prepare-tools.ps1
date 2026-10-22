[CmdletBinding()]
param(
    [ValidateSet('nsis','gitleaks','syft')][string[]]$Tools = @('nsis','gitleaks','syft'),
    [Parameter(Mandatory)][string]$Directory
)
$ErrorActionPreference = 'Stop'
$spec = Get-Content -Raw -LiteralPath (Join-Path $PSScriptRoot 'toolchain.json') | ConvertFrom-Json
$null = New-Item -ItemType Directory -Path $Directory -Force
$root = (Resolve-Path -LiteralPath $Directory).Path
foreach ($name in $Tools) {
    $download = $spec.downloads.$name
    $archive = Join-Path $root "$name.zip"
    if (!(Test-Path -LiteralPath $archive) -or (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash -ine $download.sha256) {
        Invoke-WebRequest -Uri $download.url -OutFile "$archive.partial"
        if ((Get-FileHash -LiteralPath "$archive.partial" -Algorithm SHA256).Hash -ine $download.sha256) { throw "$name download digest mismatch" }
        Move-Item -LiteralPath "$archive.partial" -Destination $archive -Force
    }
    # Only the authenticated archive is extracted; callers never execute an unverified download.
    $destination = Join-Path $root $name
    Expand-Archive -LiteralPath $archive -DestinationPath $destination -Force
    if ($name -eq 'nsis') {
        $env:PINBOARD_NSIS_DIR = Join-Path $destination 'nsis-3.13'
        if (!(Test-Path -LiteralPath (Join-Path $env:PINBOARD_NSIS_DIR 'Examples\Plugin\nsis\pluginapi.h'))) { throw 'NSIS SDK missing' }
        $pluginDirectory = Join-Path $env:PINBOARD_NSIS_DIR 'Plugins\x86-unicode\additional'
        $null = New-Item -ItemType Directory -Path $pluginDirectory -Force
        $plugin = Join-Path $pluginDirectory 'nsis_tauri_utils.dll'
        if (!(Test-Path -LiteralPath $plugin) -or (Get-FileHash -LiteralPath $plugin -Algorithm SHA256).Hash -ine $spec.nsisPlugin.sha256) {
            Invoke-WebRequest -Uri $spec.nsisPlugin.url -OutFile "$plugin.partial"
            if ((Get-FileHash -LiteralPath "$plugin.partial" -Algorithm SHA256).Hash -ine $spec.nsisPlugin.sha256) { throw 'NSIS plugin digest mismatch' }
            Move-Item -LiteralPath "$plugin.partial" -Destination $plugin -Force
        }
    } else {
        $env:Path = "$destination;$env:Path"
    }
    Write-Output "Verified tool: $name"
}
