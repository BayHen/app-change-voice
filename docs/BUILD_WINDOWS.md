# Build Windows

## Yêu cầu

- Windows 10/11 x64, WebView2 Runtime.
- Node.js LTS và npm.
- Rust stable >= 1.90 với target `x86_64-pc-windows-msvc`.
- Visual Studio Build Tools: Desktop development with C++, MSVC và Windows SDK.
- WiX/NSIS được Tauri CLI quản lý khi bundle.

VB-CABLE là dependency tùy chọn cho virtual microphone. VoiceMorph Studio không đóng gói, cài ngầm hoặc gỡ driver này.

## Kiểm tra và build

```powershell
npm.cmd ci
npm.cmd run version:check
npm.cmd run test
npm.cmd run build
cargo test --manifest-path src-tauri/Cargo.toml
npm.cmd run tauri:build:windows
```

Artifacts x64:

- `src-tauri/target/x86_64-pc-windows-msvc/release/voicemorph-studio.exe`
- `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/*-setup.exe`
- `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/msi/*.msi`

NSIS dùng `currentUser`, không yêu cầu elevation và mặc định tạo Start Menu/uninstall registration theo Tauri. Dữ liệu người dùng nằm trong app config/data/cache, không nằm trong thư mục cài đặt nên update/uninstall binary không chủ động xóa preset, settings, metadata hoặc recording.

## Signed updater artifact

Chỉ chạy khi đã có key thật:

```powershell
$env:VOICEMORPH_UPDATER_PUBLIC_KEY="<public key content>"
$env:TAURI_SIGNING_PRIVATE_KEY="<secure path or secret content>"
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD="<password if configured>"
npm.cmd run tauri -- build --target x86_64-pc-windows-msvc --bundles nsis --config src-tauri/tauri.release.conf.json
```

Không lưu private key trong repository hoặc `.env`. `createUpdaterArtifacts` chỉ bật trong release config để build dev không giả vờ là signed release.
