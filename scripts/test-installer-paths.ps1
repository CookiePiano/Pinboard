# Run the production NSIS path validator without opening a window or creating destinations.
param([string]$WorkDirectory)
$ErrorActionPreference = 'Stop'
$source = Split-Path $PSScriptRoot -Parent
if (!$WorkDirectory) { $WorkDirectory = Join-Path 'C:\build\PinboardInstaller' ('paths-' + [guid]::NewGuid().ToString('N')) }
$null = New-Item -ItemType Directory -Path $WorkDirectory -Force
$work = (Resolve-Path -LiteralPath $WorkDirectory).Path
$destination = Join-Path $work 'New folder\메모'
if (Test-Path -LiteralPath $destination) { throw 'The regression destination must not exist.' }
$cases = @('', 'relative\folder', 'C:relative', 'C:\', $destination, (Join-Path $work 'one\..\two'))
$expected = @('invalid','invalid','invalid','invalid','','')
$script = @'
Unicode true
SilentInstall silent
RequestExecutionLevel user
!include FileFunc.nsh
!define ESTIMATEDSIZE 14368
OutFile "@WORK@\paths.exe"
!include "@SOURCE@\installer\path-validation.nsh"
Section
FileOpen $9 "@WORK@\results.txt" w
'@
for ($i=0; $i -lt $cases.Count; $i++) {
    $script += "`n" + 'StrCpy $0 "' + $cases[$i] + '"' + "`n" + 'Call PbValidatePath' + "`n" + 'FileWrite $9 "' + $i + ':$PbPathError$\r$\n"'
}
$script += "`nFileClose `$9`nSectionEnd`n"
$script = $script.Replace('@WORK@',$work).Replace('@SOURCE@',$source)
$scriptPath = Join-Path $work 'paths.nsi'
[IO.File]::WriteAllText($scriptPath,$script,[Text.UTF8Encoding]::new($false))
$compiler = if ($env:PINBOARD_NSIS_DIR) { Join-Path $env:PINBOARD_NSIS_DIR 'makensis.exe' } else { Join-Path $env:LOCALAPPDATA 'tauri\NSIS\makensis.exe' }
& $compiler /V2 /INPUTCHARSET UTF8 $scriptPath
if ($LASTEXITCODE -ne 0) { throw 'Path harness compilation failed.' }
$process = Start-Process -FilePath (Join-Path $work 'paths.exe') -WindowStyle Hidden -PassThru
if (!$process.WaitForExit(10000) -or $process.ExitCode -ne 0) { throw 'Path harness failed.' }
$results = [IO.File]::ReadAllLines((Join-Path $work 'results.txt'))
for ($i=0; $i -lt $cases.Count; $i++) {
    if ($results[$i] -cne ($i.ToString() + ':' + $expected[$i])) { throw "Path case $i failed: $($results[$i])" }
}
if (Test-Path -LiteralPath (Join-Path $work 'New folder')) { throw 'Validation created a destination directory.' }
Write-Output 'PASS: 6 path cases, including absent Korean/spaced folders, relative paths and drive roots; no destination created.'
