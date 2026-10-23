# Known Issues

## RR-001 — DSP latency vượt mục tiêu (High)

Benchmark release đo algorithmic latency 58.00 ms, chưa gồm driver/endpoint. Tái hiện bằng benchmark trong `TEST_PLAN.md`. Cần profile từng stage và giảm window/lookahead hoặc đánh giá processor khác.

## RR-002 — Chưa đo end-to-end và dài hạn (High)

Chưa có loopback, test 30 phút/2 giờ, memory-growth và sleep/resume. Glitch, leak hoặc recovery lỗi có thể chưa xuất hiện trong smoke test ngắn.

## RR-003 — MSI build bị Windows Installer chặn (High)

Release EXE và NSIS installer được tạo thành công, nhưng WiX `light.exe` thất bại với `LGHT0217/LGHT0216`, `0x643`: Windows Installer Service không truy cập được. Tái hiện bằng `npm.cmd run tauri:build:windows`. Cần sửa/đăng ký lại Windows Installer service và chạy lại MSI validation.

## RR-004 — Chưa nghiệm thu ứng dụng ngoài (High)

VB-CABLE endpoint và pipeline thật PASS, nhưng Discord/OBS/VALORANT chưa được thao tác trong vòng Task 15. Cần chọn `CABLE Output` làm microphone và ghi nhận kết quả nghe/capture.

## RR-005 — RustSec dependency warnings (Medium)

`cargo audit` không phát hiện vulnerability nhưng báo 4 warning: `atty` unmaintained/unsound, `proc-macro-error` unmaintained, `glib` unsound. Trên Windows, `atty 0.2.14` đi qua `nnnoiseless -> clap 3.2.25`; cần theo dõi/nâng hoặc thay dependency noise suppression đã đánh giá.

## RR-006 — Chưa cấu hình updater ký số (Medium)

Tauri updater chưa cấu hình. Auto-update tương lai phải dùng HTTPS manifest và artifact signing; Task 15 không thêm tính năng update.
