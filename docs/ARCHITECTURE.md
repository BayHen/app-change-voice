# Kiến trúc VoiceMorph Studio

## Task 13 - Plugin architecture và Soundboard

`plugins/manifest.rs` định nghĩa manifest schema v1, descriptor và parameter schema. `EffectRegistry` chỉ nhận
manifest trusted, đúng API/platform/capability và effect có factory đã liên kết trong binary. `EffectFactory`
tạo `Box<dyn AudioProcessor>` ngoài realtime callback. Phiên bản này không quét thư mục plugin, không nạp DLL
và không coi Rust dynamic-library ABI là ABI ổn định.

Soundboard lưu metadata tại `app_data_dir/soundboard/library.json`; WAV vẫn ở vị trí người dùng chọn và được
decode thành mono Float32 trước khi CPAL stream chạy. Callback chỉ đọc clip immutable, nhận lệnh `Copy` qua
SPSC ring buffer và dùng tối đa 16 playback voices đã cấp phát. Headphone có queue riêng; virtual microphone
được mix với processed voice trước limiter. File bị di chuyển/xóa được đánh dấu `missing`.

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

## Task 09 - Recording và playback

Input callback ghép `original` đã delay theo DSP latency với `processed` sau toàn bộ input pipeline rồi
`try_push` vào bounded SPSC queue hai giây. Callback không ghi file và tăng `dropped_frames` nếu queue
đầy. Worker `voice-recording-writer` tiêu thụ queue, tạo hai WAV PCM16 mono cùng sample rate và cùng số
frame trong thư mục cache. Export chỉ xảy ra khi người dùng chọn đường dẫn qua Tauri dialog.

Playback worker đọc WAV ngoài callback và nạp bounded queue. Output callback chọn đúng một nguồn:
playback, processed monitoring khi Hear Myself bật, hoặc silence. Vì vậy không tạo output stream thứ hai.
Temporary recording bị xóa qua lệnh rõ ràng và không có network/upload.

## Task 10 - Custom preset repository

`PresetRepository` tách domain khỏi `JsonPresetRepository`. Mỗi custom preset là một file JSON riêng tại
`app_data_dir/presets/<validated-id>.json`; ID chỉ cho phép lowercase ASCII, số và dấu gạch ngang nên
không thể path traversal. Save ghi file `.tmp`, chuyển bản cũ thành `.bak`, rồi rename và rollback nếu lỗi.
Một file hỏng chỉ tạo warning khi load, không làm mất các preset hợp lệ khác.

Schema v2 bổ sung `createdAt`, `updatedAt`, `favorite`, `parameters` và `pluginRequirements`. Import giới
hạn 256 KiB, migrate v1 sang v2, reject schema tương lai, unknown effect và plugin dependency chưa hỗ trợ.
Built-in nằm trong binary và read-only; chỉ duplicate thành user preset. Effect list là tagged enum để có
thể đăng ký processor mới về sau nhưng Task 10 không load hoặc thực thi plugin binary.

Apply/Preview cập nhật atomic DSP parameters, volume và noise controls mà không restart stream. Bass/Treble
dùng EQ nội bộ allocation-free. Effect order giữ cố định theo pipeline hiện tại vì reorder chưa có contract
realtime-safe; UI không cung cấp control giả.

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

## Task 11 - Windows Virtual Microphone routing

`routing/mod.rs` định nghĩa `AudioRoutingService`, `VirtualDeviceManager`, `VirtualOutputSink`,
`RoutingConfiguration` và `RoutingStatus` độc lập với UI. Adapter CPAL/WASAPI nằm trong
`routing/windows.rs`; DSP không import Windows API.

Sau DSP và input gain, input callback fan-out processed sample sang queue monitoring/recording hiện có và
một SPSC queue bounded riêng cho virtual sink. Virtual sink có output stream riêng nhưng không capture mic
lần hai và không chạy processor chain thứ hai. Monitoring output gain không làm đổi mức gửi sang app đích;
limiter cuối được áp dụng riêng cho virtual output. Toggle route dùng atomic flag, callback không mutex,
file I/O, IPC, log hoặc heap allocation theo frame.

Cấu hình endpoint lưu tại `app_config_dir/audio-routing.json`; app không tự khởi động stream sau restart.
ID được khôi phục trước, fingerprint dùng khi WASAPI ID thay đổi; nếu endpoint mất thì cấu hình bị xóa an
toàn và người dùng phải chọn lại. Trạng thái `Active` chỉ có nghĩa playback endpoint mở thành công, không
được diễn giải là Discord/OBS đã nhận audio.

## Task 12 - Global hotkeys và system tray

`hotkeys/config.rs` chứa schema versioned, default bindings, kiểm tra shortcut trùng/reserved và persistence
tại `app_config_dir/hotkeys.json`. `hotkeys/manager.rs` sở hữu registrations của Tauri Global Shortcut
Plugin; cập nhật là transactional: nếu OS từ chối một shortcut, registrations mới được gỡ và bộ cũ được
khôi phục.

Hotkey callback và tray menu cùng gọi application service trong `application.rs`. Service chỉ cập nhật
atomic parameters hoặc stream lifecycle qua `AudioStreamManager`; không chạy công việc trong DSP callback.
Kết quả được emit về React để thông báo, sau đó `tray/manager.rs` refresh label từ backend state thật.

Tray được tạo bằng Tauri Rust API với Show/Hide, preset hiện tại, mute, effects, virtual routing, Settings và
Exit. Close/minimize-to-tray chỉ ẩn cửa sổ. Exit chủ động stop audio/recording/routing, unregister hotkeys rồi
thoát. App restart chỉ restore cấu hình/registration; không tự mở microphone hoặc audio stream.
