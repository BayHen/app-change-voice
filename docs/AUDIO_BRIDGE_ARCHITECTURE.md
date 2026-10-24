# Kiến trúc Rust-to-Driver Audio Bridge

## Luồng dữ liệu

`CPAL input -> AudioPipeline -> gain/limiter -> processed fan-out -> SPSC ring -> bridge worker -> buffered IOCTL -> kernel PCM ring -> WaveRT capture endpoint`.

Điểm fan-out nằm sau pipeline DSP hiện hữu. Monitoring, recording và virtual microphone dùng cùng một mẫu processed; không tạo AudioPipeline thứ hai và không gửi frame qua React/Tauri IPC.

## Transport đã chọn

Bridge dùng Windows device interface GUID ổn định `{B566F7B7-91D8-4F31-9712-94C55A10E52A}` và `DeviceIoControl` với `METHOD_BUFFERED`.

- Không chọn shared memory vì prototype sẽ phải quản lý section mapping, ACL, lifetime và memory ordering giữa user/kernel phức tạp hơn.
- Không dùng `METHOD_NEITHER`: driver không giữ hoặc dereference con trỏ user-mode.
- Mọi length, protocol version, message type và format đều được kiểm tra trước khi copy.
- Interface ACL cho SYSTEM/Admin toàn quyền và Authenticated Users read/write; IOCTL chỉ cho phép format/packet bị giới hạn, không expose kernel memory.
- Một `FILE_OBJECT` duy nhất sở hữu producer. Cleanup handle giải phóng owner và xóa ring.

## Realtime và format

- DSP nội bộ: Float32 mono.
- Wire/endpoint: 48.000 Hz, mono, signed PCM32.
- Packet: 480 frames (10 ms). User queue và kernel queue đều 4.096 frames, bounded.
- Callback CPAL chỉ `push` vào SPSC ring và tăng atomic counter khi đầy; không lock, I/O, allocation hoặc Win32 call.
- Worker Rust preallocate packet, sanitize NaN/Infinity, clamp và chuyển Float32 sang PCM32.
- WaveRT lấy dữ liệu dưới spin lock ngắn. Underrun được zero-fill và đếm; overflow drop phần vượt capacity và đếm.

Bridge được ưu tiên khi driver interface tồn tại. Nếu không có driver, routing CPAL/VB-CABLE cũ vẫn là fallback để tránh regression. Stop session drop worker, đóng handle và driver cleanup reset producer.

## Giới hạn hiện tại

Không có resampler trong bridge v1: session phải chạy 48 kHz. Chưa cài/ký driver trên test target nên chưa xác minh capture endpoint, recovery sau device removal hoặc round-trip latency. Ring capacity là giới hạn an toàn, không phải latency mục tiêu; độ trễ thực phụ thuộc fill level và lịch WaveRT.
