# Khắc phục System-Wide Audio

- Không thấy virtual mic: kiểm tra `CABLE Input` và `CABLE Output` trong Windows Sound, reboot sau cài driver.
- Routing active nhưng app im lặng: app phải chọn capture endpoint `CABLE Output`, không phải `CABLE Input`.
- Feedback: VoiceMorph input phải là mic vật lý; mọi app output phải là headphone.
- Fault/disconnect: Stop Routing, Refresh, chọn lại endpoint, sau đó restart audio processing.
- Underrun/overflow: dùng 48 kHz nếu được hỗ trợ, đóng app giữ exclusive mode và giảm tải CPU.
- Hai lần tiếng: không capture cùng endpoint qua nhiều OBS sources hoặc bật monitoring trùng.
- Sleep/resume: refresh device list và dựng lại streams; không giả định WASAPI ID còn hợp lệ.

Không xóa/cài driver tự động trong VoiceMorph. Instance VB-Audio báo lỗi trong Device Manager phải được sửa ở
driver/Windows trước khi chọn.
