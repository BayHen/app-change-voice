# VoiceMorph Virtual Microphone Driver Architecture

## 1. Kết quả review code hiện tại

VoiceMorph hiện có một pipeline capture duy nhất trong `audio/stream.rs`:

```text
CPAL/WASAPI physical input
  -> mono conversion
  -> AudioPipeline (noise -> pitch/formant -> creative effects)
  -> input gain / soundboard mix
  -> recording + meters + monitoring ring + virtual-routing ring
```

`AudioPipeline` và `AudioProcessor` độc lập Windows. Callback sở hữu processor state, dùng atomics và bounded
SPSC queues; không gửi frame qua Tauri IPC. `routing/windows.rs` hiện mở một CPAL **playback** endpoint như
VB-CABLE. Nó không tự tạo capture endpoint. Recording và monitoring cùng dùng processed fan-out nên không
cần capture/DSP pipeline thứ hai.

Tauri commands hiện quản lý device selection, monitoring, recording và cable routing qua application service.
Installer Tauri tạo NSIS/MSI cho ứng dụng user mode; chưa có driver package, elevation flow hay signing chain.

## 2. Kiến trúc mục tiêu

```text
Physical microphone (WASAPI/CPAL)
        |
Rust AudioEngine / DSP / gain / limiter / soundboard
        |
bounded non-blocking VirtualAudioTap
        |
user-mode AudioBridge worker (format adapter + packetization)
        |
versioned, access-controlled driver transport
        |
kernel bounded PCM ring -> SysVAD-derived WaveRT capture miniport
        |
Windows Audio Engine -> VoiceMorph Virtual Microphone endpoint
        |
Discord / OBS / games / conferencing applications
```

### Ranh giới trách nhiệm

- **Rust:** physical capture, DSP, mix, limiter, resample/channel conversion, bridge producer and metrics.
- **Audio bridge:** user-mode worker, reconnect, packet sequence, bounded queue, protocol negotiation. Không chạy
  từ callback CPAL và không cài driver.
- **C++ driver:** PnP/power, PortCls adapter, topology, WaveRT capture pin, clock/position, consume PCM, silence on
  underrun. Không có DSP/preset/network/file I/O.
- **Tauri:** explicit install/remove/status commands, UI và diagnostics. Không truyền audio frames qua IPC.

Existing `RoutingProducer` là seam phù hợp. Phase implementation thêm sink backend mới phía sau abstraction,
không thay `AudioPipeline`. CPAL virtual-cable backend được giữ làm compatibility option, nhưng không còn là
dependency bắt buộc khi driver riêng sẵn sàng.

## 3. SysVAD analysis

Đã review Microsoft Windows Driver Samples commit
`bb68ed5e770bce04dc5395ec9258dafab034184d`, thư mục `audio/sysvad`.

### Có thể tái sử dụng có kiểm soát

- PortCls adapter/common lifecycle và miniport-pair organization.
- WaveRT miniport/stream allocation, state transitions, notification events, packet/position accounting.
- Capture topology/filter/pin descriptor và format negotiation patterns.
- Componentized INF/package layout, interface registration và power/PnP patterns.
- Unit-testable format tables và package validation workflow.

Microsoft sample phải được pin commit, giữ license notices và review production hardening. Không copy cả sample,
render/offload/Bluetooth/USB/HDMI/KWS/APO endpoints không liên quan.

### Phải thay thế/phát triển riêng

- SysVAD capture mô phỏng dữ liệu bằng `ToneGenerator`; VoiceMorph phải consume PCM bridge và trả silence khi
  producer vắng mặt.
- Tạo capture endpoint/topology tối giản, GUID, hardware ID, names, provider và service riêng.
- Thiết kế bridge protocol, authentication/ACL, buffering, reconnect, counters và cleanup khi process crash.
- Clock drift, frame pacing, overrun/underrun, multi-client semantics, suspend/resume và surprise removal.
- INF/catalog/signing/HLK cho sản phẩm; không dùng sample certificate hoặc Contoso metadata.

SysVAD là WDM audio sample dùng WaveRT. Windows Audio Engine nhận capture buffer qua WaveRT/PortCls; INF đăng ký
audio/capture/realtime interfaces. Sample package gồm driver, INF, catalog và optional APO components. VoiceMorph
không cần các sample APO vì DSP nằm trong Rust.

## 4. Bridge contract v1

Canonical transport format ban đầu: 48 kHz, mono, Float32. Rust phải convert trước bridge. Header C++ và Rust
contract dùng protocol version, message type, byte length và monotonic sequence. Payload không chứa pointer,
handle hay struct phụ thuộc compiler.

Data plane phải bounded/non-blocking. Kiến trúc đề xuất cho implementation review:

1. Companion bridge worker mở device interface riêng bằng overlapped I/O.
2. Worker gửi các packet PCM kích thước cố định vào một queue I/O bounded; không gọi từ CPAL callback.
3. Driver validate version/length/format/sequence rồi copy vào nonpaged ring do driver sở hữu.
4. WaveRT capture stream lấy frame theo clock; thiếu dữ liệu thì zero-fill và tăng underrun counter.
5. Disconnect/cancel/process termination giải phóng pending requests và zeroizes queued PCM.

`METHOD_NEITHER` và mapping user pointers lâu dài không được chọn mặc định vì lifetime/security phức tạp. Shared
memory có thể giảm copy nhưng chỉ được cân nhắc sau threat model, ACL và verifier tests. Phase 11.2.1 chưa quyết
định IOCTL transport cuối cùng.

## 5. Driver lifecycle/package

```text
Build .sys + generated INF -> InfVerif -> Inf2Cat -> sign catalog/binaries
-> explicit elevated staging/install -> root devnode -> endpoint enumeration
-> query health/version -> update/rollback -> explicit uninstall
```

Application installer và driver package là hai trust/elevation boundaries. App không được âm thầm cài driver.
Release cần Microsoft signing/attestation or HLK/WHCP path phù hợp; test certificate chỉ dùng test machine.

## 6. Failure policy

- Bridge absent/slow: capture endpoint emits silence, never stale audio.
- Producer overflow: drop bounded packets and expose counter; never block audio callback.
- Driver disconnect: Rust stops bridge sink and preserves monitoring/recording.
- Format/protocol mismatch: fail closed, no reinterpretation.
- App crash: driver cancels transport, emits silence and releases client context.
- Kernel allocation failure or invalid input: return NTSTATUS error; never bugcheck for recoverable input.

## 7. Risks and required proof

- Kernel bugs can crash the OS; use a separate test target, WinDbg, Driver Verifier and crash dumps.
- Clock drift between CPAL capture and WaveRT endpoint can cause long-session under/overflow.
- DPC/ISR and lock behavior must meet realtime constraints; no pageable access at elevated IRQL.
- Driver signing and HLK/WHCP are release blockers, distinct from application code signing.
- Anti-cheat/security products may scrutinize third-party kernel drivers; do not inject/hook processes.
- Windows servicing, sleep/resume and concurrent clients require hardware/manual tests.
