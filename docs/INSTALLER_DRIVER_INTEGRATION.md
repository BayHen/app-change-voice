# Tích hợp driver vào Windows Installer

## Kiến trúc phát hành

Phase 11.2.5 dùng NSIS x64 của Tauri 2 với chế độ `perMachine`. Product identifier vẫn là
`com.voicemorph.studio`. MSI thông thường không được dùng cho bản có driver vì quy trình driver cần hook,
elevation và xử lý mã trả về của PnPUtil rõ ràng.

Luồng build production:

1. Build driver Release x64 bằng WDK và tạo SYS/INF/CAT.
2. Gửi package qua Microsoft Hardware Dashboard/HLK phù hợp và nhận package Microsoft-signed.
3. `prepare-driver-release.ps1` kiểm tra identity, version, chữ ký Authenticode, kernel policy bằng SignTool và
   tạo SHA-256 manifest trong staging bị git-ignore.
4. Tauri nhúng staging vào NSIS qua `tauri.driver-release.conf.json`.
5. NSIS chạy elevated, kiểm tra Windows 10/11 x64, so version đang cài và gọi PnPUtil với lệnh cố định.
6. Release pipeline tạo draft; không tự public release.

Không có placeholder/test-signed binary trong source hoặc installer. Frontend không được truyền command line
hay đường dẫn tùy ý tới tiến trình elevated.

## Build

```powershell
$env:VOICEMORPH_SIGNED_DRIVER_DIR = 'D:\secure\voicemorph-driver-signed'
npm.cmd run driver:package:validate
npm.cmd run tauri:build:driver-installer
```

Output dự kiến:

- `release-artifacts/windows-x64/*-setup.exe`
- `release-artifacts/windows-x64/release-artifact-manifest.json`

Script dừng trước khi bundle nếu thiếu file, sai version/architecture, catalog không hợp lệ, chữ ký không hợp lệ
hoặc signer không phải Microsoft. Private certificate/key không được lưu trong repository.

## CI/CD

Release workflow yêu cầu URL riêng và SHA-256 của archive driver đã ký qua GitHub secrets:

- `VOICEMORPH_SIGNED_DRIVER_PACKAGE_URL`
- `VOICEMORPH_SIGNED_DRIVER_PACKAGE_SHA256`

Archive được kiểm checksum trước khi giải nén, sau đó vẫn phải vượt qua toàn bộ signature validation. Tauri updater
signing và Windows driver signing là hai trust chain độc lập.

## Trạng thái hiện tại

Release driver local build thành công, version `0.1.0.0`, Inf2Cat PASS nhưng CAT/SYS chưa Microsoft-signed.
Vì vậy production NSIS có driver và clean-install test đang **BLOCKED**, không có installer production nào được
tạo từ package unsigned.
