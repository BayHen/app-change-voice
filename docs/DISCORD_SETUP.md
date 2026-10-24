# Thiết lập Discord với VoiceMorph Virtual Microphone

## Điều kiện

Windows Sound Settings và trang Virtual Microphone của VoiceMorph phải cùng hiển thị `VoiceMorph Virtual
Microphone`; trạng thái bridge phải là Connected và routing là Streaming. `Installed` hoặc việc thiết bị xuất hiện
trong danh sách chưa chứng minh Discord nhận được audio.

## Cấu hình

1. Dùng headphone vật lý để tránh feedback.
2. VoiceMorph: chọn microphone vật lý, chọn preset, Start Monitoring nếu cần nghe local, Connect rồi Start Routing.
3. Discord → User Settings → Voice & Video:
   - Input Device: `VoiceMorph Virtual Microphone`.
   - Output Device: headphone vật lý.
4. Tắt hoặc ghi lại trạng thái Discord Noise Suppression, Automatic Gain Control và Input Sensitivity để kết quả
   Natural/Deep/Robot có thể lặp lại.
5. Chạy Mic Test và xác nhận bằng nghe thực tế, không chỉ nhìn meter.
6. Kiểm tra mute, chuyển Natural → Deep → Robot, restart Discord và xác nhận routing của VoiceMorph vẫn chạy.

Không chọn microphone vật lý trực tiếp trong Discord khi đang nghiệm thu processed route. VoiceMorph không dùng
Discord API và không tự sửa cấu hình Discord.

## Trạng thái kiểm thử 24/10/2026

Discord `1.0.9265` có cài trên máy test nhưng toàn bộ Mic Test bị **BLOCKED**: VoiceMorph driver chưa được ký/cài,
Windows không có capture endpoint tương ứng. Không dùng VB-CABLE để thay thế trong Phase 11.2.6.
