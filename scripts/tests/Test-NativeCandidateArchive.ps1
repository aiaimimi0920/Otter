param(
    [Parameter(Mandatory = $true)][string]$ZipPath,
    [Parameter(Mandatory = $true)][string]$OutputDirectory
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
. (Join-Path $repo 'scripts/file-hash.ps1')
. (Join-Path $repo 'scripts/release/PathSafety.ps1')
. (Join-Path $repo 'scripts/native-candidate/Archive.ps1')
Add-Type -AssemblyName System.IO.Compression, System.IO.Compression.FileSystem
if (Test-Path -LiteralPath $OutputDirectory) { throw 'Test output must be fresh' }
[void][IO.Directory]::CreateDirectory($OutputDirectory)
$baseline = Test-NativeCandidateArchive -ZipPath $ZipPath
if ($baseline.files -lt 5) { throw 'Expected populated archive fixture' }
$passed = 1

# Supply synthetic entry streams to test exact-length handling independently
# of differences between Windows and newer .NET ZIP readers.
function New-EntryStreamFixture {
    param([long]$Length, [byte[]]$Bytes)
    $entry = [pscustomobject]@{ Length = $Length; FullName = 'entry.txt'; Stream = [IO.MemoryStream]::new($Bytes) }
    $entry | Add-Member -MemberType ScriptMethod -Name Open -Value { return $this.Stream }
    $index = [Collections.Generic.Dictionary[string,object]]::new([StringComparer]::Ordinal)
    $index.Add($entry.FullName, $entry)
    return ,$index
}
$validInputs = @('', ('x' * 65536 + [char]0x00e9))
foreach ($expected in $validInputs) {
    $bytes = [Text.Encoding]::UTF8.GetBytes($expected)
    $textIndex = New-EntryStreamFixture -Length $bytes.Length -Bytes $bytes
    $actual = Read-OtterArchiveEntryText -Index $textIndex -EntryName 'entry.txt'
    if ($actual -cne $expected -or $textIndex['entry.txt'].Stream.CanRead) { throw 'Exact text read failed' }
    $digestIndex = New-EntryStreamFixture -Length $bytes.Length -Bytes $bytes
    $digest = Get-OtterArchiveEntryDigest -Index $digestIndex -EntryName 'entry.txt'
    $sha = [Security.Cryptography.SHA256]::Create()
    try { $expectedDigest = ([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant() }
    finally { $sha.Dispose() }
    if ($digest.bytes -ne $bytes.Length -or $digest.sha256 -cne $expectedDigest -or
        $digestIndex['entry.txt'].Stream.CanRead) { throw 'Exact digest read failed' }
    $passed += 2
}
foreach ($reader in @('Read-OtterArchiveEntryText', 'Get-OtterArchiveEntryDigest')) {
    foreach ($case in @(
        @{ Length = 0; Actual = 1; Error = 'exceeds its declared length'; Opens = $true },
        @{ Length = 1; Actual = 2; Error = 'exceeds its declared length'; Opens = $true },
        @{ Length = 2; Actual = 1; Error = 'ended before its declared length'; Opens = $true },
        @{ Length = 3; Actual = 1; Error = 'Missing or oversized'; Opens = $false }
    )) {
        $index = New-EntryStreamFixture -Length $case.Length -Bytes ([byte[]]::new($case.Actual))
        $rejected = $false
        try { [void](& $reader -Index $index -EntryName 'entry.txt' -MaxBytes 2) }
        catch {
            if ($_.ToString() -notlike "*$($case.Error)*") { throw }
            $rejected = $true
        }
        $stream = $index['entry.txt'].Stream
        try {
            if (-not $rejected -or $stream.CanRead -eq $case.Opens) { throw "Invalid length handling: $reader" }
        } finally { $stream.Dispose() }
        $passed++
    }
}
$invalidUtf8 = New-EntryStreamFixture -Length 1 -Bytes ([byte[]]@(255))
$rejected = $false
try { [void](Read-OtterArchiveEntryText -Index $invalidUtf8 -EntryName 'entry.txt') }
catch {
    if ($_.Exception.InnerException -isnot [Text.DecoderFallbackException]) { throw }
    $rejected = $true
}
if (-not $rejected -or $invalidUtf8['entry.txt'].Stream.CanRead) { throw 'Invalid UTF-8 was not rejected and closed' }
$passed++

foreach ($case in @('tamper', 'missing', 'hook-missing', 'hook-tamper', 'extra', 'traversal', 'duplicate', 'false-string', 'second-exe', 'angle-egl', 'angle-gles')) {
    $copy = Join-Path $OutputDirectory ($case + '.zip')
    Copy-Item -LiteralPath $ZipPath -Destination $copy
    $archive = [IO.Compression.ZipFile]::Open($copy, [IO.Compression.ZipArchiveMode]::Update)
    try {
        switch ($case) {
            'missing' { $archive.GetEntry('README.txt').Delete(); break }
            'hook-missing' { $archive.GetEntry('otter_live_hover_hook.dll').Delete(); break }
            'hook-tamper' { $name = 'otter_live_hover_hook.dll'; $archive.GetEntry($name).Delete(); $text = 'wrong hook'; break }
            'tamper' { $name = 'otter-native.exe'; $archive.GetEntry($name).Delete(); $text = 'wrong executable'; break }
            'extra' { $name = 'extra.txt'; $text = 'extra'; break }
            'second-exe' { $name = 'nested/Capture-Helper.EXE'; $text = 'unexpected'; break }
            'traversal' { $name = '../escape.txt'; $text = 'escape'; break }
            'duplicate' { $name = 'README.TXT'; $text = 'duplicate'; break }
            'angle-egl' { $name = 'libEGL.dll'; $text = 'retired'; break }
            'angle-gles' { $name = 'nested/LIBGLESv2.DLL'; $text = 'retired'; break }
            'false-string' {
                $name = 'package-manifest.json'
                $entry = $archive.GetEntry($name)
                $reader = [IO.StreamReader]::new($entry.Open())
                try { $manifest = $reader.ReadToEnd() | ConvertFrom-Json } finally { $reader.Dispose() }
                $manifest.publishable = 'False'
                $text = $manifest | ConvertTo-Json -Depth 30
                $entry.Delete()
                break
            }
        }
        if ($case -notin @('missing', 'hook-missing')) {
            $entry = $archive.CreateEntry($name)
            $writer = [IO.StreamWriter]::new($entry.Open(), [Text.UTF8Encoding]::new($false))
            try { $writer.Write($text) } finally { $writer.Dispose() }
        }
    } finally { $archive.Dispose() }
    $rejected = $false
    try { [void](Test-NativeCandidateArchive -ZipPath $copy) } catch {
        if ($case -eq 'second-exe' -and $_.ToString() -notmatch 'Unexpected executable') { throw }
        if ($case.StartsWith('angle-') -and $_.ToString() -notmatch 'Retired ANGLE DLL') { throw }
        $rejected = $true
        [IO.File]::WriteAllText((Join-Path $OutputDirectory ($case + '.log')), $_.ToString(), [Text.UTF8Encoding]::new($false))
    }
    if (-not $rejected) { throw "Invalid ZIP accepted: $case" }
    $passed++
}
Write-Output "Native archive contracts: $passed passed"
