# Quyết định kỹ thuật

## ADR-001 — Tauri 2 và scaffold chính thức

Chọn `create-tauri-app` Tauri 2 với template React + TypeScript để bám cấu trúc và capability model
được Tauri duy trì. Identifier là `com.voicemorph.studio`.

## ADR-002 — Tailwind CSS 4 qua Vite plugin

Dùng `@tailwindcss/vite` và `@import "tailwindcss"`. Đây là tích hợp được tài liệu Tailwind hiện tại
khuyến nghị cho Vite; không cần PostCSS hoặc `tailwind.config.js` ở giai đoạn này.

## ADR-003 — Tách platform adapter

Domain `audio`, `effects`, `presets` là Rust thuần. API riêng Windows phải đi qua `platform`. Cấu trúc
này cho phép thêm adapter macOS sau này nhưng Task 01 không triển khai hoặc cam kết hỗ trợ macOS.

## ADR-004 — Chưa thêm thư viện audio/DSP

Task 01 chỉ cần nền móng và command mẫu. Không thêm CPAL, audio driver hoặc DSP crate để tránh khóa
kiến trúc trước khi yêu cầu Task 02 được xác định.

## ADR-005 — Bộ cài NSIS và MSI

`tauri.conf.json` khai báo cả NSIS và MSI. Script build truyền rõ target
`x86_64-pc-windows-msvc`; việc ký code-signing chưa nằm trong phạm vi Task 01.

## ADR-006 — CPAL 0.18.2 và WASAPI cho quản lý thiết bị

- Dùng CPAL `DeviceId` làm định danh chính vì có thể serialize và được thiết kế ổn định qua restart.
- Windows adapter nằm trong `platform/windows.rs`; domain `audio` không phụ thuộc Windows API.
- Lưu fingerprint metadata làm fallback, không dùng display name đơn lẻ làm ID.
- Task 03 chỉ enumerate và lưu lựa chọn. Không build hoặc start audio stream trước hành động audio
  rõ ràng của người dùng ở task sau.

## ADR-007 — SPSC ring buffer cho realtime monitoring

- Dùng `rtrb` 0.4.0: bounded, single-producer/single-consumer, không allocate sau khởi tạo và thao tác
  đọc/ghi lock-free, wait-free.
- Ring buffer chứa mono `f32`; conversion format và channel nằm sát biên CPAL.
- Stream callbacks chỉ giữ producer hoặc consumer riêng biệt và atomic metrics, không giữ mutex.
- Không resample trong Task 04. Pipeline chọn sample rate nằm trong giao capability của input/output,
  ưu tiên 48 kHz; nếu không có rate chung thì trả lỗi rõ ràng.
- Không tự start khi ứng dụng mở. Chỉ command do thao tác người dùng mới gọi `Stream::play`.

## ADR-008 — Atomic gain controls và block meters

- Slider 0–100% ánh xạ tuyến tính sang gain 0.0–1.0; Task 05 không cho boost trên unity để hạn chế
  clipping và feedback.
- Input/output gain và mute dùng atomic target độc lập. Callback ramp 10 ms thay vì đổi gain tức thời.
- RMS/peak tính theo block callback, sanitize NaN/Infinity và clamp float32 trước output.
- UI polling metrics ở 25 Hz; không truyền sample hoặc gọi UI từ audio callback.

## ADR-009 — Signalsmith Stretch cho pitch và formant

- Chọn `signalsmith-stretch` 0.1.3 (MIT), wrapper của Signalsmith Stretch, thay vì đổi playback sample
  rate. Thuật toán giữ số frame input/output bằng nhau trong stream và hỗ trợ formant độc lập.
- PSOLA phù hợp giọng đơn âm nhưng cần pitch tracking ổn định; phase vocoder thuần không có formant
  compensation; WSOLA chủ yếu time-stretch. Signalsmith cho trade-off chất lượng/CPU và API C++
  portable sang macOS tốt hơn trong phạm vi Task 06.
- Cấu hình realtime dùng analysis window 2048/interval 512 ở 44.1/48 kHz (4096/1024 ở 96 kHz),
  block Rust 256 frame. Buffer, FFT state và dry delay được cấp phát/warm-up trước `Stream::play`.
- UI ghi target qua atomic; callback smooth 50 ms cho pitch/formant và crossfade bypass 10 ms. Dry path
  được delay theo wet path để tránh thay đổi latency khi bypass.
- Wrapper build-time cần LLVM/libclang. Không có file I/O, network, mutex hay Tauri IPC trong callback.

## ADR-010 — Preset JSON versioned và DSP nội bộ

- Built-in preset dùng schema version 1 và tagged effect list, cho phép thêm effect type mà không đổi các
  field cấp cao. Deserialize dùng `deny_unknown_fields`; semantic validator kiểm tra ID, duplicate,
  finite/range, quan hệ cutoff và processor type hỗ trợ.
- Built-in và user preset có cờ/source tách biệt. Task 07 chỉ khởi tạo built-in registry; không tự động
  load file user để tránh vô tình biến import/export thành phạm vi của task này.
- Robot dùng ring modulation; Radio dùng one-pole high-pass/low-pass và soft saturation; Cartoon dùng
  pitch/formant cùng tremolo nhẹ. Đây là DSP truyền thống, không được mô tả là AI voice conversion.
- Preset gain là makeup gain trong effect chain, không ghi đè input/output volume cá nhân của Task 05.
