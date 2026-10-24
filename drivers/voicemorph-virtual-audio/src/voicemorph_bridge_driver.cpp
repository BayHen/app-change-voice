#include "voicemorph_bridge_driver.h"

#include <wdmsec.h>

#define VM_CTL(function) CTL_CODE(VOICEMORPH_BRIDGE_IOCTL_DEVICE_TYPE, function, METHOD_BUFFERED, FILE_READ_DATA | FILE_WRITE_DATA)

static constexpr ULONG IOCTL_VM_HELLO = VM_CTL(VOICEMORPH_BRIDGE_IOCTL_HELLO_FUNCTION);
static constexpr ULONG IOCTL_VM_CONFIGURE = VM_CTL(VOICEMORPH_BRIDGE_IOCTL_CONFIGURE_FUNCTION);
static constexpr ULONG IOCTL_VM_WRITE = VM_CTL(VOICEMORPH_BRIDGE_IOCTL_WRITE_FUNCTION);
static constexpr ULONG IOCTL_VM_STATUS = VM_CTL(VOICEMORPH_BRIDGE_IOCTL_STATUS_FUNCTION);
static constexpr ULONG IOCTL_VM_RESET = VM_CTL(VOICEMORPH_BRIDGE_IOCTL_RESET_FUNCTION);

static KSPIN_LOCK g_lock;
static LONG32 g_ring[VOICEMORPH_BRIDGE_RING_FRAMES];
static ULONG g_read = 0;
static ULONG g_write = 0;
static ULONG g_count = 0;
static BOOLEAN g_configured = FALSE;
static PFILE_OBJECT g_producer = nullptr;
static VOICEMORPH_BRIDGE_COUNTERS g_counters = {};
static PDRIVER_DISPATCH g_fallbackDeviceControl = nullptr;
static PDRIVER_DISPATCH g_fallbackCleanup = nullptr;

static void ResetLocked()
{
    g_read = g_write = g_count = 0;
    RtlZeroMemory(&g_counters, sizeof(g_counters));
}

void VoiceMorphBridgeInitialize()
{
    KeInitializeSpinLock(&g_lock);
    KIRQL oldIrql;
    KeAcquireSpinLock(&g_lock, &oldIrql);
    ResetLocked();
    g_configured = FALSE;
    g_producer = nullptr;
    KeReleaseSpinLock(&g_lock, oldIrql);
}

void VoiceMorphBridgeShutdown()
{
    KIRQL oldIrql;
    KeAcquireSpinLock(&g_lock, &oldIrql);
    ResetLocked();
    g_configured = FALSE;
    g_producer = nullptr;
    KeReleaseSpinLock(&g_lock, oldIrql);
}

void VoiceMorphBridgeSetFallbackDispatch(PDRIVER_DISPATCH deviceControl, PDRIVER_DISPATCH cleanup)
{
    g_fallbackDeviceControl = deviceControl;
    g_fallbackCleanup = cleanup;
}

ULONG VoiceMorphBridgeRead(void* destination, ULONG byteCount)
{
    auto output = static_cast<LONG32*>(destination);
    const ULONG requested = byteCount / sizeof(LONG32);
    ULONG copied = 0;
    KIRQL oldIrql;
    KeAcquireSpinLock(&g_lock, &oldIrql);
    while (copied < requested && g_count > 0)
    {
        output[copied++] = g_ring[g_read];
        g_read = (g_read + 1) % VOICEMORPH_BRIDGE_RING_FRAMES;
        --g_count;
    }
    g_counters.consumed_frames += copied;
    g_counters.underrun_frames += requested - copied;
    KeReleaseSpinLock(&g_lock, oldIrql);
    if (copied < requested)
    {
        RtlZeroMemory(output + copied, (requested - copied) * sizeof(LONG32));
    }
    if ((byteCount % sizeof(LONG32)) != 0)
    {
        RtlZeroMemory(static_cast<UCHAR*>(destination) + requested * sizeof(LONG32), byteCount % sizeof(LONG32));
    }
    return copied;
}

static NTSTATUS Complete(PIRP irp, NTSTATUS status, ULONG_PTR information = 0)
{
    irp->IoStatus.Status = status;
    irp->IoStatus.Information = information;
    IoCompleteRequest(irp, IO_NO_INCREMENT);
    return status;
}

static BOOLEAN ValidFormat(const VOICEMORPH_BRIDGE_FORMAT& format)
{
    return format.sample_rate == VOICEMORPH_BRIDGE_SAMPLE_RATE &&
        format.channels == VOICEMORPH_BRIDGE_CHANNELS &&
        format.sample_format == VOICEMORPH_BRIDGE_SAMPLE_FORMAT_PCM_S32 &&
        format.frames_per_packet > 0 &&
        format.frames_per_packet <= VOICEMORPH_BRIDGE_PACKET_FRAMES;
}

