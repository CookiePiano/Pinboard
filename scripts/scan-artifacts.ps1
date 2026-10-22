[CmdletBinding()]
param([Parameter(Mandatory)][string]$Directory, [Parameter(Mandatory)][string]$EvidenceDirectory)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $Directory).Path
$null = New-Item -ItemType Directory -Path $EvidenceDirectory -Force
$status = Get-CimInstance -Namespace root/Microsoft/Windows/Defender -ClassName MSFT_MpComputerStatus
if (!$status.AntivirusEnabled) { throw 'Windows Defender antivirus is not enabled' }
$preferences = Get-CimInstance -Namespace root/Microsoft/Windows/Defender -ClassName MSFT_MpPreference
if ($preferences.DisableArchiveScanning) { throw 'Defender archive scanning is disabled; portable ZIP inspection is required' }
$platform = Join-Path $env:ProgramData 'Microsoft\Windows Defender\Platform'
$scanner = Get-ChildItem -LiteralPath $platform -Filter MpCmdRun.exe -Recurse -ErrorAction SilentlyContinue | Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName
if (!$scanner) { $scanner = Join-Path $env:ProgramFiles 'Windows Defender\MpCmdRun.exe' }
if (!(Test-Path -LiteralPath $scanner)) { throw 'Windows Defender scanner is unavailable' }
& $scanner -SignatureUpdate *> (Join-Path $EvidenceDirectory 'definitions.log')
if ($LASTEXITCODE -ne 0) { throw 'Defender signature update failed' }
$status = Get-CimInstance -Namespace root/Microsoft/Windows/Defender -ClassName MSFT_MpComputerStatus
if ((Get-Date) - $status.AntivirusSignatureLastUpdated -gt [TimeSpan]::FromDays(2)) { throw 'Defender definitions are stale' }
$status | Select-Object AntivirusEnabled, AntivirusSignatureVersion, AntivirusSignatureLastUpdated | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $EvidenceDirectory 'engine.json') -Encoding utf8
# Hosted runners are elevated, so additionally verify their exclusion cleanup.
# On a normal developer account CheckExclusion requires elevation; the custom
# DisableRemediation scan itself ignores file exclusions and inspects archives
# (documented by MpCmdRun -?), without changing the user's Defender settings.
if ($env:GITHUB_ACTIONS -eq 'true' -and $env:RUNNER_ENVIRONMENT -eq 'github-hosted') {
    & $scanner -CheckExclusion -Path $root *> (Join-Path $EvidenceDirectory 'exclusion-check.log')
    if ($LASTEXITCODE -ne 1) { throw 'Scan directory is excluded or its exclusion state could not be verified' }
}
# Preserve evidence and never quarantine another application's files.
& $scanner -Scan -ScanType 3 -File $root -DisableRemediation *> (Join-Path $EvidenceDirectory 'scan.log')
if ($LASTEXITCODE -ne 0) { throw 'Defender detected a threat or could not complete the scan; see private evidence' }
Write-Output 'Defender custom scan completed without detections (no-remediation mode).'
