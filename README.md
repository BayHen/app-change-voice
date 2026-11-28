# VoiceMorph Studio

[Bản sửa Soundboard: Play khi engine idle và lỗi hiển thị](docs/SOUNDBOARD_PLAYBACK_FIX.md).

[Female Voice Collection: tính năng, model hợp lệ và giới hạn kiểm thử](docs/FEMALE_VOICE_COLLECTION_REPORT.md).

[Audio quality audit, measurements and current limits](docs/AUDIO_QUALITY_REPORT.md).

**Hướng dẫn sử dụng app và cấu hình mic ảo cho Discord:**
[Mở hướng dẫn tiếng Việt](docs/HUONG_DAN_SU_DUNG.md).

Ứng dụng desktop Windows xử lý giọng nói local: preset DSP, pitch/formant, EQ, noise suppression,
soundboard WAV, ghi âm và xuất WAV. Mic ảo dùng cáp audio Windows đã cài như VB-CABLE để gửi
giọng đã xử lý sang Discord/ứng dụng khác. AI Voice Studio hỗ trợ runtime w-okada tùy chọn;
Classic DSP không cần Python hoặc model AI. Trạng thái triển khai và giới hạn kiểm thử được ghi
trong [docs/TASK_PROGRESS.md](docs/TASK_PROGRESS.md).

Người dùng bản EXE/bộ cài không cần Node.js hoặc Rust. Các yêu cầu và lệnh bên dưới dành cho
việc chạy source/build ứng dụng; hướng dẫn dùng app ở liên kết đầu trang.

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
5. LLVM/Clang (cần `libclang.dll` để build Rust wrapper của Signalsmith Stretch).

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

For the original Wind Assassin and Jade Guardian voice profiles, see the
[Windows RVC training and import guide](docs/HUONG-DAN-RVC-GIONG-GOC.md).
