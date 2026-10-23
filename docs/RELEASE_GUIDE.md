# Release Guide

VoiceMorph Studio dùng Semantic Versioning. `MAJOR` cho thay đổi không tương thích, `MINOR` cho tính năng tương thích và `PATCH` cho bug/security fix tương thích.

## Checklist

1. Chọn version và cập nhật đồng thời `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`.
2. Cập nhật `CHANGELOG.md`; chạy `npm.cmd run version:check`.
3. Chạy quality gate trong `docs/TEST_PLAN.md` và đóng release blockers.
4. Kiểm tra identifier vẫn là `com.voicemorph.studio`. Không đổi sau khi phát hành.
5. Tạo updater signing key bằng Tauri CLI trên máy bảo mật; backup key ngoài repository.
6. Thêm GitHub Secrets: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `VOICEMORPH_UPDATER_PUBLIC_KEY`.
7. Nếu có Windows code-signing certificate, cấu hình certificate/sign command bằng GitHub Secrets. Updater signing và Authenticode code signing là hai cơ chế khác nhau.
8. Push tag chính xác, ví dụ `v0.1.0`. Workflow luôn tạo **draft release**, không tự public.
9. Tải artifact draft, kiểm tra chữ ký updater, cài/upgrade trên Windows sạch rồi mới Publish thủ công.

CI thường chạy test và build EXE/NSIS/MSI. Release workflow chạy theo tag, yêu cầu secrets, tạo signed NSIS updater artifact và GitHub `latest.json` trong draft release.

## Signing status hiện tại

- Updater: code/config hoàn tất nhưng chưa có key/release signed test thật.
- Updater Rust dependency tắt feature ZIP v1 không dùng; bản phát hành dùng native Tauri v2 artifacts để tránh dependency `zip 4.6.1` có RustSec advisories.
- Windows Authenticode: chưa có certificate; SmartScreen có thể cảnh báo Unknown Publisher.
- MSI local: đang bị Windows Installer Service/WiX `LGHT0217/LGHT0216` trên máy thử nghiệm. NSIS là installer chính cho đến khi blocker được sửa.
