param([switch]$Child)
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:OTTER_CI_RUNNER_ENVIRONMENT -ne 'github-hosted' -or $env:RUNNER_OS -ne 'Windows') {
    throw 'Only disposable GitHub-hosted Windows VMs may use this fixture'
}
$root = Split-Path -Parent $PSScriptRoot
$tools = Join-Path $root '.tmp-ci-runner'
$admin = ([Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if ($Child) {
    if ($admin) { throw 'Native tests require a standard account' }
    $env:USERPROFILE = [Environment]::GetFolderPath('UserProfile')
    $env:LOCALAPPDATA = [Environment]::GetFolderPath('LocalApplicationData')
    $env:APPDATA = [Environment]::GetFolderPath('ApplicationData')
    $env:TEMP = $env:TMP = Join-Path $env:LOCALAPPDATA 'Temp'
    $env:PATH = "$tools;$env:PATH"
    Set-Location -LiteralPath $root
    python scripts/ci_hosted_foreground.py cargo nextest run --binaries-metadata "$tools/binaries.json" --cargo-metadata "$tools/cargo.json" --config-file native/.config/nextest.toml --profile default --test-threads 1 --no-tests=fail --fail-fast
    exit $LASTEXITCODE
}
if (!$admin) { throw 'Expected hosted provisioning identity' }
# Compile once with the pinned toolchain. Run the resulting binaries as a real
# ordinary user; do not disable UAC, substitute a worker, or omit token tests.
New-Item -ItemType Directory -Path $tools | Out-Null
$utf8 = [Text.UTF8Encoding]::new($false)
$metadata = @(cargo metadata --locked --offline --manifest-path native/Cargo.toml --format-version 1)
if ($LASTEXITCODE) { exit $LASTEXITCODE }
[IO.File]::WriteAllText("$tools/cargo.json", ($metadata -join "`n"), $utf8)
$bins = @(cargo nextest list --locked --offline --workspace --manifest-path native/Cargo.toml --list-type binaries-only --message-format json)
if ($LASTEXITCODE) { exit $LASTEXITCODE }
[IO.File]::WriteAllText("$tools/binaries.json", ($bins -join "`n"), $utf8)
$cargo = rustup which cargo
if ($LASTEXITCODE) { exit $LASTEXITCODE }
Copy-Item -LiteralPath $cargo -Destination "$tools/cargo.exe"
Copy-Item -LiteralPath (Get-Command cargo-nextest.exe).Source -Destination "$tools/cargo-nextest.exe"
$user = 'OtterTestUser'
$password = ConvertTo-SecureString ("T1!" + [guid]::NewGuid().ToString('N')) -AsPlainText -Force
$account = New-LocalUser -Name $user -Password $password
try {
    $sid = '*' + $account.SID.Value
    # No access to the runner's profile, credentials, or release toolchain.
    # Only these repository-owned test scratch/report directories are writable.
    foreach ($dir in '.tmp','native/target/nextest','native/target/test-fixtures') {
        $path = Join-Path $root $dir
        [IO.Directory]::CreateDirectory($path) | Out-Null
        icacls $path /grant "${sid}:(OI)(CI)M" | Out-Null
        if ($LASTEXITCODE) { exit $LASTEXITCODE }
    }
    $credentials = [PSCredential]::new("$env:COMPUTERNAME\$user", $password)
    $stdout = Join-Path $env:RUNNER_TEMP 'standard-tests.out'
    $stderr = Join-Path $env:RUNNER_TEMP 'standard-tests.err'
    $process = Start-Process -FilePath "$PSHOME/powershell.exe" -ArgumentList "-NoProfile -NonInteractive -ExecutionPolicy Bypass -File `"$PSCommandPath`" -Child" -Credential $credentials -LoadUserProfile -WorkingDirectory $root -WindowStyle Hidden -RedirectStandardOutput $stdout -RedirectStandardError $stderr -Wait -PassThru
    Get-Content -LiteralPath $stdout,$stderr
    if ($process.ExitCode -ne 0) { throw "Standard-user test gate failed: $($process.ExitCode)" }
} finally {
    Remove-LocalUser -SID $account.SID
    # Only remove the four files this launcher created, never recurse a fixture tree.
    foreach ($name in 'cargo.json','binaries.json','cargo.exe','cargo-nextest.exe') {
        Remove-Item -LiteralPath (Join-Path $tools $name) -Force
    }
    [IO.Directory]::Delete($tools)
}
