# AI Voice Conversion — Task 17

## Lựa chọn kỹ thuật

Task 17 chọn **OpenVoice V2** làm prototype offline. Mã nguồn OpenVoice phát hành theo MIT; revision
được khóa tại `74a1d147b17a8c3092dd5430504bd83ef6c7eb23` để môi trường có thể tái lập.

| Hướng                | Đánh giá                                                         | Quyết định                                               |
| -------------------- | ---------------------------------------------------------------- | -------------------------------------------------------- |
| OpenVoice V2         | Tone-colour conversion, đa ngôn ngữ; cần PyTorch/checkpoint      | Prototype offline, MIT                                   |
| RVC retrieval-based  | Tốt khi có voice model; training/index và quyền weights phức tạp | Không bundle                                             |
| Seed-VC              | Có biến thể streaming nhưng cần benchmark phần cứng              | Không chọn mặc định vì GPL-3.0                           |
| ONNX Runtime / WinML | Hợp với native Windows, CPU/GPU provider                         | Mục tiêu production; chưa có artifact export đã xác minh |

Không có weights trong repository/installer. License code không thay thế quyền dùng giọng nói/checkpoint.

## Kiến trúc

```text
Realtime callback                 Background AI worker
DSP output frame ── try_send ──> bounded queue ──> inference
       │                                               │
       └── DSP fallback <── try_recv / unavailable ────┘
```

- `AIInferenceService` dùng `sync_channel(2)`, frame `[f32; 2048]` cố định và `try_send/try_recv`.
- `AIProcessingAdapter` là ranh giới realtime-safe, chưa nối CPAL vì chưa có native backend/model xác minh.
- `AIModelManager` lưu tại `app_data_dir/ai-models`, kiểm tra schema, path, `.onnx`, license, 2 GiB và SHA-256.
- `.pth/.pt` không được import vào app vì pickle PyTorch có thể thực thi mã khi deserialize.
- Prototype OpenVoice chạy ngoài Tauri, bắt buộc audio do người dùng chọn và `--acknowledge-rights`.

UI có các profile tổng quát Deep, Bright, Soft, Masculine-style, Feminine-style và Synthetic Character;
không mô phỏng cá nhân/người nổi tiếng. Controls disabled khi model/runtime không hỗ trợ.

## Trạng thái đo lường

- PCM16 pre/post-processing, bounded worker/overflow và manifest validation: đã có unit test.
- OpenVoice inference thật: **NOT TESTED** — máy chỉ có Python 3.14; chưa có Python 3.11,
  PyTorch/OpenVoice/checkpoint và cặp audio xác nhận quyền sử dụng.
- GPU, CPU/memory, intelligibility, virtual routing, end-to-end latency: **NOT TESTED**.
- Realtime: **DEFERRED**. UI ghi offline prototype và DSP fallback, không công bố latency suy đoán.

Xem `ai-prototype/README.md` để chạy. Chỉ dùng audio tự thu/có sự đồng ý. Chỉ chọn CUDA sau khi
`torch.cuda.is_available()` thành công. JSON benchmark là inference wall-clock, không phải loopback latency.
