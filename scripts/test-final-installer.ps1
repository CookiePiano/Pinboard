# This fixture may use the real product identity ONLY on a clean disposable hosted runner.
[CmdletBinding()]
param([Parameter(Mandatory)][string]$Directory, [Parameter(Mandatory)][string]$WorkDirectory)
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw 'Final installer test requires a disposable GitHub-hosted runner' }
$registry = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Pinboard'
$data = Join-Path $env:APPDATA 'dev.pinboard.notes'
if ((Test-Path -LiteralPath $registry) -or (Test-Path -LiteralPath $data) -or (Get-Process Pinboard -ErrorAction SilentlyContinue)) { throw 'Runner is not clean; never overwrite an existing installation or user data' }
$null = New-Item -ItemType Directory -Path $WorkDirectory,$data -Force
$work = (Resolve-Path -LiteralPath $WorkDirectory).Path
$install = Join-Path $work 'Installed Pinboard 한글'
$sentinel = Join-Path $data 'release-test-sentinel.txt'
[IO.File]::WriteAllText($sentinel,'preserve')
$version = (Get-Content -Raw -LiteralPath (Join-Path $Directory 'release-manifest.json') | ConvertFrom-Json).version
function Run-Wizard([string]$Path,[string]$Arguments) {
    $p = Start-Process -FilePath $Path -ArgumentList $Arguments -WindowStyle Hidden -PassThru
    if (!$p.WaitForExit(120000)) { $p.Kill(); throw 'Installer fixture timed out' }
    $p.Refresh()
    if ($p.ExitCode -ne 0) { throw "Installer returned $($p.ExitCode)" }
}
Run-Wizard (Join-Path $Directory "Pinboard_${version}_x64-setup.exe") "/S /NS /D=$install"
$installedHash = (Get-FileHash -LiteralPath (Join-Path $install 'pinboard.exe')).Hash
$portableHash = (Get-FileHash -LiteralPath (Join-Path $Directory 'Pinboard.exe')).Hash
if ($installedHash -ne $portableHash) { throw "Installed payload $installedHash differs from portable payload $portableHash" }
# The corrected distribution intentionally retains the installed product version.
# Exercise replacement of an existing same-version installation and preserve data.
Run-Wizard (Join-Path $Directory "Pinboard_${version}_x64-setup.exe") "/S /NS /D=$install"
if ((Get-FileHash -LiteralPath (Join-Path $install 'pinboard.exe')).Hash -ne $portableHash) { throw 'Same-version reinstall changed the payload' }
if ([IO.File]::ReadAllText($sentinel) -ne 'preserve') { throw 'Same-version reinstall deleted note data' }
node (Join-Path $PSScriptRoot 'test-packaged.mjs') (Join-Path $install 'pinboard.exe') (Join-Path $work 'smoke')
if ($LASTEXITCODE -ne 0) { throw 'Installed app smoke failed' }
Run-Wizard (Join-Path $Directory 'Pinboard_Uninstall.exe') '/S'
if ((Test-Path -LiteralPath $registry) -or (Test-Path -LiteralPath (Join-Path $install 'pinboard.exe'))) { throw 'Removal did not complete' }
if ([IO.File]::ReadAllText($sentinel) -ne 'preserve') { throw 'Removal deleted note data' }
Remove-Item -LiteralPath $sentinel
Remove-Item -LiteralPath $data
Write-Output 'PASS: exact published installer, same-version reinstall, installed payload hash, MCP startup and removal/data preservation.'
