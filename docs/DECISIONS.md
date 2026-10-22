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
