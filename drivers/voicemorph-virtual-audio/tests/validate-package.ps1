[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$manifestPath = Join-Path $projectRoot 'driver-package.json'
$infPath = Join-Path $projectRoot 'inf\VoiceMorphVirtualAudio.inx'
$patchPath = Join-Path $projectRoot 'patches\0001-voicemorph-capture-only-silence.patch'

$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
$inf = Get-Content -LiteralPath $infPath -Raw
$patch = Get-Content -LiteralPath $patchPath -Raw

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

Assert-True ($manifest.schemaVersion -eq 1) 'Unsupported local package manifest schema.'
Assert-True ($manifest.architecture -eq 'x64') 'The prototype must target x64.'
Assert-True ($manifest.endpoint.kind -eq 'capture') 'Only the capture endpoint may be registered.'
Assert-True ($manifest.endpoint.fallback -eq 'silence') 'Fallback must be silence.'

$requiredInfTokens = @(
    'ProviderName="VoiceMorph Studio"',
    'DeviceDescription="VoiceMorph Virtual Microphone"',
    'ROOT\VoiceMorphVirtualMicrophone',
    'AddService=VoiceMorphVirtualAudio',
    'DriverVer=10/23/2026,0.1.0.0',
    'CatalogFile=VoiceMorphVirtualAudio.cat',
    'KSCATEGORY_CAPTURE',
    'VoiceMorphBridgeInterface="{B566F7B7-91D8-4F31-9712-94C55A10E52A}"'
)
foreach ($token in $requiredInfTokens) {
    Assert-True ($inf.Contains($token)) "INF token missing: $token"
}

Assert-True (-not $inf.Contains('KSCATEGORY_RENDER')) 'Render endpoint registration is not allowed.'
Assert-True ($patch.Contains('#define g_cRenderEndpoints  0')) 'Render miniport registration is not disabled.'
Assert-True ($patch.Contains('VoiceMorphBridgeRead')) 'Bridge ring is not connected to the capture path.'
Assert-True ($patch.Contains('VoiceMorphBridgeDeviceControl')) 'Buffered IOCTL dispatch is not connected.'
Assert-True (-not $patch.Contains('+        m_ToneGenerator.GenerateSine')) 'Tone generation must not be added.'
Assert-True ($patch.Contains('<TargetName>VoiceMorphVirtualAudio</TargetName>')) 'x64 binary identity is not configured.'
Assert-True ($patch.Contains('VoiceMorphVirtualAudio.rc')) 'VoiceMorph version resource is not configured.'

[pscustomobject]@{
    PackageContract = 'PASS'
    Identity = $manifest.name
    Architecture = $manifest.architecture
    Endpoint = 'capture-only'
    Format = '48000 Hz / mono / PCM32'
    Transport = 'METHOD_BUFFERED IOCTL / bounded kernel ring'
    InfVerif = 'NOT RUN (static contract test only)'
}
