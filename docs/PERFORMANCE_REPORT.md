# Báo cáo hiệu năng

## Release baseline procedure

P0 bounded the soundboard command batch to the pending count at input callback entry. With the existing 64-entry queue, a concurrent producer cannot extend the batch beyond that snapshot. Later commands remain queued for a subsequent callback. The change removes command polling from per-sample DSP processing. Real callback execution time, contention, underruns, simultaneous-stream behavior and long-session stability remain Not Measured.

A P0 release-mode benchmark on Windows x64 processed 10 seconds of synthetic 48 kHz audio at pitch +6 and formant +3 semitones in 161.38 ms wall time, with an algorithmic delay of 58.00 ms. This single run reported a processing-to-input duration ratio of 0.016. It did not exercise the CPAL callback, soundboard burst contention, OS audio latency or end-to-end loopback.

```powershell
cargo test --locked --manifest-path src-tauri/Cargo.toml --release measures_algorithm_latency_and_cpu_time -- --ignored --nocapture
```

Current audit results for startup, process CPU and memory, end-to-end latency, dropouts, current installer size and long-session stability are **Not Measured**. The historical figures below describe a different run and must be reproduced before using them in release material.

1. Record commit, release profile, Windows build, CPU, RAM, device and driver versions, actual sample rate, negotiated buffers and active effects.
2. Time process launch through interactive UI over five cold starts. Sample process CPU and private bytes once a second for five minutes at idle and under active processing.
3. Test 48 kHz at 128, 256 and 512 frames where negotiated. Compare natural and shifted presets, then add noise suppression, routing, soundboard and monitoring. Record counter deltas, callback estimates and audible dropouts per run.
4. Measure end-to-end delay with an authorized reference signal and destination capture on a common clock or calibrated loopback fixture. Cross-correlate source and capture, repeat and publish median and p95 separately from the DSP estimate.
5. Run 30-minute and two-hour sessions with preset switches, recording and device recovery. Log process memory slope, buffer counters and recovery time. Record artifact bytes and SHA-256 from the release build.

Keep logs local and record aggregate measurements without storing microphone content unless the test explicitly requires authorized audio.

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
