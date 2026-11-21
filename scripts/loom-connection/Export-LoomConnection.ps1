# Export public LAN connection data only; the local administrator token is never read.
[CmdletBinding()]
param([string]$LocalLoom = 'http://127.0.0.1:8765', [Parameter(Mandatory = $true)][string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$uri = [Uri]$LocalLoom
if ($uri.Scheme -ne 'http' -or $uri.Host -notin @('127.0.0.1', '[::1]') -or $uri.UserInfo -or $uri.Query -or $uri.Fragment -or $uri.AbsolutePath -ne '/') { throw 'LocalLoom must be a numeric loopback HTTP origin' }
$info = Invoke-RestMethod -Uri ($LocalLoom.TrimEnd('/') + '/v1/otter/identity') -TimeoutSec 10
$endpoint = [Uri]$info.endpoint
if ($info.protocolVersion -ne 'loom.connection.v1' -or -not $info.discoverable -or $endpoint.Scheme -ne 'https' -or $endpoint.UserInfo -or $endpoint.Query -or $endpoint.Fragment -or $endpoint.AbsolutePath -ne '/') { throw 'Invalid Loom LAN identity' }
$der = [Convert]::FromBase64String($info.certificate)
$certificate = [Security.Cryptography.X509Certificates.X509Certificate2]::new($der)
try {
    if ($certificate.HasPrivateKey -or $der.Length -gt 8192) { throw 'Invalid public certificate' }
    $pem = "-----BEGIN CERTIFICATE-----`n" + [Convert]::ToBase64String($der, [Base64FormattingOptions]::InsertLineBreaks).Replace("`r`n", "`n") + "`n-----END CERTIFICATE-----`n"
} finally { $certificate.Dispose() }
$output = [IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $output) { throw 'Output directory already exists' }
[IO.Directory]::CreateDirectory($output) | Out-Null
$document = [ordered]@{ schemaVersion = 1; origin = $endpoint.GetLeftPart([UriPartial]::Authority); loomId = [string]$info.loomId; certificatePem = $pem }
[IO.File]::WriteAllText((Join-Path $output 'loom-remote.json'), ($document | ConvertTo-Json), [Text.UTF8Encoding]::new($false))
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'Import-LoomConnection.ps1') -Destination $output
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'Connect-Otter.cmd') -Destination $output
Write-Output "Public Loom connection exported: $output"
Write-Output "Origin: $($document.origin)"
