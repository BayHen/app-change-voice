#include "../include/voicemorph_bridge_protocol.h"

#include <type_traits>

static_assert(std::is_standard_layout_v<VOICEMORPH_BRIDGE_HEADER>);
static_assert(std::is_trivially_copyable_v<VOICEMORPH_BRIDGE_HEADER>);
static_assert(std::is_standard_layout_v<VOICEMORPH_BRIDGE_FORMAT>);
static_assert(std::is_trivially_copyable_v<VOICEMORPH_BRIDGE_FORMAT>);
static_assert(std::is_trivially_copyable_v<VOICEMORPH_BRIDGE_PACKET>);
static_assert(sizeof(VOICEMORPH_BRIDGE_STATUS) == 64);

int main()
{
    VOICEMORPH_BRIDGE_HEADER header{};
    header.protocol_version = VOICEMORPH_BRIDGE_PROTOCOL_VERSION;
    header.message_type = VoiceMorphBridgeHello;
    VOICEMORPH_BRIDGE_FORMAT format{48000, 1, VOICEMORPH_BRIDGE_SAMPLE_FORMAT_PCM_S32, 480};
    return header.protocol_version == 1u && header.message_type == 1u &&
        format.sample_format == 2u ? 0 : 1;
}
