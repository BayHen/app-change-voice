#pragma once

#include <ntddk.h>
#include "voicemorph_bridge_protocol.h"

void VoiceMorphBridgeInitialize();
void VoiceMorphBridgeShutdown();
ULONG VoiceMorphBridgeRead(_Out_writes_bytes_(byteCount) void* destination, ULONG byteCount);
NTSTATUS VoiceMorphBridgeDeviceControl(_In_ PDEVICE_OBJECT deviceObject, _Inout_ PIRP irp);
NTSTATUS VoiceMorphBridgeCleanup(_In_ PDEVICE_OBJECT deviceObject, _Inout_ PIRP irp);
void VoiceMorphBridgeSetFallbackDispatch(PDRIVER_DISPATCH deviceControl, PDRIVER_DISPATCH cleanup);
