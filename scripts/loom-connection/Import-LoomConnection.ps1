# Import public connection data on the receiving PC; never copy device identities.
[CmdletBinding()]
param(
    [string]$ConnectionFile = (Join-Path $PSScriptRoot 'loom-remote.json'),
    [string]$ProfileDirectory = $(if ($env:OTTER_NATIVE_PROFILE_DIR) { $env:OTTER_NATIVE_PROFILE_DIR } else { Join-Path $env:LOCALAPPDATA 'OtterNative' }),
    [string]$Executable = ''
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ([IO.Path]::GetFullPath($ProfileDirectory) -eq [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'OtterNative')) -and (Get-Process -Name 'otter-native' -ErrorAction SilentlyContinue)) { throw 'Exit Otter before importing the connection' }
$limit = 40KB
$inputFile = [IO.File]::OpenRead([IO.Path]::GetFullPath($ConnectionFile))
try {
    if ($inputFile.Length -gt $limit) { throw 'Connection file is too large' }
    $bytes = [byte[]]::new($limit + 1)
    $length = 0
    do {
        $read = $inputFile.Read($bytes, $length, $bytes.Length - $length)
        $length += $read
        if ($length -gt $limit) { throw 'Connection file is too large' }
    } while ($read -ne 0)
} finally { $inputFile.Dispose() }
$encoding = [Text.UTF8Encoding]::new($false, $true)
$document = $encoding.GetString($bytes, 0, $length) | ConvertFrom-Json
$fields = @($document.PSObject.Properties.Name | Sort-Object)
if (($fields -join ',') -ne 'certificatePem,loomId,origin,schemaVersion' -or $document.schemaVersion -ne 1) { throw 'Invalid connection document' }
$origin = [Uri]$document.origin
if ($origin.Scheme -ne 'https' -or $origin.UserInfo -or $origin.Query -or $origin.Fragment -or $origin.AbsolutePath -ne '/' -or $document.loomId -cnotmatch '\Aloom-[0-9a-f]{64}\z') { throw 'Invalid Loom origin or identity' }
if ($document.certificatePem -notmatch '\A\s*-----BEGIN CERTIFICATE-----\s*([A-Za-z0-9+/=\s]+)\s*-----END CERTIFICATE-----\s*\z') { throw 'Only a public PEM certificate is accepted' }
$certificate = [Security.Cryptography.X509Certificates.X509Certificate2]::new([Convert]::FromBase64String($Matches[1]))
$certificate.Dispose()
$profile = [IO.Path]::GetFullPath($ProfileDirectory)
for ($parent = $profile; $parent; $parent = Split-Path -Parent $parent) {
    if ((Test-Path -LiteralPath $parent) -and ((Get-Item -LiteralPath $parent -Force).Attributes -band [IO.FileAttributes]::ReparsePoint)) { throw 'Profile cannot use reparse points' }
}
[IO.Directory]::CreateDirectory($profile) | Out-Null
function Install-Bytes([string]$Name, [byte[]]$Content, [int]$Count = $Content.Length) {
    $destination = Join-Path $profile $Name
    if (Test-Path -LiteralPath $destination) {
        if ((Get-Item -LiteralPath $destination -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Connection file cannot be a reparse point' }
        $backup = "$destination.$([DateTime]::UtcNow.ToString('yyyyMMddHHmmssfffffff')).backup"
    }
    $temporary = Join-Path $profile ('.loom-import-' + [Guid]::NewGuid().ToString('N') + '.tmp')
    $output = [IO.File]::Open($temporary, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
    try { $output.Write($Content, 0, $Count) } finally { $output.Dispose() }
    if (Test-Path -LiteralPath $destination) { [IO.File]::Replace($temporary, $destination, $backup) }
    else { [IO.File]::Move($temporary, $destination) }
}
Install-Bytes 'loom-remote.json' $bytes $length
Install-Bytes 'loom-autoconnect-paused.json' ([Text.Encoding]::UTF8.GetBytes((@{paused=$false;origin=$document.origin} | ConvertTo-Json -Compress)))
Write-Output "Loom connection installed: $($document.origin)"
Write-Output 'On first connection, approve this computer in Loom device management.'
if ($Executable) {
    $exe = [IO.Path]::GetFullPath($Executable)
    if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw 'Otter executable not found' }
    $env:OTTER_NATIVE_PROFILE_DIR = $profile
    Start-Process -FilePath $exe -WorkingDirectory (Split-Path -Parent $exe)
}
