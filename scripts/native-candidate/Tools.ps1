# Build-tool and external-configuration identity; record only environment digests, never tokens or configuration contents.
function Write-NativeCandidateJson {
    param([string]$Path, $Value)
    if (Test-Path -LiteralPath $Path) { throw "Candidate record already exists: $Path" }
    [IO.File]::WriteAllText($Path, ($Value | ConvertTo-Json -Depth 30) + "`n", [Text.UTF8Encoding]::new($false))
}
function Invoke-NativeCandidateTool {
    param([string]$Executable, [string[]]$Arguments, [string]$LogPath)
    $previous = $ErrorActionPreference
    # Acquire the evidence file before launching and stream each merged record.
    $log = [IO.File]::Open($LogPath, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::Read)
    try {
        $writer = [IO.StreamWriter]::new($log, [Text.UTF8Encoding]::new($false), 4096, $true)
        try {
            $writer.NewLine = "`n"
            $wroteRecord = $false
            $ErrorActionPreference = 'Continue' # Windows PowerShell wraps ordinary stderr in ErrorRecord.
            $global:LASTEXITCODE = $null # Native commands update the global exit code; a same-named local variable would shadow it.
            try {
                & $Executable @Arguments 2>&1 | ForEach-Object -ErrorAction Stop {
                    $writer.WriteLine($_.ToString())
                    $wroteRecord = $true
                }
            } catch [Management.Automation.CommandNotFoundException] {
                # A launch failure can terminate the pipeline before it emits a record.
                $writer.WriteLine($_.ToString())
                $wroteRecord = $true
            }
            $code = $global:LASTEXITCODE
            if (-not $wroteRecord) { $writer.WriteLine() }
        } finally {
            $ErrorActionPreference = $previous
            $writer.Dispose()
        }
    } finally { $log.Dispose() }
    if ($null -eq $code -or $code -ne 0) { throw "Native candidate command failed ($code); see $LogPath" }
}
# The cache lock covers the interval from Cargo exit through artifact copying; callers must release it in finally.
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
