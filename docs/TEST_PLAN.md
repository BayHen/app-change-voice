# Kế hoạch kiểm thử trước phát hành

Task 15 kiểm tra regression frontend, backend Rust, DSP, WASAPI và VB-CABLE. Test tự động không thay thế nghiệm thu nghe hoặc đo loopback.

| Nhóm             | Lệnh                                                                                        | Loại          | Điều kiện đạt                         |
| ---------------- | ------------------------------------------------------------------------------------------- | ------------- | ------------------------------------- |
| React/TypeScript | `npm.cmd run test`, `npm.cmd run build`                                                     | Tự động       | Không test fail/lỗi TypeScript        |
| Chất lượng code  | `npm.cmd run lint`, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`        | Tự động       | Không error/warning                   |
| Rust/DSP         | `cargo test --manifest-path src-tauri/Cargo.toml`                                           | Tự động       | Unit/regression quan trọng PASS       |
| Benchmark DSP    | `cargo test --release ... measures_algorithm_latency_and_cpu_time -- --ignored --nocapture` | Tổng hợp      | Output hữu hạn, nhanh hơn realtime    |
| Windows/WASAPI   | `cargo test --release ... manual_hardware_tests -- --ignored --nocapture`                   | Phần cứng     | Mở/đóng stream và endpoint thành công |
| Dependency       | `npm audit`, `cargo audit`                                                                  | Advisory      | Không vulnerability đã biết           |
| Tauri runtime    | `npm.cmd run tauri:dev`                                                                     | Desktop smoke | Window mở, không lỗi khởi tạo         |
| Installer        | `npm.cmd run tauri:build:windows`                                                           | Release       | EXE và installer được tạo             |

## Phạm vi đã tự động hóa

- Device selection/fallback/no-device, sample conversion, ring-buffer overflow/underflow.
- Gain, mute, meter, limiter, noise và dữ liệu không hữu hạn.
- Pitch/formant, latency-aligned bypass và chuyển toàn bộ built-in preset.
- WAV 5/30 giây bằng tín hiệu kiểm thử, recording queue đầy và lỗi ghi đĩa.
- Routing queue, cấu hình, VB-CABLE open/stop và sample-rate mismatch.
- Plugin/preset validation, import/export, path traversal, backup/restore.
- Hotkey config, tray decision, bounded soundboard mixer.
- Diagnostics serialization, temporary-file export và không có payload audio/secret.

## Bắt buộc kiểm thử thủ công

- Nghe Natural/Deep/Robot, mute, suppression và soundboard trên tai nghe.
- Discord Mic Test, OBS capture và VALORANT voice chat.
- Loopback đo end-to-end latency bằng fixture phù hợp.
- Buffer 128/512 frames nếu driver hỗ trợ.
- Disconnect/reconnect, sleep/resume và đổi endpoint trong lúc stream chạy.
- Phiên 30 phút/2 giờ, theo dõi Working Set/Private Bytes và counters đầu-cuối.
- Tray, global hotkey khi mất focus và cleanup khi Exit.

Không đánh dấu PASS cho các mục này nếu chỉ chạy tín hiệu tổng hợp hoặc đọc code.
