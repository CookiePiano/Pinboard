# Exercise only a uniquely named disposable installation; never the real Pinboard key.
param([string]$WorkDirectory,[string]$RenderedDirectory,[string]$UiDirectory)
$ErrorActionPreference = 'Stop'
$source = Split-Path $PSScriptRoot -Parent
$compiler = if ($env:PINBOARD_NSIS_DIR) { Join-Path $env:PINBOARD_NSIS_DIR 'makensis.exe' } else { Join-Path $env:LOCALAPPDATA 'tauri\NSIS\makensis.exe' }
$rendered = if ($RenderedDirectory) { (Resolve-Path -LiteralPath $RenderedDirectory).Path } else { Join-Path $source 'src-tauri\target\release\nsis\x64' }
if (!$UiDirectory) { $UiDirectory = Join-Path $source 'installer\build' }
$previousPluginDirectory = $env:PINBOARD_UI_PLUGIN_DIR
$env:PINBOARD_UI_PLUGIN_DIR = (Resolve-Path -LiteralPath $UiDirectory).Path
if (!$WorkDirectory) { $WorkDirectory = Join-Path 'C:\build\PinboardInstaller' ('verify-' + [guid]::NewGuid().ToString('N')) }
New-Item -ItemType Directory -Path $WorkDirectory -Force | Out-Null
$work = (Resolve-Path -LiteralPath $WorkDirectory).Path
$qaName = 'PinboardInstallerQA_' + [guid]::NewGuid().ToString('N').Substring(0, 12)
$qaBundle = 'dev.pinboard.installerqa.' + $qaName
$qaInstall = Join-Path $work 'Installed Pinboard'
$qaRegistry = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\' + $qaName
$qaPrefs = 'HKCU:\Software\pinboard-qa\' + $qaName
$qaData = Join-Path $env:APPDATA $qaBundle
$productionRegistry = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Pinboard'
$productionBefore = Get-ItemProperty -LiteralPath $productionRegistry -ErrorAction SilentlyContinue |
    Select-Object DisplayName, DisplayVersion, InstallLocation, UninstallString | ConvertTo-Json -Compress
$productionProcesses = @(Get-CimInstance Win32_Process -Filter "Name='Pinboard.exe'" | Select-Object ProcessId, ExecutablePath)
$checks = [Collections.Generic.List[string]]::new()
$dummyProcess = $null

function Assert-Installer([bool]$Condition, [string]$Message) {
    if (!$Condition) { throw $Message }
    $checks.Add($Message)
    Write-Output "PASS: $Message"
}

function Invoke-SilentWizard([string]$FilePath, [string]$Arguments) {
    $process = Start-Process -FilePath $FilePath -ArgumentList $Arguments -WindowStyle Hidden -PassThru
    if (!$process.WaitForExit(45000)) { throw "Timed out waiting for the disposable wizard: $FilePath" }
    $process.Refresh()
    return $process.ExitCode
}

