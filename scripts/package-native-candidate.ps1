[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$CandidateDirectory,
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [switch]$PublicRelease
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..')).TrimEnd('\')
. (Join-Path $PSScriptRoot 'file-hash.ps1')
. (Join-Path $PSScriptRoot 'release/PathSafety.ps1')
. (Join-Path $PSScriptRoot 'native-candidate/Tools.ps1')
. (Join-Path $PSScriptRoot 'native-candidate/Archive.ps1')
$candidate = [IO.Path]::GetFullPath($CandidateDirectory).TrimEnd('\')
$output = [IO.Path]::GetFullPath($OutputDirectory).TrimEnd('\')
foreach ($directory in @($candidate, $output)) {
    if ($directory.StartsWith('\\')) { throw 'Use local or existing mapped-drive paths' }
    [void](Assert-OtterAbsolutePathNoReparsePoints -Path $directory -TrustedRootPath ([IO.Path]::GetPathRoot($directory)))
}
if (Test-Path -LiteralPath $output) { throw 'Native package output already exists' }
foreach ($source in @($repo, $candidate)) {
    if ($output -eq $source -or $output.StartsWith($source + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Native package output overlaps source' }
}
[void][IO.Directory]::CreateDirectory($output)
try {
    $node = (Get-Command node -CommandType Application | Select-Object -First 1).Source
    $stage = Join-Path $output 'payload'
    $channel = if ($PublicRelease) { 'public' } else { 'internal' }
    Invoke-NativeCandidateTool -Executable $node -Arguments @((Join-Path $PSScriptRoot 'native-candidate/stage.cjs'), $repo, $candidate, $stage, $channel) -LogPath (Join-Path $output 'stage.log')
    $manifest = (Read-OtterBoundedText -Path (Join-Path $stage 'package-manifest.json')) | ConvertFrom-Json
    $pattern = if ($PublicRelease) { '^v\d+\.\d+\.\d+$' } else { '^v\d+\.\d+\.\d+\.\d+$' }
    if ($manifest.buildVersion -cnotmatch $pattern) { throw 'Invalid package version' }
    $suffix = if ($PublicRelease) { '' } else { '-internal' }
    $zip = Join-Path $output ("Otter-native-$($manifest.buildVersion)-windows-x64$suffix.zip")
    New-NativeCandidateArchive -Directory $stage -ZipPath $zip
    $result = Test-NativeCandidateArchive -ZipPath $zip -PublicRelease:$PublicRelease
    Write-NativeCandidateJson -Path (Join-Path $output 'archive-verification.json') -Value ([ordered]@{
        buildVersion = $result.buildVersion; files = $result.files; executableSha256 = $result.executableSha256
        archiveSha256 = Get-OtterFileSha256 -Path $zip; archive = [IO.Path]::GetFileName($zip)
        publishable = $false; signed = $false; runtimeVerified = $false
    })
    Write-Output "Native $channel ZIP verified: $zip"
} catch {
    [IO.File]::WriteAllText((Join-Path $output 'FAILED.txt'), $_.ToString(), [Text.UTF8Encoding]::new($false))
    throw
}
