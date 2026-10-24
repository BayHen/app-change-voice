#include "voicemorph_silence.h"

_IRQL_requires_max_(DISPATCH_LEVEL)
void VoiceMorphFillSilence(void* destination, size_t byte_count) noexcept
{
    if (destination == nullptr || byte_count == 0)
    {
        return;
    }

    RtlZeroMemory(destination, byte_count);
}
