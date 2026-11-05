# 构建工具与外部配置身份；环境只记录摘要，不写入令牌或配置内容。
function Write-NativeCandidateJson {
    param([string]$Path, $Value)
    if (Test-Path -LiteralPath $Path) { throw "Candidate record already exists: $Path" }
    [IO.File]::WriteAllText($Path, ($Value | ConvertTo-Json -Depth 30) + "`n", [Text.UTF8Encoding]::new($false))
}
function Invoke-NativeCandidateTool {
    param([string]$Executable, [string[]]$Arguments, [string]$LogPath)
    $previous = $ErrorActionPreference
    try {
        $ErrorActionPreference = 'Continue' # Windows PowerShell 将正常 stderr 包装为 ErrorRecord。
        $global:LASTEXITCODE = $null # 原生命令更新全局退出码；局部同名变量会遮蔽它。
        $lines = @(& $Executable @Arguments 2>&1)
        $code = $global:LASTEXITCODE
    } finally { $ErrorActionPreference = $previous }
    [IO.File]::WriteAllText($LogPath, (($lines | ForEach-Object { $_.ToString() }) -join "`n") + "`n", [Text.UTF8Encoding]::new($false))
    if ($null -eq $code -or $code -ne 0) { throw "Native candidate command failed ($code); see $LogPath" }
}
# 缓存锁覆盖 Cargo 退出到产物复制的窗口；调用者必须在 finally 释放。
function Open-NativeCandidateCacheLock {
    param([string]$TargetDirectory)
    [void][IO.Directory]::CreateDirectory($TargetDirectory)
    $path = Join-Path $TargetDirectory '.otter-native-candidate.lock'
    [void](Assert-OtterAbsolutePathNoReparsePoints -Path $path -TrustedRootPath $TargetDirectory)
    return [IO.File]::Open($path, [IO.FileMode]::OpenOrCreate, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
}
function Get-NativeCandidateTools {
    param([string]$RepoRoot, $Commands)
    $records = [ordered]@{}
    foreach ($name in @('cargo', 'rustc', 'resource', 'node', 'git')) {
        $file = $Commands[$name]
        $records[$name] = [ordered]@{ path = $file; sha256 = Get-OtterFileSha256 -Path $file }
    }
    $records['rustcVersion'] = (@(& $Commands['rustc'] -vV) -join "`n")
    if ($LASTEXITCODE -ne 0 -or $records['rustcVersion'] -notmatch '(?m)^host: x86_64-pc-windows-msvc\s*$') {
        throw 'Native candidate requires the verified x86_64-pc-windows-msvc host toolchain.'
    }
    $records['cargoVersion'] = (@(& $Commands['cargo'] -V) -join "`n")
    if ($LASTEXITCODE -ne 0) { throw 'Cannot identify Cargo toolchain.' }
    $sha = [Security.Cryptography.SHA256]::Create()
    try {
        $environment = [ordered]@{}
        foreach ($entry in @(Get-ChildItem Env: | Sort-Object Name)) {
            if ($entry.Name -match '^(CARGO_|RUST|OTTER_NATIVE_RC$|PATH$|CC$|CXX$|AR$|CL$|LINK$|LIB$|INCLUDE$|SDK|VS|VCTools|WindowsSdk)' -and
                $entry.Name -notmatch '(?i)TOKEN|SECRET|PASSWORD|CREDENTIAL|AUTH|KEY') {
                $bytes = [Text.Encoding]::UTF8.GetBytes($entry.Value)
                $environment[$entry.Name] = ([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
            }
        }
        $records['environmentHashes'] = $environment
    } finally { $sha.Dispose() }
    $configs = [ordered]@{}
    $directories = [Collections.Generic.List[string]]::new()
    $directory = [IO.Path]::GetFullPath($RepoRoot)
    while ($directory) {
        $directories.Add((Join-Path $directory '.cargo'))
        $directory = Split-Path -Parent $directory
    }
    $cargoHome = if ($env:CARGO_HOME) { $env:CARGO_HOME } else { Join-Path $env:USERPROFILE '.cargo' }
    $directories.Add($cargoHome)
    foreach ($directory in $directories) {
        foreach ($name in @('config', 'config.toml')) {
            $file = Join-Path $directory $name
            if (Test-Path -LiteralPath $file -PathType Leaf) {
                [void](Assert-OtterAbsolutePathNoReparsePoints -Path $file -TrustedRootPath $directory)
                $configs[$file] = Get-OtterFileSha256 -Path $file
            }
        }
    }
    $records['externalCargoConfigHashes'] = $configs
    return $records
}
