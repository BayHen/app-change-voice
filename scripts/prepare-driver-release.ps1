[CmdletBinding()]
param(
    [string]$SourceDirectory = $env:VOICEMORPH_SIGNED_DRIVER_DIR,
    [string]$DestinationDirectory,
    [switch]$ValidateOnly
)

$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$manifestPath = Join-Path $repoRoot 'drivers\voicemorph-virtual-audio\driver-package.json'
$manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding utf8 | ConvertFrom-Json

if ([string]::IsNullOrWhiteSpace($SourceDirectory)) {
    throw 'Set VOICEMORPH_SIGNED_DRIVER_DIR or pass -SourceDirectory with a Microsoft-signed production driver package.'
}
$source = [IO.Path]::GetFullPath($SourceDirectory)
if (-not (Test-Path -LiteralPath $source -PathType Container)) {
    throw "Signed driver source directory does not exist: $source"
}

$required = @(
    $manifest.outputs.driver,
    $manifest.outputs.inf,
    $manifest.outputs.catalog
)
$paths = @{}
foreach ($name in $required) {
    $candidate = Join-Path $source $name
    if (-not (Test-Path -LiteralPath $candidate -PathType Leaf)) {
        throw "Signed driver package is missing required file: $name"
    }
    $paths[$name] = $candidate
}

$infText = Get-Content -LiteralPath $paths[$manifest.outputs.inf] -Raw -Encoding utf8
$expectedVersion = [string]$manifest.driverVersion
if ($infText -notmatch "(?im)^DriverVer\s*=\s*[^,]+,$([regex]::Escape($expectedVersion))\s*$") {
    throw "Driver INF version does not match manifest version $expectedVersion."
}
if ($infText -notmatch "(?im)^CatalogFile\s*=\s*$([regex]::Escape([string]$manifest.outputs.catalog))\s*$") {
    throw 'Driver INF does not reference the expected catalog.'
}
foreach ($identityToken in @(
    'ProviderName="VoiceMorph Studio"',
    'DeviceDescription="VoiceMorph Virtual Microphone"',
    'ROOT\VoiceMorphVirtualMicrophone'
)) {
    if (-not $infText.Contains($identityToken)) { throw "Driver INF identity mismatch: $identityToken" }
}
$driverFileVersion = (Get-Item -LiteralPath $paths[$manifest.outputs.driver]).VersionInfo.FileVersion
if ($driverFileVersion -ne $expectedVersion) {
    throw "Driver SYS version $driverFileVersion does not match manifest version $expectedVersion."
}

$catalogSignature = Get-AuthenticodeSignature -LiteralPath $paths[$manifest.outputs.catalog]
$driverSignature = Get-AuthenticodeSignature -LiteralPath $paths[$manifest.outputs.driver]
foreach ($signature in @($catalogSignature, $driverSignature)) {
    if ($signature.Status -ne 'Valid') {
        throw "Production driver signature validation failed for $($signature.Path): $($signature.Status)"
    }
    $subject = $signature.SignerCertificate.Subject
    if ($subject -notmatch 'Microsoft') {
        throw "Production driver must carry a Microsoft signature: $($signature.Path)"
    }
}

$signtool = Get-ChildItem 'C:\Program Files (x86)\Windows Kits\10\bin' -Filter signtool.exe -Recurse -File |
    Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
    Sort-Object FullName -Descending |
    Select-Object -First 1
if (-not $signtool) { throw 'SignTool x64 was not found in the Windows SDK.' }

& $signtool.FullName verify /kp /v /c $paths[$manifest.outputs.catalog] $paths[$manifest.outputs.inf]
if ($LASTEXITCODE -ne 0) { throw 'Kernel-policy catalog verification failed.' }
& $signtool.FullName verify /kp /v $paths[$manifest.outputs.driver]
if ($LASTEXITCODE -ne 0) { throw 'Kernel-policy SYS verification failed.' }

$fileEntries = foreach ($name in $required) {
    $item = Get-Item -LiteralPath $paths[$name]
    [ordered]@{
        name = $name
        size = $item.Length
        sha256 = (Get-FileHash -LiteralPath $item.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}
$releaseManifest = [ordered]@{
    schemaVersion = 1
    packageName = $manifest.name
    provider = $manifest.provider
    architecture = $manifest.architecture
    driverVersion = $manifest.driverVersion
    hardwareId = $manifest.hardwareId
    signed = $true
    files = @($fileEntries)
}

if ($ValidateOnly) {
    $releaseManifest | ConvertTo-Json -Depth 5
    return
}

if ([string]::IsNullOrWhiteSpace($DestinationDirectory)) {
    $DestinationDirectory = Join-Path $repoRoot 'src-tauri\driver-package'
}
$destination = [IO.Path]::GetFullPath($DestinationDirectory)
$allowedDestination = [IO.Path]::GetFullPath((Join-Path $repoRoot 'src-tauri\driver-package'))
if (-not $destination.Equals($allowedDestination, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Release staging is restricted to: $allowedDestination"
}
if (Test-Path -LiteralPath $destination) {
    Remove-Item -LiteralPath $destination -Recurse -Force
}
New-Item -ItemType Directory -Path $destination -Force | Out-Null
foreach ($name in $required) {
    Copy-Item -LiteralPath $paths[$name] -Destination (Join-Path $destination $name)
}
$releaseManifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $destination 'driver-release-manifest.json') -Encoding utf8

[pscustomobject]@{
    Status = 'READY'
    DriverVersion = $manifest.driverVersion
    StagingDirectory = $destination
    Signed = $true
}
