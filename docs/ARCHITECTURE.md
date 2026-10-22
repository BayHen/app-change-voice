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

- `components/`: UI dùng lại như sidebar, logo và status badge.
- `pages/`: Dashboard và các trang khung Voice Effects, Voice Studio, Settings.
- `hooks/`: quản lý lifecycle khi gọi backend.
- `services/`: duy nhất nơi gọi `invoke`, giúp UI không phụ thuộc trực tiếp Tauri.
- `types/`: contract và navigation type.
- `styles/`: entry CSS của Tailwind 4 và global styles.

## Rust backend

- `audio/`: trait `AudioEngine`; không import `platform` hoặc API Windows.
- `effects/`: trait `VoiceEffect`; thuật toán DSP phải là Rust thuần và dễ unit test.
- `presets/`: model preset độc lập UI.
- `platform/`: trait `PlatformInfo` và adapter `WindowsPlatform`.

Bốn module domain được export từ library crate để tạo extension API rõ ràng và tránh che giấu các
seam có chủ đích. Binary entry point vẫn chỉ gọi `voicemorph_studio_lib::run()`.

Adapter macOS chưa được triển khai. `PlatformInfo` là điểm mở rộng: về sau có thể thêm
`platform/macos.rs` mà không thay đổi audio engine hoặc DSP.

## Quy tắc phụ thuộc

1. `audio`, `effects`, `presets` không phụ thuộc module `platform`.
2. UI chỉ gọi backend qua `services`.
3. Code Windows-specific chỉ nằm trong `platform/windows.rs` hoặc module adapter tương lai.
4. Tauri command mỏng: nhận/trả dữ liệu và điều phối domain, không chứa DSP.
