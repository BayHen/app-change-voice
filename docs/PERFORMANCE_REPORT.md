# Báo cáo hiệu năng

Ngày đo: 2026-10-23. Build: Rust `--release`, Windows x64.

## Đã đo

Benchmark tín hiệu tổng hợp 48 kHz, pitch `+6` và formant `+3` semitone:

| Chỉ số                  |                      Kết quả |
| ----------------------- | ---------------------------: |
| Dữ liệu audio           |                      10 giây |
| Wall-clock xử lý        |                    143.34 ms |
| Tỷ lệ xử lý/realtime    | 0.014 (xấp xỉ 1.4% một core) |
| Algorithmic DSP latency |                     58.00 ms |

Đây là benchmark offline của processor, không phải CPU toàn tiến trình hay end-to-end latency. 58 ms vượt mục tiêu thử nghiệm 50 ms trước đây; cần tối ưu hoặc đổi thuật toán trước khi tuyên bố low-latency dưới 50 ms.

Hardware integration PASS với driver hiện có: start/stop hai chu kỳ, pipeline microphone thật sang VB-CABLE, open/stop virtual output và mismatch sample rate. Test ngắn không đại diện session dài.

Settings → Audio Diagnostics hiển thị sample rate, buffers, DSP/estimated latency, processing estimate, underrun/overflow và stream errors. Export JSON chỉ chứa metadata/counters. `estimatedLatencyMs` là phép tính buffer/ring/DSP, không phải loopback.

## Chưa đo

- End-to-end loopback latency: **NOT TESTED**, chưa có fixture.
- CPU/memory toàn tiến trình và memory growth: **NOT TESTED** trong phiên dài.
- Buffer 128/512 trên phần cứng: **NOT TESTED**; CPAL hiện yêu cầu 256 và driver thương lượng cấu hình thực tế.
- Noise + routing + soundboard + monitoring đồng thời dài hạn: **NOT TESTED**.
- Phiên 30 phút/2 giờ, sleep/resume và reconnect: **NOT TESTED**.
