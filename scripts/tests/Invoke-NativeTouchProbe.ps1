# 显式交互测试：普通隐藏终端的启动方式不能证明窗口真实可命中。
[CmdletBinding()]
param(
    [switch]$RunInteractive,
    [ValidateSet('touch', 'text-selection', 'text-double-click')]
    [string]$Probe = 'touch',
    [string]$EvidenceRoot = 'C:/Users/Public/nas_home/AI/GameEditor/linshi'
)
$ErrorActionPreference = 'Stop'
if (-not $RunInteractive) {
    throw 'Pass -RunInteractive to show the owned probe window and inject system input into it.'
}
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../..'))
if ($repo.StartsWith('\\')) {
    throw 'Run this script through the existing mapped drive or a local repository path.'
}
$evidence = Join-Path ([IO.Path]::GetFullPath($EvidenceRoot)) (
    "otter-native-$Probe-" + (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
$null = New-Item -ItemType Directory -Path $evidence
$oldTarget = $env:CARGO_TARGET_DIR
$oldProbe = $env:OTTER_NATIVE_TOUCH_PROBE
$oldTextProbe = $env:OTTER_NATIVE_TEXT_SELECTION_PROBE
$filter = switch ($Probe) {
    'touch' { 'surface_slider_system_touch_probe' }
    'text-selection' { 'text_selection_system_mouse_probe' }
    'text-double-click' { 'text_selection_system_double_click_probe' }
}
$process = $null
Push-Location $repo
try {
    $env:CARGO_TARGET_DIR = Join-Path ([IO.Path]::GetFullPath($EvidenceRoot)) 'otter-native-target'
    if ($Probe -eq 'touch') { $env:OTTER_NATIVE_TOUCH_PROBE = '1' }
    else { $env:OTTER_NATIVE_TEXT_SELECTION_PROBE = '1' }
    # 由 Cargo 的实际构建结果定位测试 EXE，不猜测 hash 或挑选历史目录中最新的文件。
    $artifacts = @(& rtk proxy cargo test --manifest-path native/Cargo.toml --locked --offline --lib --no-run --message-format=json |
        ForEach-Object {
            $item = $_ | ConvertFrom-Json
            if ($item.reason -eq 'compiler-message' -and $item.message.rendered) {
                [Console]::Error.Write($item.message.rendered)
            }
            if ($item.reason -eq 'compiler-artifact' -and $item.profile.test -and
                $item.target.name -eq 'otter_native' -and $item.executable) {
                $item.executable
            }
        })
    if ($LASTEXITCODE -ne 0 -or $artifacts.Count -ne 1) {
        throw 'Cargo did not produce exactly one current native library test executable.'
    }
    $stdout = Join-Path $evidence 'stdout.log'
    $stderr = Join-Path $evidence 'stderr.log'
    $hash = (Get-FileHash -LiteralPath $artifacts[0] -Algorithm SHA256).Hash
    # 此门明确需要可见窗口；保持命中保护，不能用隐藏启动结果冒充触摸验收。
    $process = Start-Process -FilePath $artifacts[0] -WorkingDirectory $repo -WindowStyle Normal -PassThru `
        -ArgumentList "$filter --ignored --nocapture --test-threads=1" `
        -RedirectStandardOutput $stdout -RedirectStandardError $stderr
    $null = $process.Handle
    if (-not $process.WaitForExit(30000)) {
        # 只终止持有句柄的自启子进程，不按名字清理用户窗口或其他测试。
        $process.Kill()
        if (-not $process.WaitForExit(5000)) { throw "Owned probe did not exit after termination. Evidence: $evidence" }
        throw "Native $Probe probe timed out. Evidence: $evidence"
    }
    $code = $process.ExitCode
    $receipt = @{
        executable = $artifacts[0]
        sha256 = $hash
        exitCode = $code
        probe = $Probe
        scope = 'System injection probe, not full input parity or physical-device acceptance'
        stdout = $stdout
        stderr = $stderr
    }
    [IO.File]::WriteAllText((Join-Path $evidence 'receipt.json'), ($receipt | ConvertTo-Json),
        (New-Object Text.UTF8Encoding($false)))
    Write-Output "Native $Probe probe exit=$code; evidence=$evidence"
    if ($code -ne 0) { throw "Native $Probe probe failed. See $stderr" }
} finally {
    if ($null -ne $process) {
        try {
            if (-not $process.HasExited) {
                $process.Kill()
                if (-not $process.WaitForExit(5000)) { throw "Owned probe remains alive: $($process.Id)" }
            }
        } finally { $process.Dispose() }
    }
    $env:CARGO_TARGET_DIR = $oldTarget
    $env:OTTER_NATIVE_TOUCH_PROBE = $oldProbe
    $env:OTTER_NATIVE_TEXT_SELECTION_PROBE = $oldTextProbe
    Pop-Location
}
