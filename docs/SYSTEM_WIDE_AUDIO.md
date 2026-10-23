# System-Wide Voice Changer

VoiceMorph dùng một microphone capture và một DSP pipeline chung. Processed samples được fan-out tới monitoring,
recording và virtual output; việc mở/đóng ứng dụng đích không tạo hoặc hủy engine.

```text
Physical Mic → Capture → Noise → Pitch/Formant → Effects → Gain → routing queue → Limiter → CABLE Input
                                                                                           ↓ driver
Games / Discord / OBS / Zoom / Teams ← CABLE Output
```

Profiles được chọn thủ công và lưu tại `app_config_dir/application-profiles.json`. Apply profile thay preset,
noise suppression và limiter bằng atomic controls, không restart DSP. Tự động nhận diện process không được bật.
VoiceMorph không inject/hook process, đọc memory, sửa game file hay tương tác Vanguard.

Routing Active chỉ xác nhận playback endpoint mở. Mỗi ứng dụng phải tự chọn `CABLE Output`, và trạng thái ứng
dụng nhận signal chỉ được xác minh qua meter/recording của chính ứng dụng đó.
