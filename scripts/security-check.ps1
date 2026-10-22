[CmdletBinding()]
param(
    [string]$AuditTool = 'cargo-audit',
    [switch]$StrictAudit
)

$ErrorActionPreference = 'Stop'
$sourceRoot = Split-Path $PSScriptRoot -Parent
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
foreach ($command in @('node', 'npm', 'cargo', 'gitleaks', $AuditTool)) {
    if (!(Get-Command $command -ErrorAction SilentlyContinue)) { throw "Required security-check tool is unavailable: $command" }
}
function Invoke-Checked([string]$Executable, [string[]]$Arguments) {
    & $Executable @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Executable failed with exit code $LASTEXITCODE" }
}
Push-Location $sourceRoot
try {
    Invoke-Checked 'npm' @('audit', '--audit-level=low')
    Invoke-Checked 'gitleaks' @('dir', '.', '--redact', '--no-banner')
    $auditArgs = @('audit', '--file', 'src-tauri/Cargo.lock', '--deny', 'yanked', '--json')
    if ($StrictAudit) { $auditArgs += @('--deny', 'warnings') }
    $auditText = & $AuditTool @auditArgs
    if ($LASTEXITCODE -ne 0) { throw 'Cargo audit failed; vulnerabilities, yanked dependencies, or scanner errors block release.' }
    $audit = ($auditText -join "`n") | ConvertFrom-Json
    $maintenance = Get-Content -Raw -LiteralPath 'scripts/audit-maintenance.json' | ConvertFrom-Json
    if ((Get-Date).ToUniversalTime() -gt [datetime]::Parse($maintenance.reviewAgainBy).AddDays(1)) { throw 'Dependency maintenance review has expired' }
    $warningCount = 0
    foreach ($category in $audit.warnings.PSObject.Properties) {
        foreach ($warning in $category.Value) {
            $identity = "$($warning.advisory.id):$($warning.package.name)@$($warning.package.version)"
            if ($category.Name -ne 'unmaintained' -or $maintenance.accepted -notcontains $identity) { throw "Unreviewed dependency warning: $identity" }
            $warningCount++
        }
    }
    Write-Output "Cargo audit: no vulnerabilities/yanked crates; $warningCount reviewed maintenance warnings."
    Invoke-Checked 'node' @('scripts/test-security.mjs')
    Invoke-Checked 'node' @('scripts/test-release-manifest.mjs')
    Invoke-Checked 'npm' @('run', 'check')
    foreach ($test in @('test-links.mjs', 'test-formatting.mjs', 'test-calendar-dates.mjs', 'test-i18n.mjs', 'test-save-queue.mjs', 'test-shortcut-capture.mjs', 'test-window-drag.mjs')) {
        Invoke-Checked 'node' @("scripts/$test")
    }
    Invoke-Checked 'cargo' @('test', '--manifest-path', 'src-tauri/Cargo.toml', '--locked', '-j', '24')
    Write-Output 'Security checks passed. Review informational dependency warnings before release.'
} finally {
    Pop-Location
}
