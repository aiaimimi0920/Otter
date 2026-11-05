param([Parameter(Mandatory = $true)][string]$OutputDirectory)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
. (Join-Path $repo 'scripts/release/PathSafety.ps1')
. (Join-Path $repo 'scripts/native-candidate/Tools.ps1')
[void][IO.Directory]::CreateDirectory($OutputDirectory)
$node = (Get-Command node -CommandType Application | Select-Object -First 1).Source
Invoke-NativeCandidateTool -Executable $node -Arguments @('-e', "process.stderr.write('normal stderr'); process.exit(0)") -LogPath (Join-Path $OutputDirectory 'success.log')
$count = 1
foreach ($case in @(
    @{ Name = 'nonzero'; Executable = $node; Arguments = @('-e', 'process.exit(7)') },
    @{ Name = 'missing'; Executable = (Join-Path $OutputDirectory 'does-not-exist.exe'); Arguments = @() }
)) {
    $LASTEXITCODE = 0
    $rejected = $false
    try {
        Invoke-NativeCandidateTool -Executable $case.Executable -Arguments $case.Arguments -LogPath (Join-Path $OutputDirectory ($case.Name + '.log'))
    } catch { $rejected = $true }
    if (-not $rejected) { throw "Expected command rejection: $($case.Name)" }
    $count++
}
$target = Join-Path $OutputDirectory ('cache-' + [Guid]::NewGuid().ToString('N'))
$first = Open-NativeCandidateCacheLock -TargetDirectory $target
try {
    $second = $null
    $rejected = $false
    try { $second = Open-NativeCandidateCacheLock -TargetDirectory $target } catch { $rejected = $true }
    finally { if ($null -ne $second) { $second.Dispose() } }
    if (-not $rejected) { throw 'Concurrent cache owner was not rejected' }
    $count++
} finally { $first.Dispose() }
$after = Open-NativeCandidateCacheLock -TargetDirectory $target
$after.Dispose()
$count++
Write-Output "Native candidate tool contracts: $count passed"
