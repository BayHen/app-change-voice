# VoiceMorph Studio

Ứng dụng desktop Windows để xây dựng trải nghiệm biến đổi giọng nói. Task 01 tạo nền móng
Tauri 2 + React + TypeScript; chưa có audio engine hoặc DSP thực tế.

## Công nghệ

- Tauri 2 / Rust stable (MSVC)
- React 19, TypeScript, Vite
- Tailwind CSS 4 qua plugin Vite chính thức
- ESLint, Prettier, Vitest và `rustfmt`

## Yêu cầu Windows

1. Node.js LTS và npm.
2. Rust stable với target `x86_64-pc-windows-msvc`.
3. Visual Studio 2022 Build Tools, workload **Desktop development with C++**.
4. Windows 10/11 SDK và Microsoft Edge WebView2 Runtime.

Chi tiết cài đặt và trạng thái máy hiện tại xem tại [docs/SETUP_WINDOWS.md](docs/SETUP_WINDOWS.md).

## Chạy dự án

```powershell
npm.cmd install
npm.cmd run tauri:dev
```

Do Execution Policy trên máy hiện tại chặn `npm.ps1`, tài liệu dùng `npm.cmd`. Nếu terminal của bạn
không gặp lỗi này, `npm` và `npm.cmd` là tương đương.

## Kiểm tra

```powershell
npm.cmd run lint
npm.cmd run format
npm.cmd run test
npm.cmd run build
npm.cmd run rust:fmt
npm.cmd run rust:check
npm.cmd run rust:test
```

Build bộ cài Windows x64:

```powershell
npm.cmd run tauri:build:windows
```

Output dự kiến nằm trong `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/`.

## Cấu trúc chính

```text
src/                    React UI
  components/           Thành phần dùng lại
  hooks/                React hooks
  pages/                Màn hình ứng dụng
  services/             Cầu nối Tauri command
  styles/               Tailwind và global styles
  types/                Kiểu TypeScript
src-tauri/src/           Rust backend
  audio/                 Audio engine độc lập nền tảng
  effects/               DSP độc lập nền tảng
  platform/              Adapter hệ điều hành
  presets/               Domain model cho preset
tests/                   Vitest
docs/                    Tài liệu kỹ thuật
```

## Tài liệu

- [Kiến trúc](docs/ARCHITECTURE.md)
- [Cài đặt Windows](docs/SETUP_WINDOWS.md)
- [Tiến độ Task 01](docs/TASK_PROGRESS.md)
- [Quyết định kỹ thuật](docs/DECISIONS.md)
