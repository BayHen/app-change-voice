# Thiết lập Windows

Hướng dẫn cài và cấu hình virtual audio cable cho Task 11 nằm tại [VIRTUAL_MIC_WINDOWS.md](VIRTUAL_MIC_WINDOWS.md).

Nếu WiX báo `LGHT0217`/`0x643`, kiểm tra Windows Installer bằng `Get-Service msiserver` trong PowerShell
Administrator và bảo đảm service không bị policy hoặc Windows component lỗi chặn. NSIS có thể vẫn được tạo
thành công dù bước MSI thất bại; không dùng file MSI cũ còn sót trong `target` để kết luận build mới đã đạt.

## Global hotkeys và system tray

Task 12 dùng `tauri-plugin-global-shortcut` 2.4.1. Plugin yêu cầu Rust 1.90 trở lên; toolchain stable hiện tại
phải đáp ứng trước khi build. Nếu Save hotkeys báo conflict, tổ hợp đã được Windows hoặc ứng dụng khác đăng
ký; bộ shortcut cũ vẫn được giữ. Không chạy nhiều instance VoiceMorph khi kiểm thử registrations.

Khi Close to tray bật, nút đóng chỉ ẩn cửa sổ. Chọn **Exit VoiceMorph Studio** từ tray để giải phóng audio
streams và unregister hotkeys trước khi thoát hoàn toàn.

## Kết quả kiểm tra ngày 22/10/2026

| Thành phần            | Trạng thái trên máy hiện tại | Ghi chú                                   |
| --------------------- | ---------------------------- | ----------------------------------------- |
| Node.js               | Có — v24.13.0                | Hoạt động                                 |
| npm                   | Có — v11.6.2                 | Dùng `npm.cmd`; PowerShell chặn `npm.ps1` |
| Git                   | Có                           | Repository đã tồn tại, chưa có commit     |
| WebView2 Runtime      | Có — 155.0.4283.62           | Phát hiện ở thư mục cài đặt chuẩn         |
| Rust / Cargo / rustup | Có — rustc/cargo 1.99.1      | Stable `x86_64-pc-windows-msvc`           |
| MSVC Build Tools      | Có — Build Tools 2026        | Hoạt động                                 |
| Windows SDK           | Có — 10.0.26100.0            | Hoạt động                                 |
| LLVM / libclang       | Có — 23.1.4                  | Bindgen cho Signalsmith Stretch           |

## Cài dependency còn thiếu

### 1. Visual Studio Build Tools và Windows SDK

Tải **Build Tools for Visual Studio 2022**, chọn workload **Desktop development with C++**. Bảo đảm
các component MSVC x64/x86 và Windows 10 hoặc Windows 11 SDK được chọn.

Trang chính thức: <https://visualstudio.microsoft.com/visual-cpp-build-tools/>

### 2. Rust stable MSVC

```powershell
winget install --id Rustlang.Rustup
rustup default stable-msvc
rustup target add x86_64-pc-windows-msvc
```

Đóng và mở lại VS Code/terminal sau khi cài. Xác minh bằng:

```powershell
rustc --version
cargo --version
rustup show active-toolchain
```

Hướng dẫn Tauri chính thức: <https://v2.tauri.app/start/prerequisites/>

### 3. MSI và VBSCRIPT

Cấu hình tạo cả NSIS `.exe` và MSI. Nếu build MSI báo `failed to run light.exe`, mở
**Settings → Apps → Optional features → More Windows features** và bật VBSCRIPT theo hướng dẫn Tauri.

### 4. LLVM/Clang cho DSP

Task 06 dùng `signalsmith-stretch` 0.1.3. Build script của wrapper sinh FFI binding bằng Bindgen nên
cần `libclang.dll`. Cài LLVM rồi mở lại terminal/VS Code:

```powershell
winget install --id LLVM.LLVM --exact
```

Nếu LLVM không ở đường dẫn mặc định, đặt `LIBCLANG_PATH` trỏ tới thư mục chứa `libclang.dll`.

## Khởi chạy sau khi cài đủ

```powershell
npm.cmd install
npm.cmd run tauri:dev
```

Các hook `beforeDevCommand` và `beforeBuildCommand` trong `tauri.conf.json` cũng gọi trực tiếp
`npm.cmd`. Cấu hình này tránh việc PowerShell chọn `npm.ps1` và thất bại do Execution Policy.

Script `tauri:dev` dùng riêng `src-tauri/target-dev` cho Cargo incremental cache. Cách này tránh lỗi
`failed to garbage collect ... Access is denied (os error 5)` khi thư mục `target` trước đó được tạo bởi
CI, sandbox hoặc một tài khoản Windows khác. Release build vẫn dùng `src-tauri/target` như bình thường.

Nếu Tauri báo `beforeDevCommand terminated with a non-zero status code`, kiểm tra port cố định 1420:

```powershell
Get-NetTCPConnection -LocalPort 1420 -State Listen
```

Đóng Vite/Tauri dev cũ đang giữ port rồi chạy lại; không chạy riêng `npm.cmd run dev` đồng thời với
`npm.cmd run tauri:dev` vì Tauri tự khởi động Vite.

Nếu log có `EPERM: operation not permitted, unlink node_modules\\.vite\\deps\\...`, đóng ứng dụng
VoiceMorph/Vite đang chạy và đóng các tab editor đang mở trực tiếp file trong `node_modules/.vite`.
Project hiện cô lập Vite cache theo tài khoản Windows tại `.vite-cache/<username>`, vì vậy tài khoản desktop
không cần unlink cache do CI/sandbox tạo. Với cache cũ từ phiên bản trước, không cần xóa
`node_modules/.vite`; chỉ cần chạy lại lệnh sau:

```powershell
npm.cmd run tauri:dev
```

Nếu WebView2 báo `0x800700AA: The requested resource is in use` ngay sau dòng
`Running target\debug\voicemorph-studio.exe`, hãy chạy lệnh từ terminal của phiên desktop Windows đang
đăng nhập (VS Code/PowerShell của chính người dùng), không chạy qua Windows service, tài khoản automation
cô lập hoặc session không tương tác. Có thể kiểm tra tài khoản hiện tại bằng `whoami`. Trên môi trường dự án
đã xác minh, cùng executable khởi động bình thường trong phiên desktop tương tác; đổi hoặc xóa WebView2
profile không khắc phục lỗi khi tiến trình vẫn chạy trong session cô lập.

Build x64:

```powershell
npm.cmd run tauri:build:windows
```

Tauri hỗ trợ chính thức Windows target `x86_64-pc-windows-msvc`. WebView2 Evergreen Runtime được
dùng để render React UI.

# Khắc phục MSI/WiX LGHT0217

Nếu NSIS build thành công nhưng MSI dừng ở `light.exe` với `LGHT0217` và thông báo Windows Installer
Service không truy cập được, kiểm tra service `msiserver` trong `services.msc`, khởi động lại Windows
Installer và chạy terminal với quyền phù hợp. Có thể dùng bộ cài NSIS x64 đã tạo trong khi sửa môi trường
MSI; lỗi này xảy ra sau khi Rust executable đã link thành công.
