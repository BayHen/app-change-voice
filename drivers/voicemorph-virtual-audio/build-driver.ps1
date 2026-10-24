[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$WindowsDriverSamplesRoot,
    [ValidateSet('Debug', 'Release')]
    [string]$Configuration = 'Debug',
    [switch]$PrepareOnly
)

$ErrorActionPreference = 'Stop'
$PinnedCommit = 'bb68ed5e770bce04dc5395ec9258dafab034184d'
$ProjectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$SourceRoot = Join-Path $WindowsDriverSamplesRoot 'audio\simpleaudiosample'
$WorkRoot = Join-Path $ProjectRoot 'obj\simpleaudiosample'
$OutputRoot = Join-Path $ProjectRoot "artifacts\$Configuration\x64"

if (-not (Test-Path (Join-Path $SourceRoot 'SimpleAudioSample.sln'))) {
    throw "Microsoft Simple Audio Sample was not found under: $SourceRoot"
}

$sourceGitRoot = (& git -c "safe.directory=$WindowsDriverSamplesRoot" -C $WindowsDriverSamplesRoot rev-parse --show-toplevel 2>$null).Trim()
$normalizedSamplesRoot = [IO.Path]::GetFullPath($WindowsDriverSamplesRoot).TrimEnd('\', '/')
$actualCommit = if ($sourceGitRoot -and [IO.Path]::GetFullPath($sourceGitRoot).TrimEnd('\', '/') -eq $normalizedSamplesRoot) {
    (& git -c "safe.directory=$WindowsDriverSamplesRoot" -C $WindowsDriverSamplesRoot rev-parse HEAD).Trim()
} elseif ((Split-Path -Leaf $normalizedSamplesRoot) -eq 'windows-driver-samples-bb68ed5e' -and (Test-Path (Join-Path $normalizedSamplesRoot 'LICENSE'))) {
    # The documented sparse archive has no .git metadata; its immutable directory name records the pinned revision.
    $PinnedCommit
} else {
    throw 'Windows Driver Samples source provenance could not be verified.'
}
if ($actualCommit -ne $PinnedCommit) {
    throw "Windows Driver Samples revision mismatch. Expected $PinnedCommit, got $actualCommit"
}

if (Test-Path $WorkRoot) {
    Remove-Item -LiteralPath $WorkRoot -Recurse -Force
}
New-Item -ItemType Directory -Path (Split-Path -Parent $WorkRoot) -Force | Out-Null
Copy-Item -LiteralPath $SourceRoot -Destination $WorkRoot -Recurse

$main = Join-Path $WorkRoot 'Source\Main'
Copy-Item -LiteralPath (Join-Path $ProjectRoot 'src\voicemorph_silence.h') -Destination $main
Copy-Item -LiteralPath (Join-Path $ProjectRoot 'src\voicemorph_silence.cpp') -Destination $main
Copy-Item -LiteralPath (Join-Path $ProjectRoot 'src\voicemorph_bridge_driver.h') -Destination $main
Copy-Item -LiteralPath (Join-Path $ProjectRoot 'src\voicemorph_bridge_driver.cpp') -Destination $main
Copy-Item -LiteralPath (Join-Path $ProjectRoot 'include\voicemorph_bridge_protocol.h') -Destination $main
Copy-Item -LiteralPath (Join-Path $ProjectRoot 'src\VoiceMorphVirtualAudio.rc') -Destination $main

$repositoryRoot = [IO.Path]::GetFullPath((& git -C $ProjectRoot rev-parse --show-toplevel).Trim()).TrimEnd('\', '/')
$normalizedWorkRoot = [IO.Path]::GetFullPath($WorkRoot)
$repositoryPrefix = $repositoryRoot.TrimEnd('\') + '\'
if (-not $normalizedWorkRoot.StartsWith($repositoryPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Prepared source directory must remain inside the repository.'
}
$relativeWorkRoot = $normalizedWorkRoot.Substring($repositoryPrefix.Length).Replace('\', '/')
& git -C $repositoryRoot apply --whitespace=error "--directory=$relativeWorkRoot" (Join-Path $ProjectRoot 'patches\0001-voicemorph-capture-only-silence.patch')
if ($LASTEXITCODE -ne 0) { throw 'VoiceMorph source overlay could not be applied.' }

Remove-Item -LiteralPath (Join-Path $main 'SimpleAudioSample.inx') -Force
Copy-Item -LiteralPath (Join-Path $ProjectRoot 'inf\VoiceMorphVirtualAudio.inx') -Destination $main

# WDK StampInf otherwise replaces DriverVer with a build-derived version. Keep
# INF, resource metadata and release manifest aligned for deterministic upgrades.
$packageManifest = Get-Content -LiteralPath (Join-Path $ProjectRoot 'driver-package.json') -Raw -Encoding utf8 | ConvertFrom-Json
$mainProjectPath = Join-Path $main 'Main.vcxproj'
$mainProject = Get-Content -LiteralPath $mainProjectPath -Raw -Encoding utf8
$infItem = '<Inf Exclude="@(Inx)" Include="*.inx" />'
$versionedInfItem = @"
<Inf Exclude="@(Inx)" Include="*.inx">
      <DateStamp>$($packageManifest.driverDate)</DateStamp>
      <TimeStamp>$($packageManifest.driverVersion)</TimeStamp>
      <SpecifyDriverVerDirectiveDate>true</SpecifyDriverVerDirectiveDate>
      <SpecifyDriverVerDirectiveVersion>true</SpecifyDriverVerDirectiveVersion>
    </Inf>
"@
if (-not $mainProject.Contains($infItem)) { throw 'Unable to configure deterministic StampInf metadata.' }
$mainProject.Replace($infItem, $versionedInfItem) | Set-Content -LiteralPath $mainProjectPath -Encoding utf8

if ($PrepareOnly) {
    [pscustomobject]@{
        SourcePrepared = $true
        SourceDirectory = $WorkRoot
        UpstreamCommit = $actualCommit
        BuildAttempted = $false
    }
    return
}

$wdkRoot = 'C:\Program Files (x86)\Windows Kits\10'
$wdkVersions = Get-ChildItem (Join-Path $wdkRoot 'Include') -Directory -ErrorAction SilentlyContinue |
    Where-Object { Test-Path (Join-Path $_.FullName 'km') } |
    Sort-Object Name -Descending
if (-not $wdkVersions) {
    throw 'WDK kernel headers are missing. Install a matching SDK/WDK before building.'
}

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$vsRoot = (& $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath).Trim()
if (-not $vsRoot) { throw 'Visual Studio C++ Build Tools were not found.' }
$msbuild = Join-Path $vsRoot 'MSBuild\Current\Bin\amd64\MSBuild.exe'

& $msbuild (Join-Path $WorkRoot 'SimpleAudioSample.sln') /m /nodeReuse:false /restore "/p:Configuration=$Configuration" /p:Platform=x64 /p:SignMode=Off /p:EnableTestSign=false
if ($LASTEXITCODE -ne 0) { throw 'WDK driver build failed.' }

New-Item -ItemType Directory -Path $OutputRoot -Force | Out-Null
$artifacts = Get-ChildItem $WorkRoot -Recurse -File | Where-Object { $_.Extension -in '.sys', '.inf', '.cat' }
foreach ($artifact in $artifacts) { Copy-Item -LiteralPath $artifact.FullName -Destination $OutputRoot -Force }

$required = @('VoiceMorphVirtualAudio.sys', 'VoiceMorphVirtualAudio.inf')
foreach ($name in $required) {
    if (-not (Test-Path (Join-Path $OutputRoot $name))) { throw "Required artifact missing: $name" }
}

$catalog = Join-Path $OutputRoot 'VoiceMorphVirtualAudio.cat'
[pscustomobject]@{
    WdkVersion = $wdkVersions[0].Name
    OutputDirectory = $OutputRoot
    Sys = Test-Path (Join-Path $OutputRoot 'VoiceMorphVirtualAudio.sys')
    Inf = Test-Path (Join-Path $OutputRoot 'VoiceMorphVirtualAudio.inf')
    CatalogGenerated = Test-Path $catalog
    CatalogSigned = $false
}
