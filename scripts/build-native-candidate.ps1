[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$OutputDirectory,
    [Parameter(Mandatory = $true)][string]$TargetDirectory,
    [switch]$PublicRelease
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
. (Join-Path $PSScriptRoot 'file-hash.ps1')
. (Join-Path $PSScriptRoot 'release/PathSafety.ps1')
. (Join-Path $PSScriptRoot 'version-identity.ps1')
. (Join-Path $PSScriptRoot 'native-candidate/Tools.ps1')
$version = Get-OtterVersionIdentity -RepoRoot $repo -PublicRelease:$PublicRelease
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
    if ($PublicRelease) {
        if ($dirtyLines.Count -or $version.internalRevision -ne 0 -or $env:GITHUB_SHA -cne $commit -or
            $env:GITHUB_REPOSITORY -cne 'aiaimimi0920/Otter' -or $env:GITHUB_REF -cne "refs/tags/$($version.buildVersion)" -or
            $env:GITHUB_RUN_ID -cnotmatch '^[1-9]\d*$' -or $env:GITHUB_RUN_ATTEMPT -cnotmatch '^[1-9]\d*$') {
            throw 'Public portable build requires a clean, exact release tag on the official Actions runner.'
        }
        & (Join-Path $PSScriptRoot 'assert-release-version.ps1') -Tag $version.buildVersion -RequireTagAtHead -RequireReachableFromBranch origin/main
    }
    New-Item -ItemType Directory -Path $output | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $output 'payload') | Out-Null
    Write-NativeCandidateJson -Path (Join-Path $output 'tools-before.json') -Value $tools
    $metadata = Join-Path $output 'metadata.json'
    $data = @(& $commands['cargo'] metadata --manifest-path native/Cargo.toml --format-version 1 --locked --offline)
    if ($LASTEXITCODE -ne 0) { throw 'Native offline metadata failed.' }
    [IO.File]::WriteAllText($metadata, ($data -join "`n"), [Text.UTF8Encoding]::new($false))
    $sourceScript = Join-Path $PSScriptRoot 'native-candidate/source.cjs'
    & $commands['node'] $sourceScript $repo $metadata (Join-Path $output 'source-before.json') $version.channel
    if ($LASTEXITCODE -ne 0) { throw 'Native source preflight failed.' }
    $arguments = @('build', '--manifest-path', 'native/Cargo.toml', '--locked', '--offline', '--release', '-p', 'otter-native', '-p', 'otter-live-hover-hook', '--lib', '--bin', 'otter-native', '--target', 'x86_64-pc-windows-msvc')
    Write-Output "Building $($version.buildVersion); log will be retained in $output"
    Invoke-NativeCandidateTool -Executable $commands['cargo'] -Arguments $arguments -LogPath (Join-Path $output 'build.log')
    & $commands['node'] $sourceScript $repo $metadata (Join-Path $output 'source-after.json') $version.channel
    if ($LASTEXITCODE -ne 0) { throw 'Native source postflight failed.' }
    $afterCommit = (@(& $commands['git'] rev-parse HEAD) -join '').Trim()
    if ($LASTEXITCODE -ne 0 -or $afterCommit -ne $commit) { throw 'Source commit changed during build.' }
    Write-NativeCandidateJson -Path (Join-Path $output 'tools-after.json') -Value (Get-NativeCandidateTools -RepoRoot $repo -Commands $commands)
    $build = [ordered]@{
        channel = $version.channel; profile = 'release'; target = 'x86_64-pc-windows-msvc'; exitCode = 0
        arguments = $arguments; gitCommit = $commit; gitDirty = [bool]($dirtyLines.Count -gt 0)
    }
    if ($PublicRelease) {
        $changes = @(& $commands['git'] status --porcelain --untracked-files=all)
        if ($LASTEXITCODE -ne 0 -or $changes.Count) { throw 'Public build changed the checkout.' }
        $build.releaseSource = [ordered]@{ repository = $env:GITHUB_REPOSITORY; tag = $version.buildVersion; runId = $env:GITHUB_RUN_ID; attempt = $env:GITHUB_RUN_ATTEMPT }
    }
    Write-NativeCandidateJson -Path (Join-Path $output 'build.json') -Value $build
    $exe = Join-Path $target 'x86_64-pc-windows-msvc/release/otter-native.exe'
    [void](Assert-OtterAbsolutePathNoReparsePoints -Path $exe -TrustedRootPath $target)
    Copy-Item -LiteralPath $exe -Destination (Join-Path $output 'payload/otter-native.exe')
    $hook = Join-Path $target 'x86_64-pc-windows-msvc/release/otter_live_hover_hook.dll'
    [void](Assert-OtterAbsolutePathNoReparsePoints -Path $hook -TrustedRootPath $target)
    Copy-Item -LiteralPath $hook -Destination (Join-Path $output 'payload/otter_live_hover_hook.dll')
    & $commands['node'] (Join-Path $PSScriptRoot 'native-candidate/provenance.cjs') create $output $version.channel
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
