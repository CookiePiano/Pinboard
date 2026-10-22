[CmdletBinding()]
param([Parameter(Mandatory)][string]$Directory, [Parameter(Mandatory)][string]$EvidenceDirectory)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path -LiteralPath $Directory).Path
$null = New-Item -ItemType Directory -Path $EvidenceDirectory -Force
$status = Get-CimInstance -Namespace root/Microsoft/Windows/Defender -ClassName MSFT_MpComputerStatus
if (!$status.AntivirusEnabled) { throw 'Windows Defender antivirus is not enabled' }
$platform = Join-Path $env:ProgramData 'Microsoft\Windows Defender\Platform'
$scanner = Get-ChildItem -LiteralPath $platform -Filter MpCmdRun.exe -Recurse -ErrorAction SilentlyContinue | Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName
if (!$scanner) { $scanner = Join-Path $env:ProgramFiles 'Windows Defender\MpCmdRun.exe' }
if (!(Test-Path -LiteralPath $scanner)) { throw 'Windows Defender scanner is unavailable' }
& $scanner -SignatureUpdate *> (Join-Path $EvidenceDirectory 'definitions.log')
if ($LASTEXITCODE -ne 0) { throw 'Defender signature update failed' }
$status = Get-CimInstance -Namespace root/Microsoft/Windows/Defender -ClassName MSFT_MpComputerStatus
if ((Get-Date) - $status.AntivirusSignatureLastUpdated -gt [TimeSpan]::FromDays(2)) { throw 'Defender definitions are stale' }
$status | Select-Object AntivirusEnabled, AntivirusSignatureVersion, AntivirusSignatureLastUpdated | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $EvidenceDirectory 'engine.json') -Encoding utf8
# Disable remediation: preserve evidence and never quarantine another application's files.
& $scanner -Scan -ScanType 3 -File $root -DisableRemediation *> (Join-Path $EvidenceDirectory 'scan.log')
if ($LASTEXITCODE -ne 0) { throw 'Defender detected a threat or could not complete the scan; see private evidence' }
Write-Output 'Defender custom scan completed without detections (no-remediation mode).'
