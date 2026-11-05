[CmdletBinding()]
param([Parameter(Mandatory = $true)][string]$ZipPath)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'file-hash.ps1')
. (Join-Path $PSScriptRoot 'release/PathSafety.ps1')
. (Join-Path $PSScriptRoot 'native-candidate/Archive.ps1')
$result = Test-NativeCandidateArchive -ZipPath $ZipPath
[ordered]@{
    buildVersion = $result.buildVersion
    files = $result.files
    executableSha256 = $result.executableSha256
    archiveSha256 = Get-OtterFileSha256 -Path $ZipPath
    publishable = $false
    runtimeVerified = $false
    signed = $false
} | ConvertTo-Json
