# Thiết lập OBS Studio với VoiceMorph Virtual Microphone

1. Xác nhận `VoiceMorph Virtual Microphone` hoạt động trong Windows Sound Settings.
2. Start Routing trong VoiceMorph.
3. OBS → Sources → Add → Audio Input Capture → chọn `VoiceMorph Virtual Microphone`.
4. Không thêm cùng endpoint đồng thời trong Global Audio Devices và Sources; điều này có thể tạo duplicate/echo.
5. Xác nhận meter, ghi ít nhất 30 giây, nghe file recording và đổi Natural/Deep/Robot trong lúc ghi.
6. Khi test Discord + OBS, giữ output của cả hai ở headphone vật lý và xác nhận đóng một ứng dụng không dừng route.

Meter chuyển động chỉ chứng minh có level; PASS yêu cầu file recording chứa processed voice nghe được.

## Trạng thái kiểm thử 24/10/2026

OBS Studio `32.1.0` có cài trên máy test nhưng capture/recording bị **BLOCKED**: driver chưa được ký/cài và Windows
không có VoiceMorph capture endpoint. OBS không được mở để tạo kết quả giả từ thiết bị khác.
