[CmdletBinding()]
param([Parameter(Mandatory)][string]$WorkDirectory, [Parameter(Mandatory)][string]$UiDirectory)
$ErrorActionPreference = 'Stop'
$null = New-Item -ItemType Directory -Path $WorkDirectory -Force
$work = (Resolve-Path -LiteralPath $WorkDirectory).Path
$signed = Join-Path $work 'Microsoft bootstrapper.exe'
# Download for verification only. This test never installs WebView2.
Invoke-WebRequest 'https://go.microsoft.com/fwlink/p/?LinkId=2124703' -OutFile $signed
$signature = Get-AuthenticodeSignature -LiteralPath $signed
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation(?:,|$)') { throw 'Test fixture is not a trusted Microsoft binary' }
$tampered = Join-Path $work 'tampered.exe'
$bytes = [IO.File]::ReadAllBytes($signed)
$bytes[4096] = $bytes[4096] -bxor 1
[IO.File]::WriteAllBytes($tampered, $bytes)
$unsigned = Join-Path $work 'unsigned.exe'
[IO.File]::WriteAllText($unsigned, 'not an executable')
$other = (Get-Command node).Source
$otherSignature = Get-AuthenticodeSignature -LiteralPath $other
if ($otherSignature.Status -ne 'Valid' -or $otherSignature.SignerCertificate.Subject -match 'O=Microsoft Corporation(?:,|$)') { throw 'Node must provide a trusted non-Microsoft negative fixture' }
$cases = @($signed, $tampered, $unsigned, $other, (Join-Path $work 'missing.exe'))
$expected = @('ok','untrusted','untrusted','untrusted','untrusted')
$script = @'
Unicode true
SilentInstall silent
RequestExecutionLevel user
!include LogicLib.nsh
!addplugindir "@UI@"
OutFile "@WORK@\verify.exe"
Section
FileOpen $9 "@WORK@\results.txt" w
'@
for ($i=0; $i -lt $cases.Count; $i++) {
    $script += "`n" + 'PinboardVerify::VerifyMicrosoft /NOUNLOAD "' + $cases[$i] + '"' + "`nPop `$1`n" + 'FileWrite $9 "' + $i + ':$1$\r$\n"'
    if ($i -eq 0) {
        $script += "`n" + "System::Call 'kernel32::CreateFileW(w `"$signed`", i 0x40000000, i 7, p 0, i 3, i 0, p 0)p.r0'" + "`n" + 'FileWrite $9 "locked:$0$\r$\n"' + "`n" + '${If} $0 != -1' + "`n" + "System::Call 'kernel32::CloseHandle(p r0)'" + "`n" + '${EndIf}'
    }
    $script += "`nPinboardVerify::CloseVerified`n"
}
$script += "`nFileClose `$9`nSectionEnd`n"
$script = $script.Replace('@UI@',(Resolve-Path -LiteralPath $UiDirectory).Path).Replace('@WORK@',$work)
$file = Join-Path $work 'verify.nsi'
[IO.File]::WriteAllText($file,$script,[Text.UTF8Encoding]::new($false))
& (Join-Path $env:PINBOARD_NSIS_DIR 'makensis.exe') /V2 /INPUTCHARSET UTF8 $file
if ($LASTEXITCODE -ne 0) { throw 'Verifier fixture compilation failed' }
$process = Start-Process -FilePath (Join-Path $work 'verify.exe') -WindowStyle Hidden -PassThru
if (!$process.WaitForExit(60000)) { $process.Kill(); throw 'Verifier fixture timed out' }
$process.Refresh()
if ($process.ExitCode -ne 0) { throw 'Verifier fixture failed' }
$results = [IO.File]::ReadAllLines((Join-Path $work 'results.txt'))
for ($i=0; $i -lt $cases.Count; $i++) {
    if ($results -cnotcontains "${i}:$($expected[$i])") { throw "Signature case $i failed" }
}
if ($results -cnotcontains 'locked:-1') { throw 'Verified file can be replaced before execution' }
Write-Output 'PASS: trusted Microsoft, tampered, unsigned, other publisher, missing file and write-lock checks; nothing installed.'
