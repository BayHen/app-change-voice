# Tiến độ Task 01

## Đã hoàn thành

- Scaffold Tauri 2 + React + TypeScript tại root repository hiện tại.
- Cấu hình Tailwind CSS, ESLint, Prettier, Vitest và rustfmt.
- Tạo UI VoiceMorph Studio với sidebar, Dashboard, Voice Effects, Voice Studio, Settings và version.
- Tạo command `get_app_status` để xác minh React ↔ Rust.
- Tách module Rust `audio`, `effects`, `platform`, `presets` và thêm unit test.
- Cấu hình app version `0.1.0`, tên hiển thị và build Windows x64.
- Kiểm tra toolchain Windows và ghi nhận dependency còn thiếu.

## Kết quả lệnh xác minh

| Lệnh                              | Kết quả                                                |
| --------------------------------- | ------------------------------------------------------ |
| `npm.cmd install`                 | Thành công — 193 package, 0 vulnerability              |
| `npm.cmd run build`               | Thành công — TypeScript và Vite production build       |
| `npm.cmd run test`                | Thành công — 1 test file, 1 test                       |
| `npm.cmd run lint`                | Thành công                                             |
| `npm.cmd run format`              | Thành công                                             |
| `npm.cmd run dev`                 | Thành công — Vite trả HTTP 200 tại port 1420           |
| `cargo check --all-targets`       | Thành công — không có compiler warning                 |
| `cargo clippy -- -D warnings`     | Thành công — không có Clippy warning                   |
| `cargo test`                      | Thành công — 4/4 Rust unit test                        |
| `cargo fmt -- --check`            | Thành công                                             |
| `npm.cmd run tauri:dev`           | Thành công — chạy `target/debug/voicemorph-studio.exe` |
| `npm.cmd run tauri:build:windows` | Thành công — tạo NSIS và MSI x64                       |

Rust, MSVC Build Tools và Windows SDK đã được nhận diện. Rust compile, cửa sổ Tauri dev và hai bộ
cài Windows x64 đã được xác nhận trong quality gate ngày 22/10/2026.

## Task 02 - Desktop UI & Navigation

### Đã hoàn thành

- Xây dựng dark desktop shell responsive từ 900x600 với sidebar và mobile navigation fallback.
- Hoàn thiện Dashboard, Voice Effects, Voice Studio và Settings.
- Tạo component dùng lại: `AppSidebar`, `AppHeader`, `DeviceSelector`, `AudioMeter`,
  `VolumeSlider`, `EffectCard`, `VoiceControls`, `ToggleSwitch`, `StatusIndicator` và
  `NotificationToast`.
- Tách UI state khỏi backend audio state bằng TypeScript interfaces.
- Thêm loading, error, disabled, empty state và keyboard focus states.
- Giữ meter ở zero khi audio offline; không dùng waveform hoặc dữ liệu realtime giả.
- Đánh dấu toàn bộ action cần audio engine là disabled hoặc Coming Soon.
- Thêm test DOM cho navigation, render, microphone offline, preset search, slider và toggle.
- Chụp screenshot kiểm tra responsive tại 900x600 trong `docs/screenshots/`.

### Kết quả kiểm thử

- `npm.cmd run build`: thành công, TypeScript không có lỗi.
- `npm.cmd run lint`: thành công.
- `npm.cmd run format`: thành công.
- `npm.cmd run test`: thành công, 2 test files và 5 tests.
- Resize smoke test: thành công tại 1440x900 và 900x600, không tràn ngang.
- `cargo check`, `cargo test`, `cargo fmt -- --check` và `cargo clippy --all-targets -- -D warnings`:
  thành công; 4/4 Rust unit tests đạt và không có compiler warning.
- Tauri runtime smoke test: thành công; `target/debug/voicemorph-studio.exe` mở cửa sổ
  `VoiceMorph Studio`, tiến trình phản hồi và command `get_app_status` cung cấp trạng thái backend.

## Ngoài phạm vi

- Thu/phát âm thanh, DSP và voice effects thực tế.
- macOS adapter.
- Code signing và phát hành.
- Push GitHub hoặc commit tự động.

## Task 03 - Audio Device Management

### Đã hoàn thành

- Tích hợp CPAL 0.18.2 với WASAPI mặc định trên Windows.
- Liệt kê input/output, nhận diện default và trả metadata sample rate, channels, sample format,
  buffer range khi driver hỗ trợ.
- Dùng CPAL `DeviceId` làm ID chính; dùng fingerprint metadata khi driver đổi ID.
- Thêm các command `get_input_devices`, `get_output_devices`, `get_default_devices`,
  `select_input_device`, `select_output_device` và `refresh_audio_devices`.
- Lưu lựa chọn tại `app_config_dir/audio-devices.json`; fallback về default có thông báo.
- Dashboard dùng dữ liệu backend thật, hỗ trợ refresh và tự kiểm tra ngắt thiết bị mỗi 5 giây.
- Không tạo hoặc khởi động audio stream.

### Phân loại kiểm thử

- Unit test tự động kiểm tra nhiều input, ID thay đổi, thiết bị biến mất, default thay đổi và không có
  microphone.
- Test frontend kiểm tra render metadata và refresh command.
- Hai hardware test được đánh dấu `#[ignore]`; chỉ được báo PASS khi chạy thủ công trên máy có thiết bị.

### Kết quả xác minh

- `npm.cmd run test`: thành công, 6/6 frontend tests.
- `npm.cmd run build`, `npm.cmd run lint`, `npm.cmd run format`: thành công.
- `cargo test`: thành công, 9 unit tests; 2 hardware tests được bỏ qua theo mặc định.
- `cargo test manual_hardware_tests -- --ignored --nocapture`: thành công 2/2 trên máy Windows hiện
  tại; WASAPI báo ít nhất một input và CPAL ID không trùng trong snapshot.
- `cargo check`, `cargo fmt -- --check`, `cargo clippy --all-targets -- -D warnings`: thành công.
