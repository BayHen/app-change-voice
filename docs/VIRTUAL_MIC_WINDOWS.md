# Virtual Microphone trên Windows

## Phương án được chọn

Task 11 dùng phương án A: VoiceMorph Studio phát tín hiệu đã xử lý vào **playback endpoint** của một
virtual audio cable đã được người dùng cài riêng. Ứng dụng gọi như Discord/Zoom/Teams/OBS chọn
**capture endpoint ghép cặp** làm microphone.

Phiên bản đầu tiên được thiết kế và nhận diện tên phổ biến của VB-CABLE, nhưng không khóa vào một nhà
cung cấp: mọi WASAPI output endpoint đều có thể được chọn sau khi người dùng xác minh nó có capture
endpoint ghép cặp. VoiceMorph không tải, cài, cập nhật hoặc gỡ kernel driver.

VB-CABLE là phần mềm bên thứ ba theo mô hình donationware/licensing của VB-Audio. Hãy đọc điều khoản
hiện hành trên trang chính thức trước khi dùng trong môi trường thương mại.

## 1. Chuẩn bị virtual audio device

1. Tải driver trực tiếp từ trang chính thức của nhà cung cấp, ví dụ VB-CABLE tại
   <https://vb-audio.com/Cable/index.htm>.
2. Giải nén, chạy installer phù hợp Windows x64 bằng quyền Administrator và reboot theo hướng dẫn của
   nhà cung cấp.
3. Mở **Settings > System > Sound** và xác nhận có cả hai endpoint. Với VB-CABLE, playback thường tên
   `CABLE Input`, capture thường tên `CABLE Output`.

Không dùng SYSVAD sample như driver production. Tự phát triển driver cần WDK, ký driver, kiểm thử kernel,
installer đặc quyền và quy trình phát hành riêng; nằm ngoài Task 11.

## 2. Chọn routing output

1. Mở trang **Virtual Microphone**.
2. Chọn playback endpoint của cable, ví dụ `CABLE Input (VB-Audio Virtual Cable)`.
3. Nếu đổi endpoint khi audio đang chạy, VoiceMorph chủ động dừng session để giải phóng stream cũ.

Không chọn speaker/headphone vật lý chỉ vì nó xuất hiện trong danh sách. Một output stream thông thường
không tự trở thành microphone ảo.

## 3. Khởi động audio processing

1. Trên Dashboard, chọn microphone vật lý làm input.
2. Chọn headphone làm monitoring output và nhấn **Start monitoring**. Có thể tắt Hear Myself nếu không
   muốn nghe trực tiếp; routing vẫn dùng cùng processed pipeline.
3. Quay lại **Virtual Microphone**, nhấn **Start routing**.
4. Trạng thái `Routing active` chỉ xác nhận playback endpoint đã mở và callback đang chạy. Nó chưa xác
   nhận ứng dụng đích đã mở capture endpoint.

Luồng xử lý:

```text
Microphone -> Noise/Gate -> Pitch/Formant/Effects -> Input Gain
                                                   |-> Recording processed
                                                   |-> Monitoring queue -> headphone
                                                   `-> Routing queue -> limiter -> virtual playback endpoint
                                                                         |
                                                                         `-> paired capture endpoint -> Discord/OBS
```

Routing dùng queue SPSC bounded 80 ms. Callback input chỉ `try_push`; khi đầy sẽ bỏ sample mới và tăng
overflow counter. Callback output phát silence khi thiếu dữ liệu, tăng underrun counter, không lock, không
I/O, không log và không allocation theo frame. Nếu sample rate khác, sink chuyển tần số streaming; device
sample rate chính không bị đổi.

## 4. Chọn microphone trong ứng dụng đích

- Discord/Zoom/Teams: chọn capture endpoint của cable, ví dụ `CABLE Output`.
- OBS Studio: thêm **Audio Input Capture**, sau đó chọn `CABLE Output`.
- Không chọn microphone vật lý trong ứng dụng đích nếu muốn nhận processed voice.

Nói vào microphone và kiểm tra meter ở VoiceMorph lẫn ứng dụng đích. Thay preset trong khi routing để xác
nhận ứng dụng đích nhận đúng tín hiệu processed.

## 5. Test đầu ra

1. Dùng headphone để tránh feedback.
2. Kiểm tra meter virtual output có chuyển động.
3. Dùng Discord Mic Test hoặc OBS meter/recording.
4. So sánh với bản recording processed trong VoiceMorph.
5. Ghi lại driver/version, sample rate và underrun/overflow counter khi báo lỗi.

## 6. Khắc phục lỗi

- **Không thấy cable:** reboot sau khi cài driver, kiểm tra Device Manager và Windows Sound Settings, rồi
  Refresh trong VoiceMorph.
- **Active nhưng app đích im lặng:** kiểm tra app đích đang chọn capture endpoint (`CABLE Output`), không
  phải playback endpoint (`CABLE Input`). Kiểm tra quyền microphone của Windows.
- **Routing Faulted:** endpoint đã bị rút/tắt hoặc driver báo lỗi. Stop routing, refresh, chọn lại device và
  restart audio processing.
- **Nhiều underrun/overflow:** đóng ứng dụng giữ exclusive mode, đồng bộ format mặc định của hai endpoint,
  thử 48 kHz và tránh tải CPU cao.
- **Feedback:** không dùng capture endpoint của cùng cable làm input VoiceMorph; dùng microphone vật lý và
  headphone.
- **Âm gốc bị gửi:** app đích phải chọn capture endpoint của cable. VoiceMorph chỉ đưa nhánh processed vào
  routing queue; nhánh original chỉ dùng cho recording A/B.

## Kiểm thử thủ công bắt buộc

Các mục Discord/OBS, nghe chất lượng, disconnect driver và latency end-to-end cần máy có virtual cable thật.
Không được đánh dấu PASS chỉ từ unit test hoặc từ việc stream playback mở thành công.

### Trạng thái máy kiểm thử 23/10/2026

Windows đã nhận `CABLE Input` (playback) và `CABLE Output` (capture). Kiểm thử tự động đã mở virtual output,
kiểm tra nguồn 44.1 kHz và chạy pipeline microphone → DSP → virtual output thành công. Discord/OBS vẫn cần
người dùng chọn `CABLE Output (VB-Audio Virtual Cable)` và xác nhận meter/recording bằng thao tác thủ công.
