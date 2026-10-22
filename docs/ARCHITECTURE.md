# Kiến trúc VoiceMorph Studio

## Mục tiêu

Task 01 thiết lập ranh giới rõ giữa giao diện, domain audio và tích hợp hệ điều hành. Chưa có xử lý
âm thanh thực tế; các interface hiện tại là seam để mở rộng ở task sau.

## Luồng giao tiếp

```text
React page → hook → src/services/tauri.ts
                         ↓ invoke
                 Tauri command (Rust)
                         ↓
       audio / effects / presets / platform adapter
```

Command mẫu `get_app_status` trả về tên ứng dụng, phiên bản, nền tảng và trạng thái audio engine.
TypeScript định nghĩa contract tương ứng bằng `AppStatus`.

## Frontend

- `components/`: component dùng lại gồm navigation, header, device selector, meter, slider, effect
  card, voice controls, toggle, status và toast.
- `pages/`: Dashboard, Voice Effects, Voice Studio và Settings là các màn hình hoàn chỉnh của Task 02.
- `hooks/`: quản lý lifecycle khi gọi backend.
- `services/`: duy nhất nơi gọi `invoke`, giúp UI không phụ thuộc trực tiếp Tauri.
- `types/`: contract và navigation type.
- `styles/`: entry CSS của Tailwind 4 và global styles.

UI state như navigation, tìm kiếm preset, filter và draft control được quản lý trong React. Backend
audio state chỉ được đọc qua `useAppStatus` và Tauri service. Control chưa có backend được disabled
hoặc ghi rõ là local UI draft, không mô phỏng audio realtime.

## Rust backend

- `audio/`: trait `AudioEngine`, model thiết bị, selection resolver, lỗi và persistence contract;
  không import `platform` hoặc API Windows.
- `effects/`: trait `VoiceEffect`; thuật toán DSP phải là Rust thuần và dễ unit test.
- `presets/`: model preset độc lập UI.
- `platform/`: trait `PlatformInfo` và adapter `WindowsPlatform`.

Task 03 bổ sung `AudioDeviceProvider`. `platform/windows.rs` hiện thực trait này bằng CPAL WASAPI;
application service trong `lib.rs` điều phối enumerate, chọn thiết bị và persistence. CPAL chỉ đọc
metadata, không tạo stream. ID chính là `cpal::DeviceId`; fingerprint gồm tên, hướng, sample rate,
channels và format chỉ dùng làm fallback nếu driver thay ID.

Lựa chọn được lưu tại `app_config_dir/audio-devices.json`. Mỗi lần refresh và mỗi chu kỳ 5 giây,
snapshot được tạo lại nên thiết bị bị tháo sẽ biến mất khỏi UI. Nếu lựa chọn cũ không còn tồn tại,
resolver ưu tiên thiết bị default hiện tại rồi thiết bị đầu tiên; UI nhận message để thông báo fallback.

## Pipeline realtime

Task 04 dùng hai CPAL stream và bounded SPSC ring buffer `rtrb`:

```text
WASAPI input callback
  -> convert sample về f32
  -> downmix input channels thành mono
  -> AudioPipeline<PassthroughProcessor>
  -> lock-free bounded ring buffer
  -> output callback hoặc silence khi underrun
  -> nhân mono ra số channel của output
  -> convert sang sample format của output
```

`AudioStreamManager` sở hữu hai stream và quản lý start, pause, resume, stop. Drop manager hoặc stop
sẽ drop cả hai CPAL stream. Callback không gọi Tauri IPC, không khóa mutex, không ghi file/log, không
network và không cấp phát heap. Overflow bỏ sample mới; underrun ghi silence. Counters, peak level và
processing time dùng atomic relaxed; stream fault dùng atomic acquire/release.

Pipeline ưu tiên sample rate chung 48 kHz, sau đó chọn giao của range hai thiết bị. Kích thước callback
256 frames chỉ là request và được clamp theo driver. Latency trên UI là ước tính từ buffer input,
output và ring depth; không phải kết quả loopback measurement.

