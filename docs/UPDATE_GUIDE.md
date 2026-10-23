# Update Guide

## Kiến trúc

Settings → Application Updates sử dụng plugin updater chính thức của Tauri 2. Manifest được đọc qua HTTPS tại GitHub Releases. Update artifact bắt buộc vượt qua Minisign verification trước khi install; downgrade bị tắt và manifest version phải khớp version đã ký.

Build không có `VOICEMORPH_UPDATER_PUBLIC_KEY` hiển thị **Not configured** và khóa nút check. Đây là fail-closed behavior, không phải lỗi mạng.

## Tạo key

```powershell
npm.cmd run tauri signer generate -- -w C:\secure\voicemorph-updater.key
```

- Private key và password: secret manager/GitHub Actions Secrets.
- Public key: `VOICEMORPH_UPDATER_PUBLIC_KEY` tại build-time.
- Mất private key sẽ làm các bản đã phát hành không thể xác minh update mới; cần backup bảo mật.

Không commit `*.key`, password hoặc private-key content. Signature verification không được tắt.

## Luồng người dùng

1. Check for updates.
2. Xem version và release notes.
3. Download and install; progress được hiển thị.
4. Restart application sau khi plugin báo cài thành công.

Network error, invalid manifest, download failure và invalid signature được hiển thị như update error, không xóa bản đang chạy. Update không ghi vào app data/config/cache; schema migration phải validate và giữ dữ liệu cũ trước khi thay đổi schema.

## Test bắt buộc trước publish

- Signed update từ bản N lên N+1 trên Windows sạch.
- Tamper artifact/signature: phải bị reject.
- Endpoint offline/timeout: app vẫn chạy và báo lỗi.
- Restart sau update; version mới chính xác.
- Presets/settings/soundboard metadata/recordings còn nguyên.
- Uninstall/reinstall và rollback plan.

Chưa có signing secrets hoặc public test release thì phải ghi **NOT TESTED**, không được tuyên bố updater hoạt động end-to-end.
