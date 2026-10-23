# Thiết lập VALORANT

1. VoiceMorph: microphone vật lý → Start Monitoring → chọn `CABLE Input` → Start Routing.
2. VALORANT Audio/Voice Chat: chọn input `CABLE Output (VB-Audio Virtual Cable)`.
3. Output của game phải là headphone vật lý. Kiểm tra Team/Party Voice và input threshold.
4. Apply profile VALORANT thủ công; thử Natural/Deep và mute trước khi vào trận.

VoiceMorph chỉ dùng Windows Audio API. Không inject, hook, sửa file, bypass hoặc can thiệp Riot Vanguard.
Nếu VALORANT không thấy endpoint, đóng/mở lại game sau khi VB-CABLE đã được Windows nhận diện.