Task 05 bổ sung hai gain stage độc lập. Input gain chạy sau downmix/passthrough và trước ring buffer;
output gain chạy sau ring buffer và trước sample-format conversion. UI ghi target gain/mute vào atomics;
mỗi callback giữ `SmoothedGain` cục bộ và ramp khoảng 10 ms. Meter peak/RMS được tính theo callback
block trên tín hiệu sau gain, sau đó publish bằng atomics. UI chỉ đọc snapshot khoảng 25 lần/giây.

Volume settings được lưu tại `app_config_dir/volume-settings.json`. Preview khi kéo slider chỉ cập
nhật atomics; pointer/key commit mới ghi file, tránh file I/O dày đặc. Callback không đọc/ghi file.

Task 06 thay passthrough bằng `ProcessorChain`: mono f32 được gom thành block 256 frame, qua
`PitchShiftProcessor`/Signalsmith Stretch và `FormantProcessor`, sau đó mới qua input gain. Tham số
`EffectParameters` được chia sẻ bằng atomic; không truyền frame qua IPC. Bypass crossfade sang dry
delay đã căn latency, vì vậy bật/tắt effect không làm thay đổi timing của stream. DSP không import
Windows API; backend có thể dùng lại với CPAL host cho macOS trong tương lai.

## Task 08 - Noise suppression và bảo vệ output

Pipeline realtime: Capture/downmix -> RNNoise -> Noise Gate -> Pitch/Formant -> Voice Effects -> Input
Gain -> lock-free ring -> Output Gain -> Limiter -> Meter -> Device.

`NoiseProcessor` dùng `nnnoiseless` (RNNoise thuần Rust), frame 480 mẫu tại 48 kHz. Adapter bounded giữ
nguyên sample rate CPAL cho thiết bị khác 48 kHz. Strength là dry/wet mix trên output RNNoise; bypass dùng
dry frame đã căn latency. Limiter cuối chuỗi đếm peak đã bảo vệ. Callback không mutex, I/O, log, network
hoặc heap allocation theo frame. Model được nhúng trong binary; persistence nằm ngoài callback.

## Preset pipeline

Task 07 đặt built-in preset JSON trong `src-tauri/presets/builtin.json`. Dữ liệu đi qua loader, model versioned, semantic
validation và `PresetRegistry` trước khi Tauri command có thể trả hoặc áp dụng preset. Registry built-in
không đọc chung storage với user preset; loader cho user preset/import chưa được nối trong Task 07.

```text
Built-in JSON → loader → schema/model → validation → registry
                                               ↓ apply
React Voice Effects → Tauri command → SharedEffectParameters (atomic)
                                      ↓
Pitch/Formant → Ring Mod → Radio Filter/Drive → Tremolo → Makeup Gain
```

Apply preset chỉ ghi một snapshot parameter mới vào atomics và cập nhật active preset ID; CPAL streams
không bị drop/restart. `ProcessorChain` smooth pitch/formant, creative-effect mix và makeup gain. State
oscillator/filter nằm trong callback-owned chain, không có mutex hay allocation theo sample.

Bốn module domain được export từ library crate để tạo extension API rõ ràng và tránh che giấu các
seam có chủ đích. Binary entry point vẫn chỉ gọi `voicemorph_studio_lib::run()`.

Adapter macOS chưa được triển khai. `PlatformInfo` là điểm mở rộng: về sau có thể thêm
`platform/macos.rs` mà không thay đổi audio engine hoặc DSP.

## Quy tắc phụ thuộc

1. `audio`, `effects`, `presets` không phụ thuộc module `platform`.
2. UI chỉ gọi backend qua `services`.
3. Code Windows-specific chỉ nằm trong `platform/windows.rs` hoặc module adapter tương lai.
4. Tauri command mỏng: nhận/trả dữ liệu và điều phối domain, không chứa DSP.
