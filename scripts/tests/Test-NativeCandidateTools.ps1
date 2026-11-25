param([Parameter(Mandatory = $true)][string]$OutputDirectory)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
. (Join-Path $repo 'scripts/release/PathSafety.ps1')
. (Join-Path $repo 'scripts/native-candidate/Tools.ps1')
if (Test-Path -LiteralPath $OutputDirectory) { throw 'Test output must be fresh' }
[void][IO.Directory]::CreateDirectory($OutputDirectory)
$node = (Get-Command node -CommandType Application | Select-Object -First 1).Source
$successLog = Join-Path $OutputDirectory 'success.log'
Invoke-NativeCandidateTool -Executable $node -Arguments @('-e', "process.stderr.write('normal stderr'); process.exit(0)") -LogPath $successLog
if ([IO.File]::ReadAllText($successLog) -cne "normal stderr`n") { throw 'Successful stderr was not retained verbatim' }
$count = 1
$streamLog = Join-Path $OutputDirectory 'stream.log'
$streamProbe = @'
const fs = require('fs');
const log = process.argv[1];
process.stdout.write(('x'.repeat(300) + '\n').repeat(128));
const deadline = Date.now() + 10000;
const timer = setInterval(() => {
  if (fs.existsSync(log) && fs.statSync(log).size > 4096) {
    clearInterval(timer);
    process.stderr.write('child saw log before exit\n');
  } else if (Date.now() >= deadline) {
    clearInterval(timer);
    process.exitCode = 22;
  }
}, 25);
'@
Invoke-NativeCandidateTool -Executable $node -Arguments @('-e', $streamProbe, '--', $streamLog) -LogPath $streamLog
$prefix = ((('x' * 300) + "`n") * 128)
if ([IO.File]::ReadAllText($streamLog) -cne ($prefix + "child saw log before exit`n") -or
    [IO.File]::ReadAllBytes($streamLog)[0] -ne 120) { throw 'Streamed log changed records or added a BOM' }
$count++

$blocked = $false
try {
    Invoke-NativeCandidateTool -Executable $node -Arguments @('-e', "process.exit(99)") -LogPath $successLog
} catch {
    if ($_.Exception.InnerException -isnot [IO.IOException]) { throw }
    $blocked = $true
}
if (-not $blocked -or [IO.File]::ReadAllText($successLog) -cne "normal stderr`n") {
    throw 'Existing log was not preserved'
}
$count++
foreach ($case in @(
    @{ Name = 'nonzero'; Code = '7'; Executable = $node; Arguments = @('-e', "process.stderr.write('failed stderr'); process.exit(7)") },
    @{ Name = 'missing'; Code = ''; Executable = (Join-Path $OutputDirectory 'does-not-exist.exe'); Arguments = @() }
)) {
    $LASTEXITCODE = 0
    $rejected = $false
    $log = Join-Path $OutputDirectory ($case.Name + '.log')
    try {
        Invoke-NativeCandidateTool -Executable $case.Executable -Arguments $case.Arguments -LogPath $log
    } catch {
        if (-not $_.ToString().StartsWith("Native candidate command failed ($($case.Code));")) { throw }
        $rejected = $true
    }
    $expected = if ($case.Name -eq 'nonzero') { 'failed stderr' } else { 'does-not-exist.exe' }
    if (-not $rejected -or -not [IO.File]::ReadAllText($log).Contains($expected)) {
        throw "Expected command rejection with log: $($case.Name)"
    }
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
