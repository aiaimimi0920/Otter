param([Parameter(Mandatory = $true)][string]$OutputDirectory)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (Test-Path -LiteralPath $OutputDirectory) { throw 'Test output must be fresh' }
[void][IO.Directory]::CreateDirectory($OutputDirectory)
$importer = Join-Path (Split-Path -Parent $PSScriptRoot) 'loom-connection/Import-LoomConnection.ps1'
$encoding = [Text.UTF8Encoding]::new($false, $true)
$key = [Security.Cryptography.RSACng]::new(2048)
try {
    $request = [Security.Cryptography.X509Certificates.CertificateRequest]::new(
        'CN=Otter import fixture', $key, [Security.Cryptography.HashAlgorithmName]::SHA256,
        [Security.Cryptography.RSASignaturePadding]::Pkcs1)
    $now = [DateTimeOffset]::UtcNow
    $certificate = $request.CreateSelfSigned($now.AddMinutes(-1), $now.AddHours(1))
    try { $public = [Convert]::ToBase64String($certificate.RawData) } finally { $certificate.Dispose() }
} finally { $key.Dispose() }
$fixture = [ordered]@{
    schemaVersion = 1
    origin = 'https://fixture.invalid:8766'
    loomId = 'loom-' + ('a' * 64)
    certificatePem = "-----BEGIN CERTIFICATE-----`n$public`n-----END CERTIFICATE-----`n"
}
$inputPath = Join-Path $OutputDirectory 'connection.json'
$profile = Join-Path $OutputDirectory 'profile'
$json = $fixture | ConvertTo-Json -Compress
$exact = $json.PadRight(40KB)
[IO.File]::WriteAllText($inputPath, $exact, $encoding)
& $importer -ConnectionFile $inputPath -ProfileDirectory $profile
$record = Join-Path $profile 'loom-remote.json'
$autoconnect = Get-Content -LiteralPath (Join-Path $profile 'loom-autoconnect-paused.json') -Raw | ConvertFrom-Json
if ((Get-Item -LiteralPath $record).Length -ne 40KB -or [IO.File]::ReadAllText($record, $encoding) -cne $exact -or
    $autoconnect.paused -ne $false -or $autoconnect.origin -cne $fixture.origin) { throw 'Import evidence differs' }
$before = (Get-FileHash -LiteralPath $record).Hash
foreach ($case in @(
    @{ bytes = $encoding.GetBytes($exact + ' '); error = 'Connection file is too large' },
    @{ bytes = [byte[]]@(0xff, 0xff); error = 'GetString' },
    @{ bytes = $encoding.GetBytes($json.Replace($fixture.loomId, $fixture.loomId.ToUpperInvariant())); error = 'Invalid Loom origin or identity' }
)) {
    [IO.File]::WriteAllBytes($inputPath, $case.bytes)
    try {
        & $importer -ConnectionFile $inputPath -ProfileDirectory $profile
        throw 'Invalid connection fixture was accepted'
    } catch {
        if (-not $_.ToString().Contains($case.error)) { throw }
    }
    if ((Get-FileHash -LiteralPath $record).Hash -ne $before -or @(Get-ChildItem -LiteralPath $profile -Force).Count -ne 2) {
        throw 'Rejected input changed the profile'
    }
}
Write-Output 'Loom connection import contracts: 4 passed'
