# Restore scanning only in an ephemeral GitHub-hosted Windows VM.
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted') {
    throw 'Defender configuration is restricted to disposable GitHub-hosted runners'
}
# runner-images disables scanning and excludes both drives. Never apply this to a developer PC.
$policy = 'HKLM:\SOFTWARE\Policies\Microsoft\Windows Advanced Threat Protection'
if (Test-Path -LiteralPath $policy) {
    Set-ItemProperty -LiteralPath $policy -Name ForceDefenderPassiveMode -Value 0 -Type DWord
}
Start-Service WinDefend
Set-MpPreference -DisableArchiveScanning $false -DisableScriptScanning $false -DisableRealtimeMonitoring $false -DisableIOAVProtection $false -DisableBehaviorMonitoring $false -DisableBlockAtFirstSeen $false -SubmitSamplesConsent NeverSend
$preferences = Get-CimInstance -Namespace root/Microsoft/Windows/Defender -ClassName MSFT_MpPreference
foreach ($kind in @('ExclusionPath','ExclusionProcess','ExclusionExtension')) {
    if ($preferences.$kind) {
        $arguments = @{ $kind = @($preferences.$kind) }
        Remove-MpPreference @arguments
    }
}
$status = Get-CimInstance -Namespace root/Microsoft/Windows/Defender -ClassName MSFT_MpComputerStatus
$preferences = Get-CimInstance -Namespace root/Microsoft/Windows/Defender -ClassName MSFT_MpPreference
if (!$status.AntivirusEnabled -or $preferences.DisableArchiveScanning -or $preferences.DisableScriptScanning -or $preferences.ExclusionPath -or $preferences.ExclusionProcess -or $preferences.ExclusionExtension) {
    throw 'Hosted runner Defender preparation did not take effect'
}
Write-Output 'Hosted Defender enabled; archive/script scanning enabled; exclusions cleared; sample submission disabled.'
