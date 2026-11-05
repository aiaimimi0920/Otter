[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [Parameter(Mandatory = $true)][string]$TargetDirectory
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
. (Join-Path $PSScriptRoot 'file-hash.ps1')
. (Join-Path $PSScriptRoot 'release/PathSafety.ps1')
. (Join-Path $PSScriptRoot 'version-identity.ps1')
. (Join-Path $PSScriptRoot 'native-candidate/Tools.ps1')
$version = Get-OtterVersionIdentity -RepoRoot $repo
$output = [IO.Path]::GetFullPath($OutputDirectory)
$target = [IO.Path]::GetFullPath($TargetDirectory)
foreach ($directory in @($output, $target)) {
    if ($directory -eq $repo -or $directory.StartsWith($repo + '\', [StringComparison]::OrdinalIgnoreCase) -or $directory.StartsWith('\\')) {
        throw 'Native candidate output/cache must be outside the source repository and use a local or existing mapped-drive path.'
    }
    [void](Assert-OtterAbsolutePathNoReparsePoints -Path $directory -TrustedRootPath ([IO.Path]::GetPathRoot($directory)))
}
if ($output -eq $target -or $output.StartsWith($target + '\', [StringComparison]::OrdinalIgnoreCase) -or $target.StartsWith($output + '\', [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Candidate output and reusable build cache must be separate directories.'
}
if (Test-Path -LiteralPath $output) { throw "Candidate destination already exists: $output" }
$commands = @{}
foreach ($name in @('cargo', 'rustc', 'node', 'git')) {
    $commands[$name] = (Get-Command $name -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source
}
$resource = $env:OTTER_NATIVE_RC
if (-not $resource) {
    foreach ($name in @('rc.exe', 'llvm-rc.exe')) {
        $found = Get-Command $name -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($found) { $resource = $found.Source; break }
    }
}
if (-not $resource -or -not (Test-Path -LiteralPath $resource -PathType Leaf)) { throw 'A resource compiler executable is required.' }
$commands['resource'] = [IO.Path]::GetFullPath($resource)
$oldTarget = $env:CARGO_TARGET_DIR
$oldResource = $env:OTTER_NATIVE_RC
$cacheLock = $null
Push-Location $repo
try {
    $cacheLock = Open-NativeCandidateCacheLock -TargetDirectory $target
    $env:CARGO_TARGET_DIR = $target
    $env:OTTER_NATIVE_RC = $commands['resource']
    $tools = Get-NativeCandidateTools -RepoRoot $repo -Commands $commands
    $commit = (@(& $commands['git'] rev-parse HEAD) -join '').Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Cannot identify source commit.' }
    $dirtyLines = @(& $commands['git'] status --porcelain --untracked-files=all)
    if ($LASTEXITCODE -ne 0) { throw 'Cannot identify source dirty state.' }
    New-Item -ItemType Directory -Path $output | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $output 'payload') | Out-Null
    Write-NativeCandidateJson -Path (Join-Path $output 'tools-before.json') -Value $tools
    $metadata = Join-Path $output 'metadata.json'
    $data = @(& $commands['cargo'] metadata --manifest-path native/Cargo.toml --format-version 1 --locked --offline)
    if ($LASTEXITCODE -ne 0) { throw 'Native offline metadata failed.' }
    [IO.File]::WriteAllText($metadata, ($data -join "`n"), [Text.UTF8Encoding]::new($false))
    $sourceScript = Join-Path $PSScriptRoot 'native-candidate/source.cjs'
    & $commands['node'] $sourceScript $repo $metadata (Join-Path $output 'source-before.json')
    if ($LASTEXITCODE -ne 0) { throw 'Native source preflight failed.' }
    $arguments = @('build', '--manifest-path', 'native/Cargo.toml', '--locked', '--offline', '--release', '--bin', 'otter-native', '--target', 'x86_64-pc-windows-msvc')
    Write-Output "Building $($version.buildVersion); log will be retained in $output"
    Invoke-NativeCandidateTool -Executable $commands['cargo'] -Arguments $arguments -LogPath (Join-Path $output 'build.log')
    & $commands['node'] $sourceScript $repo $metadata (Join-Path $output 'source-after.json')
    if ($LASTEXITCODE -ne 0) { throw 'Native source postflight failed.' }
    $afterCommit = (@(& $commands['git'] rev-parse HEAD) -join '').Trim()
    if ($LASTEXITCODE -ne 0 -or $afterCommit -ne $commit) { throw 'Source commit changed during build.' }
    Write-NativeCandidateJson -Path (Join-Path $output 'tools-after.json') -Value (Get-NativeCandidateTools -RepoRoot $repo -Commands $commands)
    Write-NativeCandidateJson -Path (Join-Path $output 'build.json') -Value ([ordered]@{
        channel = 'internal'; profile = 'release'; target = 'x86_64-pc-windows-msvc'; exitCode = 0
        arguments = $arguments; gitCommit = $commit; gitDirty = [bool]($dirtyLines.Count -gt 0)
    })
    $exe = Join-Path $target 'x86_64-pc-windows-msvc/release/otter-native.exe'
    [void](Assert-OtterAbsolutePathNoReparsePoints -Path $exe -TrustedRootPath $target)
    Copy-Item -LiteralPath $exe -Destination (Join-Path $output 'payload/otter-native.exe')
    & $commands['node'] (Join-Path $PSScriptRoot 'native-candidate/provenance.cjs') create $output
    if ($LASTEXITCODE -ne 0) { throw 'Native candidate identity verification failed.' }
} catch {
    if (Test-Path -LiteralPath $output -PathType Container) {
        [IO.File]::WriteAllText((Join-Path $output 'FAILED.txt'), $_.ToString(), [Text.UTF8Encoding]::new($false))
    }
    throw
} finally {
    if ($null -ne $cacheLock) { $cacheLock.Dispose() }
    $env:CARGO_TARGET_DIR = $oldTarget
    $env:OTTER_NATIVE_RC = $oldResource
    Pop-Location
}
