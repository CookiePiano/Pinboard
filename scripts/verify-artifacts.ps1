[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Directory,
    [Parameter(Mandatory)][string]$Version,
    [ValidateSet('Signed','UnsignedInitial')][string]$SigningPolicy = 'Signed',
    [string]$ExpectedSignerThumbprint
)
$ErrorActionPreference = 'Stop'
if ($SigningPolicy -eq 'UnsignedInitial' -and $Version -ne '0.1.0') { throw 'Unsigned policy is restricted to 0.1.0' }
if ($SigningPolicy -eq 'Signed' -and $ExpectedSignerThumbprint -notmatch '^[0-9a-fA-F]{40}$') { throw 'Approved publisher thumbprint is required' }
foreach ($name in @('Pinboard.exe', "Pinboard_${Version}_x64-setup.exe", 'Pinboard_Uninstall.exe')) {
    $file = Get-Item -LiteralPath (Join-Path $Directory $name)
    if ($file.VersionInfo.ProductVersion -ne $Version -or $file.VersionInfo.ProductName -ne 'Pinboard') { throw "Invalid product metadata: $name" }
    $signature = Get-AuthenticodeSignature -LiteralPath $file.FullName
    if ($SigningPolicy -eq 'Signed') {
        if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Thumbprint -ine $ExpectedSignerThumbprint -or !$signature.TimeStamperCertificate) { throw "Missing or unexpected publisher signature/timestamp: $name" }
    } elseif ($signature.Status -ne 'NotSigned') { throw "Initial unsigned release has an unexpected signature state: $name" }
    $bytes = [IO.File]::ReadAllBytes($file.FullName)
    foreach ($encoding in @([Text.Encoding]::UTF8, [Text.Encoding]::Unicode)) {
        if ($encoding.GetString($bytes).IndexOf($env:USERPROFILE, [StringComparison]::OrdinalIgnoreCase) -ge 0) { throw "Developer profile path in $name" }
    }
}
Write-Output "Artifact versions and $SigningPolicy policy verified."
