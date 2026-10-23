# Tích hợp Discord qua microphone ảo

VoiceMorph Studio không dùng Discord Bot/API. Ứng dụng xử lý microphone cục bộ rồi phát processed audio vào
playback endpoint của VB-CABLE. Discord đọc capture endpoint tương ứng như một microphone Windows bình thường.

```text
Physical microphone
  → RNNoise / noise gate
  → Pitch + formant
  → Voice effects / preset
  → Input gain
  → bounded routing queue
  → limiter
  → CABLE Input (playback)
  → VB-CABLE driver
  → CABLE Output (capture)
  → Discord
```

Original microphone samples chỉ được dùng cho track Original của recording A/B. Nhánh routing nhận sample sau
DSP và input gain; không có capture hoặc processor chain thứ hai dành cho Discord.

## Cấu hình

1. Trong Dashboard chọn microphone vật lý và headphone, sau đó Start Monitoring.
2. Settings → Discord Integration → chọn `CABLE Input (VB-Audio Virtual Cable)`.
3. Nhấn Start Routing và nói thử. Meter processed signal phải chuyển động.
4. Discord → User Settings → Voice & Video:
   - Input Device: `CABLE Output (VB-Audio Virtual Cable)`.
   - Output Device: headphone vật lý.
5. Chạy Mic Test. Chỉ kết quả Mic Test/meter của Discord mới xác minh Discord nhận audio.

## Xử lý âm thanh của Discord

Khi đánh giá preset, nên tạm tắt Discord Noise Suppression và Automatic Gain Control vì chúng có thể che hoặc
làm thay đổi hiệu ứng VoiceMorph. Sau khi so sánh xong có thể bật lại nếu cần. Điều chỉnh Input Sensitivity để
giọng nhỏ vẫn mở gate nhưng tiếng nền không kích hoạt liên tục. VoiceMorph không tự sửa các cài đặt này.

## Tránh feedback

- Không chọn `CABLE Output` làm input của VoiceMorph; input phải là microphone vật lý.
- Discord output phải là headphone, không phải `CABLE Input`.
- Hear Myself là tùy chọn độc lập và nên dùng headphone.

## Trạng thái và lỗi

- **Device available:** tìm thấy endpoint có fingerprint/ID hiện tại; chưa có stream.
- **Routing active:** playback stream đã mở; chưa đồng nghĩa Discord đã chọn capture endpoint.
- **Audio signal detected:** peak processed output lớn hơn -55 dBFS trong lần poll hiện tại.
- **Device missing:** refresh không tìm thấy endpoint đã lưu; chọn lại endpoint sau khi cài/reconnect driver.
- **Routing error:** stream callback báo fault hoặc endpoint biến mất; Stop, Refresh, chọn lại và Start Monitoring.

## Checklist nghiệm thu thủ công

Thử Natural, Deep và Robot; bật noise suppression; thay volume/mute; chuyển preset bằng hotkey; Stop/Start
routing; đóng/mở Discord. Với mỗi trường hợp ghi nhận Discord Mic Test còn nhận đúng processed voice. Việc đóng
Discord không được làm dừng VoiceMorph vì ứng dụng chỉ sở hữu playback endpoint của virtual cable.
