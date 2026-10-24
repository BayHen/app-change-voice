#pragma once

#if defined(_NTDDK_) || defined(_NTIFS_)
typedef ULONG vm_u32;
typedef USHORT vm_u16;
typedef ULONGLONG vm_u64;
typedef LONG32 vm_i32;
#else
#include <stdint.h>
typedef uint32_t vm_u32;
typedef uint16_t vm_u16;
typedef uint64_t vm_u64;
typedef int32_t vm_i32;
#endif

// Versioned METHOD_BUFFERED IOCTL contract shared by user mode and the driver.
#define VOICEMORPH_BRIDGE_PROTOCOL_VERSION 1u
#define VOICEMORPH_BRIDGE_SAMPLE_RATE 48000u
#define VOICEMORPH_BRIDGE_CHANNELS 1u
#define VOICEMORPH_BRIDGE_SAMPLE_FORMAT_PCM_S32 2u
#define VOICEMORPH_BRIDGE_PACKET_FRAMES 480u
#define VOICEMORPH_BRIDGE_RING_FRAMES 4096u
#define VOICEMORPH_DRIVER_VERSION_MAJOR 0u
#define VOICEMORPH_DRIVER_VERSION_MINOR 1u
#define VOICEMORPH_DRIVER_VERSION_PATCH 0u
#define VOICEMORPH_DRIVER_VERSION_BUILD 0u

// {B566F7B7-91D8-4F31-9712-94C55A10E52A}
#define VOICEMORPH_BRIDGE_INTERFACE_GUID_INIT \
    {0xb566f7b7, 0x91d8, 0x4f31, {0x97, 0x12, 0x94, 0xc5, 0x5a, 0x10, 0xe5, 0x2a}}

#define VOICEMORPH_BRIDGE_IOCTL_DEVICE_TYPE 0x8337u
#define VOICEMORPH_BRIDGE_IOCTL_HELLO_FUNCTION 0x800u
#define VOICEMORPH_BRIDGE_IOCTL_CONFIGURE_FUNCTION 0x801u
#define VOICEMORPH_BRIDGE_IOCTL_WRITE_FUNCTION 0x802u
#define VOICEMORPH_BRIDGE_IOCTL_STATUS_FUNCTION 0x803u
#define VOICEMORPH_BRIDGE_IOCTL_RESET_FUNCTION 0x804u

enum VOICEMORPH_BRIDGE_MESSAGE_TYPE : vm_u32 {
    VoiceMorphBridgeHello = 1,
    VoiceMorphBridgeConfigure = 2,
    VoiceMorphBridgeAudioPacket = 3,
    VoiceMorphBridgeStatus = 4,
    VoiceMorphBridgeReset = 5,
};

struct VOICEMORPH_BRIDGE_HEADER {
    vm_u32 protocol_version;
    vm_u32 message_type;
    vm_u32 payload_bytes;
    vm_u32 reserved;
    vm_u64 sequence;
};

struct VOICEMORPH_BRIDGE_FORMAT {
    vm_u32 sample_rate;
    vm_u16 channels;
    vm_u16 sample_format;
    vm_u32 frames_per_packet;
};

struct VOICEMORPH_BRIDGE_COUNTERS {
    vm_u64 written_frames;
    vm_u64 consumed_frames;
    vm_u64 dropped_frames;
    vm_u64 underrun_frames;
};

struct VOICEMORPH_BRIDGE_STATUS {
    vm_u32 protocol_version;
    vm_u32 configured;
    vm_u32 queued_frames;
    vm_u32 capacity_frames;
    vm_u16 driver_version_major;
    vm_u16 driver_version_minor;
    vm_u16 driver_version_patch;
    vm_u16 driver_version_build;
    vm_u32 reserved[2];
    VOICEMORPH_BRIDGE_COUNTERS counters;
};

struct VOICEMORPH_BRIDGE_PACKET {
    VOICEMORPH_BRIDGE_HEADER header;
    vm_i32 samples[VOICEMORPH_BRIDGE_PACKET_FRAMES];
};

static_assert(sizeof(VOICEMORPH_BRIDGE_HEADER) == 24, "Bridge header ABI changed");
static_assert(sizeof(VOICEMORPH_BRIDGE_FORMAT) == 12, "Bridge format ABI changed");
static_assert(sizeof(VOICEMORPH_BRIDGE_COUNTERS) == 32, "Bridge counters ABI changed");
static_assert(sizeof(VOICEMORPH_BRIDGE_STATUS) == 64, "Bridge status ABI changed");
static_assert(sizeof(VOICEMORPH_BRIDGE_PACKET) == 1944, "Bridge packet ABI changed");