NTSTATUS VoiceMorphBridgeDeviceControl(PDEVICE_OBJECT deviceObject, PIRP irp)
{
    auto stack = IoGetCurrentIrpStackLocation(irp);
    const ULONG code = stack->Parameters.DeviceIoControl.IoControlCode;
    if (code != IOCTL_VM_HELLO && code != IOCTL_VM_CONFIGURE && code != IOCTL_VM_WRITE &&
        code != IOCTL_VM_STATUS && code != IOCTL_VM_RESET)
    {
        return g_fallbackDeviceControl ? g_fallbackDeviceControl(deviceObject, irp) : Complete(irp, STATUS_INVALID_DEVICE_REQUEST);
    }

    void* buffer = irp->AssociatedIrp.SystemBuffer;
    const ULONG inputLength = stack->Parameters.DeviceIoControl.InputBufferLength;
    const ULONG outputLength = stack->Parameters.DeviceIoControl.OutputBufferLength;
    auto fileObject = stack->FileObject;

    if (code == IOCTL_VM_HELLO || code == IOCTL_VM_STATUS)
    {
        if (!buffer || outputLength < sizeof(VOICEMORPH_BRIDGE_STATUS)) return Complete(irp, STATUS_BUFFER_TOO_SMALL);
        auto status = static_cast<VOICEMORPH_BRIDGE_STATUS*>(buffer);
        KIRQL oldIrql;
        KeAcquireSpinLock(&g_lock, &oldIrql);
        status->protocol_version = VOICEMORPH_BRIDGE_PROTOCOL_VERSION;
        status->configured = g_configured ? 1u : 0u;
        status->queued_frames = g_count;
        status->capacity_frames = VOICEMORPH_BRIDGE_RING_FRAMES;
        status->driver_version_major = VOICEMORPH_DRIVER_VERSION_MAJOR;
        status->driver_version_minor = VOICEMORPH_DRIVER_VERSION_MINOR;
        status->driver_version_patch = VOICEMORPH_DRIVER_VERSION_PATCH;
        status->driver_version_build = VOICEMORPH_DRIVER_VERSION_BUILD;
        status->reserved[0] = status->reserved[1] = 0;
        status->counters = g_counters;
        KeReleaseSpinLock(&g_lock, oldIrql);
        return Complete(irp, STATUS_SUCCESS, sizeof(*status));
    }

    if (code == IOCTL_VM_CONFIGURE)
    {
        if (!buffer || inputLength != sizeof(VOICEMORPH_BRIDGE_FORMAT)) return Complete(irp, STATUS_INFO_LENGTH_MISMATCH);
        const auto format = *static_cast<VOICEMORPH_BRIDGE_FORMAT*>(buffer);
        if (!ValidFormat(format)) return Complete(irp, STATUS_NOT_SUPPORTED);
        KIRQL oldIrql;
        KeAcquireSpinLock(&g_lock, &oldIrql);
        if (g_producer && g_producer != fileObject)
        {
            KeReleaseSpinLock(&g_lock, oldIrql);
            return Complete(irp, STATUS_DEVICE_BUSY);
        }
        g_producer = fileObject;
        g_configured = TRUE;
        ResetLocked();
        KeReleaseSpinLock(&g_lock, oldIrql);
        return Complete(irp, STATUS_SUCCESS);
    }

    if (code == IOCTL_VM_RESET)
    {
        KIRQL oldIrql;
        KeAcquireSpinLock(&g_lock, &oldIrql);
        if (g_producer != fileObject)
        {
            KeReleaseSpinLock(&g_lock, oldIrql);
            return Complete(irp, STATUS_ACCESS_DENIED);
        }
        ResetLocked();
        KeReleaseSpinLock(&g_lock, oldIrql);
        return Complete(irp, STATUS_SUCCESS);
    }

    if (!buffer || inputLength < sizeof(VOICEMORPH_BRIDGE_HEADER)) return Complete(irp, STATUS_INFO_LENGTH_MISMATCH);
    const auto header = static_cast<const VOICEMORPH_BRIDGE_HEADER*>(buffer);
    if (header->protocol_version != VOICEMORPH_BRIDGE_PROTOCOL_VERSION || header->message_type != VoiceMorphBridgeAudioPacket ||
        header->reserved != 0 || header->payload_bytes > VOICEMORPH_BRIDGE_PACKET_FRAMES * sizeof(LONG32) ||
        header->payload_bytes % sizeof(LONG32) != 0 || inputLength != sizeof(*header) + header->payload_bytes)
    {
        return Complete(irp, STATUS_INVALID_PARAMETER);
    }
    const ULONG frames = header->payload_bytes / sizeof(LONG32);
    const auto samples = reinterpret_cast<const LONG32*>(header + 1);
    KIRQL oldIrql;
    KeAcquireSpinLock(&g_lock, &oldIrql);
    if (!g_configured || g_producer != fileObject)
    {
        KeReleaseSpinLock(&g_lock, oldIrql);
        return Complete(irp, STATUS_INVALID_DEVICE_STATE);
    }
    const ULONG writable = min(frames, VOICEMORPH_BRIDGE_RING_FRAMES - g_count);
    for (ULONG i = 0; i < writable; ++i)
    {
        g_ring[g_write] = samples[i];
        g_write = (g_write + 1) % VOICEMORPH_BRIDGE_RING_FRAMES;
    }
    g_count += writable;
    g_counters.written_frames += writable;
    g_counters.dropped_frames += frames - writable;
    KeReleaseSpinLock(&g_lock, oldIrql);
    // A full bounded ring is a recoverable realtime condition, not a transport failure.
    return Complete(irp, STATUS_SUCCESS, writable * sizeof(LONG32));
}

NTSTATUS VoiceMorphBridgeCleanup(PDEVICE_OBJECT deviceObject, PIRP irp)
{
    auto fileObject = IoGetCurrentIrpStackLocation(irp)->FileObject;
    KIRQL oldIrql;
    KeAcquireSpinLock(&g_lock, &oldIrql);
    if (g_producer == fileObject)
    {
        ResetLocked();
        g_configured = FALSE;
        g_producer = nullptr;
    }
    KeReleaseSpinLock(&g_lock, oldIrql);
    return g_fallbackCleanup ? g_fallbackCleanup(deviceObject, irp) : Complete(irp, STATUS_SUCCESS);
}
