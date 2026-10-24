# Kiểm thử Audio Bridge

## Tự động đã chạy

- Rust format/check/test: ABI format, invalid sample sanitization và regression suite.
- Driver Debug x64 WDK build với warning-as-error.
- ApiValidator và Inf2Cat package validation.
- Static package contract: capture-only, interface GUID, IOCTL dispatch, WaveRT bridge read và không khôi phục sample tone.

## Quy trình test máy/VM ký hợp lệ

1. Ký test package theo chính sách Windows trên máy test; không tắt Secure Boot/signature enforcement trên máy làm việc.
2. Cài package, xác nhận `VoiceMorph Virtual Microphone` và bridge interface xuất hiện.
3. Chạy audio session 48 kHz, Start Routing, phát test signal có quyền sử dụng.
4. Capture endpoint bằng WASAPI/OBS; so sánh sample/FFT với nguồn và kiểm tra sequence/counters.
5. Lặp với microphone thật, Natural/Pitch/Noise Suppression/Mute; xác nhận monitoring và recording không đổi.
6. Gây underrun/overflow, đóng producer đột ngột, restart app, disable/enable device và xác nhận silence/recovery.
7. Chạy tối thiểu 30 phút rồi 2 giờ; ghi CPU, working set, dropped frames và handle count.
8. Đo loopback bằng marker/impulse để tách DSP latency, bridge queue latency và end-to-end latency.

## Trạng thái

Build/package validation là PASS. Capture PCM thật, pitch qua endpoint, disconnect/reconnect, long-running và đo latency end-to-end là **NOT TESTED** vì driver unsigned chưa được cài trên test environment. Không công bố mục tiêu dưới 50 ms khi chưa có phép đo loopback.
