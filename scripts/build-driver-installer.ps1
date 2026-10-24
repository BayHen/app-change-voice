[CmdletBinding()]
param(
    [string]$SignedDriverDirectory = $env:VOICEMORPH_SIGNED_DRIVER_DIR,
    [string]$ArtifactDirectory
)

$ErrorActionPreference = 'Stop'
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if ([string]::IsNullOrWhiteSpace($ArtifactDirectory)) {
    $ArtifactDirectory = Join-Path $repoRoot 'release-artifacts\windows-x64'
}

& (Join-Path $PSScriptRoot 'prepare-driver-release.ps1') -SourceDirectory $SignedDriverDirectory
if ($LASTEXITCODE -ne 0) { throw 'Driver release staging failed.' }

Push-Location $repoRoot
try {
    npm.cmd run version:check
    if ($LASTEXITCODE -ne 0) { throw 'Version alignment check failed.' }
    npm.cmd exec tauri build -- --target x86_64-pc-windows-msvc --bundles nsis --config src-tauri/tauri.release.conf.json --config src-tauri/tauri.driver-release.conf.json
    if ($LASTEXITCODE -ne 0) { throw 'Tauri NSIS build failed.' }
} finally {
    Pop-Location
}

$bundleDirectory = Join-Path $repoRoot 'src-tauri\target\x86_64-pc-windows-msvc\release\bundle\nsis'
$installers = Get-ChildItem -LiteralPath $bundleDirectory -Filter '*-setup.exe' -File
if (-not $installers) { throw "No NSIS installer was produced under $bundleDirectory" }

$artifactRoot = [IO.Path]::GetFullPath($ArtifactDirectory)
New-Item -ItemType Directory -Path $artifactRoot -Force | Out-Null
$entries = foreach ($installer in $installers) {
    $target = Join-Path $artifactRoot $installer.Name
    Copy-Item -LiteralPath $installer.FullName -Destination $target -Force
    $copied = Get-Item -LiteralPath $target
    [ordered]@{
        name = $copied.Name
        size = $copied.Length
        sha256 = (Get-FileHash -LiteralPath $copied.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}
$version = (Get-Content -LiteralPath (Join-Path $repoRoot 'package.json') -Raw | ConvertFrom-Json).version
$manifest = [ordered]@{
    schemaVersion = 1
    application = 'VoiceMorph Studio'
    applicationVersion = $version
    architecture = 'x64'
    driverVersion = (Get-Content -LiteralPath (Join-Path $repoRoot 'drivers\voicemorph-virtual-audio\driver-package.json') -Raw | ConvertFrom-Json).driverVersion
    createdAtUtc = [DateTime]::UtcNow.ToString('o')
    artifacts = @($entries)
}
$manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $artifactRoot 'release-artifact-manifest.json') -Encoding utf8
$manifest
