# Thiết lập Windows

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

## Khởi chạy sau khi cài đủ

```powershell
npm.cmd install
npm.cmd run tauri:dev
```

Build x64:

```powershell
npm.cmd run tauri:build:windows
```

Tauri hỗ trợ chính thức Windows target `x86_64-pc-windows-msvc`. WebView2 Evergreen Runtime được
dùng để render React UI.