try {
    $template = [IO.File]::ReadAllText((Join-Path $rendered 'installer.nsi'))
    if (!$template.Contains('!define PRODUCTNAME "Pinboard"') -or !$template.Contains('!define BUNDLEID "dev.pinboard.notes"')) {
        throw 'Unexpected generated template. Review the fixture before running this test.'
    }
    $payloadDefinition = [regex]::Match($template, '(?m)^!define MAINBINARYSRCPATH "([^"\r\n]+)"\r?$')
    if (!$payloadDefinition.Success) { throw 'The generated template does not identify its executable payload.' }
    $payloadPath = $payloadDefinition.Groups[1].Value.Replace('$$', '$')
    $payloadHash = (Get-FileHash -LiteralPath $payloadPath -Algorithm SHA256).Hash
    $versionDefinition = [regex]::Match($template, '(?m)^!define VERSION "([^"\r\n]+)"\r?$')
    if (!$versionDefinition.Success) { throw 'The generated template does not identify its packaged version.' }
    $packagedVersion = $versionDefinition.Groups[1].Value
    $template = $template.Replace('!define PRODUCTNAME "Pinboard"', "!define PRODUCTNAME `"$qaName`"")
    $template = $template.Replace('!define BUNDLEID "dev.pinboard.notes"', "!define BUNDLEID `"$qaBundle`"")
    $template = [regex]::Replace($template, '(?m)^!define MANUFACTURER "[^"\r\n]+"', '!define MANUFACTURER "pinboard-qa"')
    $qaSetup = Join-Path $work 'QA-setup.exe'
    $template = $template.Replace('!define OUTFILE "nsis-output.exe"', "!define OUTFILE `"$qaSetup`"")
    # The runtime is already present; the fixture must never download/install system components.
    $template = [regex]::Replace($template, '(?ms)^Section WebView2\r?\n.*?^SectionEnd\r?\n', '; Runtime acquisition intentionally excluded from the isolated fixture.' + "`n")
    $qaScript = Join-Path $work 'installer.nsi'
    [IO.File]::WriteAllText($qaScript, $template, [Text.UTF8Encoding]::new($false))
    foreach ($file in @('utils.nsh', 'FileAssociation.nsh')) {
        Copy-Item -LiteralPath (Join-Path $rendered $file) -Destination (Join-Path $work $file)
    }
    Push-Location $work
    try {
        & $compiler '/V2' '/INPUTCHARSET' 'UTF8' $qaScript
        if ($LASTEXITCODE -ne 0) { throw 'Disposable installer compilation failed.' }
    } finally { Pop-Location }
    & (Join-Path $PSScriptRoot 'build-uninstaller.ps1') -ProductName $qaName -OutputDirectory (Join-Path $work 'launcher') -UiDirectory $UiDirectory
    $remover = Join-Path $work 'launcher\Pinboard_Uninstall.exe'

    Assert-Installer ((Invoke-SilentWizard $remover '/S') -eq 2) 'A missing installation returns 2 without removing files.'
    Assert-Installer ((Invoke-SilentWizard $qaSetup "/S /NS /D=$qaInstall") -eq 0) 'A clean isolated installation succeeds at a path containing spaces.'
    Assert-Installer (Test-Path -LiteralPath (Join-Path $qaInstall 'pinboard.exe')) 'The disposable app executable is installed.'
    Assert-Installer ((Get-FileHash -LiteralPath (Join-Path $qaInstall 'pinboard.exe') -Algorithm SHA256).Hash -eq $payloadHash) 'A clean installation preserves the exact packaged executable bytes.'
    Assert-Installer (Test-Path -LiteralPath $qaRegistry) 'Only the uniquely named uninstall registration is created.'

    New-Item -ItemType Directory -Path $qaData -Force | Out-Null
    $sentinel = Join-Path $qaData 'notes-preservation-sentinel.txt'
    [IO.File]::WriteAllText($sentinel, 'keep-this-disposable-note', [Text.UTF8Encoding]::new($false))
    $registration = Get-ItemProperty -LiteralPath $qaRegistry
    Set-ItemProperty -LiteralPath $qaRegistry -Name UninstallString -Value ($registration.UninstallString + ' --unexpected')
    Assert-Installer ((Invoke-SilentWizard $remover '/S') -eq 2) 'Unexpected arguments in a registration are rejected.'
    Assert-Installer (Test-Path -LiteralPath (Join-Path $qaInstall 'pinboard.exe')) 'A rejected registration leaves the app in place.'
    Set-ItemProperty -LiteralPath $qaRegistry -Name UninstallString -Value $registration.UninstallString

    # Passive mode exercises the real native shell without clicking a destructive UI.
    Assert-Installer ((Invoke-SilentWizard $qaSetup "/P /NS /LANG=1033 /D=$qaInstall") -eq 0) 'Passive same-version reinstall succeeds with the English native UI.'
    Assert-Installer ((Get-FileHash -LiteralPath (Join-Path $qaInstall 'pinboard.exe') -Algorithm SHA256).Hash -eq $payloadHash) 'Same-version reinstall preserves the packaged executable hash.'
    Assert-Installer (([IO.File]::ReadAllText($sentinel)) -eq 'keep-this-disposable-note') 'Same-version reinstall preserves note data.'
    Set-ItemProperty -LiteralPath $qaRegistry -Name DisplayVersion -Value '0.0.9'
    Assert-Installer ((Invoke-SilentWizard $qaSetup "/P /NS /LANG=1042 /D=$qaInstall") -eq 0) 'Passive upgrade succeeds through the Korean native UI.'
    Assert-Installer ((Get-FileHash -LiteralPath (Join-Path $qaInstall 'pinboard.exe') -Algorithm SHA256).Hash -eq $payloadHash) 'Upgrade preserves the packaged executable hash.'
    Assert-Installer ((Get-ItemProperty -LiteralPath $qaRegistry).DisplayVersion -eq $packagedVersion) 'Upgrade writes the packaged version.'
    Set-ItemProperty -LiteralPath $qaRegistry -Name DisplayVersion -Value '0.2.0'
    Assert-Installer ((Invoke-SilentWizard $qaSetup "/S /NS /D=$qaInstall") -ne 0) 'An unattended downgrade is rejected.'
    Assert-Installer ((Get-ItemProperty -LiteralPath $qaRegistry).DisplayVersion -eq '0.2.0') 'Rejected downgrade preserves the installed version.'
    Assert-Installer ((Get-FileHash -LiteralPath (Join-Path $qaInstall 'pinboard.exe') -Algorithm SHA256).Hash -eq $payloadHash) 'Rejected downgrade preserves the executable bytes.'
    Set-ItemProperty -LiteralPath $qaRegistry -Name DisplayVersion -Value $packagedVersion
    Assert-Installer ((Invoke-SilentWizard $qaSetup "/S /UPDATE /NS /D=$qaInstall") -eq 0) 'Silent update mode remains compatible.'
    Assert-Installer ((Get-FileHash -LiteralPath (Join-Path $qaInstall 'pinboard.exe') -Algorithm SHA256).Hash -eq $payloadHash) 'Silent update preserves the packaged executable hash.'
    Assert-Installer (([IO.File]::ReadAllText($sentinel)) -eq 'keep-this-disposable-note') 'Upgrade, downgrade and silent update preserve note data.'

    # A task-owned inert process proves the wizard refuses to force-close an app.
    $dummySource = Join-Path $work 'RunningFixture.cs'
    [IO.File]::WriteAllText($dummySource, 'using System.Threading; class RunningFixture { static void Main() { Thread.Sleep(120000); } }')
    $dummyExe = Join-Path $work 'RunningFixture.exe'
    & "$env:WINDIR\Microsoft.NET\Framework64\v4.0.30319\csc.exe" '/nologo' '/target:winexe' "/out:$dummyExe" $dummySource
    if ($LASTEXITCODE -ne 0) { throw 'Running-process fixture compilation failed.' }
    $installedExe = Join-Path $qaInstall 'pinboard.exe'
    $originalExe = Join-Path $qaInstall 'pinboard-fixture-backup.exe'
    Move-Item -LiteralPath $installedExe -Destination $originalExe
    Copy-Item -LiteralPath $dummyExe -Destination $installedExe
    $dummyProcess = Start-Process -FilePath $installedExe -WindowStyle Hidden -PassThru
    Start-Sleep -Milliseconds 350
    Assert-Installer ((Invoke-SilentWizard $remover '/S') -eq 3) 'An installed app that is running returns 3.'
    Assert-Installer (!$dummyProcess.HasExited) 'The running app is not force-closed.'
    $dummyProcess.Kill()
    $dummyProcess.WaitForExit()
    $dummyProcess = $null
    Remove-Item -LiteralPath $installedExe
    Move-Item -LiteralPath $originalExe -Destination $installedExe

    Assert-Installer ((Invoke-SilentWizard $remover '/S') -eq 0) 'The registered uninstaller completes successfully.'
    Assert-Installer (!(Test-Path -LiteralPath $qaInstall)) 'The disposable app and its installation directory are removed.'
    Assert-Installer (!(Test-Path -LiteralPath $qaRegistry)) 'The disposable uninstall registration is removed.'
    Assert-Installer (([IO.File]::ReadAllText($sentinel)) -eq 'keep-this-disposable-note') 'Notes and settings data are preserved.'
    Assert-Installer ((Invoke-SilentWizard $remover '/S') -eq 2) 'Running the uninstaller again is safe.'

    Assert-Installer ((Invoke-SilentWizard $qaSetup "/S /NS /D=$qaInstall") -eq 0) 'A removed fixture can be installed again.'
    $nativeUninstaller = Join-Path $qaInstall 'uninstall.exe'
    Assert-Installer ((Invoke-SilentWizard $nativeUninstaller "/P _?=$qaInstall") -eq 0) 'The installed native uninstaller completes in passive mode.'
    Assert-Installer (!(Test-Path -LiteralPath $qaRegistry)) 'The installed native uninstaller removes its own registration.'
    Assert-Installer (([IO.File]::ReadAllText($sentinel)) -eq 'keep-this-disposable-note') 'The native uninstaller preserves notes by default.'
    # In-place execution permits awaiting the actual uninstaller; clean its known file only.
    if (Test-Path -LiteralPath $nativeUninstaller) { Remove-Item -LiteralPath $nativeUninstaller }
    if (Test-Path -LiteralPath $qaInstall) { Remove-Item -LiteralPath $qaInstall }

    $productionAfter = Get-ItemProperty -LiteralPath $productionRegistry -ErrorAction SilentlyContinue |
        Select-Object DisplayName, DisplayVersion, InstallLocation, UninstallString | ConvertTo-Json -Compress
    Assert-Installer ($productionBefore -ceq $productionAfter) 'The real Pinboard uninstall registration remains unchanged.'
    foreach ($original in $productionProcesses) {
        $current = Get-CimInstance Win32_Process -Filter "ProcessId=$($original.ProcessId)"
        Assert-Installer ($null -ne $current -and $current.ExecutablePath -ceq $original.ExecutablePath) "Pre-existing Pinboard process $($original.ProcessId) is preserved."
    }
    $result = [pscustomobject]@{ Product = $qaName; WorkDirectory = $work; PayloadSha256 = $payloadHash; Passed = $checks.Count; Checks = $checks }
    $result | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $work 'results.json') -Encoding utf8NoBOM
    # Clean only the known disposable sentinel and preference key, after preservation is proved.
    Remove-Item -LiteralPath $sentinel
    Remove-Item -LiteralPath $qaData
    if ($qaPrefs -notlike 'HKCU:\Software\pinboard-qa\PinboardInstallerQA_*') { throw 'Unexpected fixture preference path.' }
    if (Test-Path -LiteralPath $qaPrefs) { Remove-Item -LiteralPath $qaPrefs }
    $result | Select-Object Product, WorkDirectory, Passed | ConvertTo-Json -Compress
} finally {
    $env:PINBOARD_UI_PLUGIN_DIR = $previousPluginDirectory
    if ($dummyProcess -and !$dummyProcess.HasExited) {
        $dummyProcess.Kill()
        $dummyProcess.WaitForExit()
    }
}
