# Thiết lập VALORANT với VoiceMorph Virtual Microphone

1. Xác nhận `VoiceMorph Virtual Microphone` hoạt động trong Windows Sound Settings trước khi mở game.
2. VoiceMorph: Start Routing với microphone vật lý và preset mong muốn.
3. VALORANT → Settings → Audio → Voice Chat:
   - Input Device: `VoiceMorph Virtual Microphone`.
   - Output Device: headphone vật lý.
4. Chạy Loopback Test, sau đó kiểm tra Push-to-Talk, Party/Team Voice, mute và chuyển preset khi game đang mở.
5. Đóng/mở lại game và xác nhận VoiceMorph Audio Engine không bị dừng.

VoiceMorph chỉ dùng Windows Audio APIs chuẩn. Không inject code, hook process, đọc bộ nhớ, thay đổi file game hoặc
can thiệp/bypass Riot Vanguard.

## Trạng thái kiểm thử 24/10/2026

VALORANT/Riot Client không được phát hiện trên máy test và VoiceMorph virtual endpoint chưa tồn tại. Các test game
được ghi **BLOCKED**, không mô phỏng và không dùng VB-CABLE thay cho driver riêng.
