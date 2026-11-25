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

Nếu Tauri báo `beforeDevCommand terminated with a non-zero status code`, kiểm tra port cố định 1420:

```powershell
Get-NetTCPConnection -LocalPort 1420 -State Listen
```

Đóng Vite/Tauri dev cũ đang giữ port rồi chạy lại; không chạy riêng `npm.cmd run dev` đồng thời với
`npm.cmd run tauri:dev` vì Tauri tự khởi động Vite.

Nếu log có lỗi EPERM khi Vite xóa cache cũ, kiểm tra `cacheDir` trong `vite.config.ts`.
Vite dùng thư mục `session-<pid>-<uuid>` riêng cho mỗi lần chạy dưới `.vite-cache/`.
Không dùng `USERNAME`/`USERDOMAIN` để phân biệt cache vì tiến trình sandbox có thể kế thừa
biến môi trường của tài khoản khác. Cache cũ được giữ nguyên; không cần đổi ACL hoặc chạy
terminal bằng Administrator. Mỗi phiên mới sẽ tối ưu dependency lại một lần.
Chạy lại:

```powershell
npm.cmd run tauri:dev
```

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

# Optional managed AI runtime

Classic DSP works without AI dependencies. In **AI Voice Studio** or **Settings → AI runtime**,
choose **Install recommended runtime**. Keep 13 GiB free; the pinned download is 3.90 GiB.
The application installs its own embedded Python, dependencies and HuBERT, with progress,
cancel/retry, integrity checks and atomic activation. End users do not install Python,
open a terminal, run pip, install CUDA drivers or start w-okada separately.

Choose **Detect hardware**, then optionally **Get official Tsukuyomi-chan voice**. Read
and acknowledge the original owner's custom terms before downloading/preparing it.
Select the voice and **Load & start AI**. This does not start the microphone; monitoring
and virtual output remain explicit controls. **Stop AI** keeps the AI voice muted;
**Use Classic DSP** explicitly restores the Classic voice. CPU fallback is opt-in.
After assets are installed, inference stays offline through private process pipes.

Runtime: `%APPDATA%\com.voicemorph.studio\ai-runtime`; models:
`%APPDATA%\com.voicemorph.studio\ai-models`. A local trusted package import is also
available. The base NSIS installer does not include the multi-GB runtime or voice weights.

The validated compatibility recipe currently uses **Python 3.10.11 (end of support)**.
A supported baseline and complete redistribution/security review remain release blockers.
Real CUDA/CPU inference has now been exercised with an authorized original-owner model
and sample; hardware loopback and conferencing tests remain separate. See
[runtime guide](../ai-sidecar/README.md) and the current
[execution, licensing and verification report](AI_RUNTIME_EXECUTION_REPORT.md).
