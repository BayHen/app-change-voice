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

Bốn module domain được export từ library crate để tạo extension API rõ ràng và tránh che giấu các
seam có chủ đích. Binary entry point vẫn chỉ gọi `voicemorph_studio_lib::run()`.

Adapter macOS chưa được triển khai. `PlatformInfo` là điểm mở rộng: về sau có thể thêm
`platform/macos.rs` mà không thay đổi audio engine hoặc DSP.

## Quy tắc phụ thuộc

1. `audio`, `effects`, `presets` không phụ thuộc module `platform`.
2. UI chỉ gọi backend qua `services`.
3. Code Windows-specific chỉ nằm trong `platform/windows.rs` hoặc module adapter tương lai.
4. Tauri command mỏng: nhận/trả dữ liệu và điều phối domain, không chứa DSP.
