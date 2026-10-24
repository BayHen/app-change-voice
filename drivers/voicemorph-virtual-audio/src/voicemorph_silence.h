#pragma once

#include <ntddk.h>

_IRQL_requires_max_(DISPATCH_LEVEL)
void VoiceMorphFillSilence(
    _Out_writes_bytes_all_(byte_count) void* destination,
    _In_ size_t byte_count) noexcept;
