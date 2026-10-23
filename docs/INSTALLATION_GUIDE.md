# Installation Guide

## Cài đặt

1. Tải NSIS x64 installer từ GitHub Release chính thức.
2. Xác minh checksum/chữ ký do release cung cấp.
3. Chạy installer. Bản NSIS cài cho current user và không cần quyền Administrator.
4. Mở VoiceMorph Studio từ Start Menu hoặc shortcut được chọn trong installer.
5. Cho phép microphone trong Windows Privacy & security → Microphone khi cần monitoring/recording.

Nếu chưa có Authenticode certificate, Windows SmartScreen có thể hiển thị Unknown Publisher. Chỉ tiếp tục nếu artifact đến từ release chính thức và checksum khớp.

## Virtual microphone

VB-CABLE không đi kèm installer. Người dùng chủ động cài từ nhà cung cấp, có thể cần quyền quản trị vì đây là audio driver. Sau đó chọn CABLE Input trong VoiceMorph Studio và CABLE Output trong Discord/OBS. Xem `VIRTUAL_MIC_WINDOWS.md`.

## Nâng cấp và dữ liệu

Update thay binary trong thư mục cài đặt. Presets, settings, soundboard metadata và recordings được lưu trong thư mục application data/config/cache, không bị installer chủ động xóa. Luôn tạo backup trước bản nâng cấp lớn.

## Gỡ cài đặt

Dùng Settings → Apps → Installed apps → VoiceMorph Studio → Uninstall. VB-CABLE là phần mềm bên thứ ba và không bị gỡ theo VoiceMorph Studio. Dữ liệu người dùng không bị xóa ngầm; nếu muốn xóa hoàn toàn, backup trước rồi xóa thủ công thư mục app data sau khi xác nhận.
