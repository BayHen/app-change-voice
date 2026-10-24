# VoiceMorph Virtual Audio Driver

Prototype virtual microphone x64 dựa trên kiến trúc WDM/PortCls/WaveRT của Microsoft Simple Audio Sample,
một sample tối giản cùng họ kiến trúc với SysVAD. Source Microsoft không được vendor toàn bộ: build script yêu cầu
checkout đúng commit đã ghim rồi áp dụng overlay VoiceMorph.

## Phạm vi dự kiến

- Root-enumerated adapter `ROOT\VoiceMorphVirtualMicrophone`.
- Chỉ đăng ký capture endpoint `VoiceMorph Virtual Microphone`; render endpoint mẫu bị vô hiệu hóa.
- Format prototype: 48 kHz, mono, PCM signed 32-bit.
- Capture path đọc PCM32 từ bounded kernel ring; khi underrun, phần còn thiếu luôn được zero-fill an toàn.
- Bridge v1 dùng device interface `{B566F7B7-91D8-4F31-9712-94C55A10E52A}` và buffered IOCTL; chỉ một producer được cấu hình tại một thời điểm.

## Build an toàn

```powershell
.\drivers\voicemorph-virtual-audio\build-driver.ps1 `
  -WindowsDriverSamplesRoot C:\src\Windows-driver-samples `
  -Configuration Debug
```

Production packaging phải build `-Configuration Release`, gửi package qua quy trình Microsoft driver signing,
rồi đặt thư mục package đã ký vào `VOICEMORPH_SIGNED_DRIVER_DIR`. `npm.cmd run
tauri:build:driver-installer` sẽ từ chối artifact unsigned/test-signed, sai identity hoặc sai version. Xem
`docs/INSTALLER_DRIVER_INTEGRATION.md`.

Script chỉ chuẩn bị/build package trong `obj/` và `artifacts/`; script không cài driver, không ký driver và không
thay đổi cấu hình boot/security. Cần WDK đầy đủ tương thích với Visual Studio. `driver-package.json` là contract
metadata có thể kiểm thử, không thay thế INF hay catalog do WDK tạo.

Dùng `-PrepareOnly` để xác minh pinned source và overlay mà không compile; chế độ này cũng không phải bằng chứng
driver build thành công.

Microsoft sample được sử dụng theo Microsoft Public License; xem `LICENSE.MS-PL.txt` và
`THIRD_PARTY_NOTICES.md`.

Xem `docs/VIRTUAL_DRIVER_ARCHITECTURE.md`, `docs/DRIVER_DEVELOPMENT_SETUP.md` và
`docs/VIRTUAL_DRIVER_PROTOTYPE.md`.
