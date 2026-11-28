# Hướng dẫn Agent Voices và kiểm thử giọng neural

**Phase 6:** Agent Voices hiện chỉ hiển thị Wind Assassin và Jade Guardian. Xem [hướng dẫn cài pack gốc](HUONG-DAN-RVC-GIONG-GOC.md). Các bước định tuyến Discord bên dưới vẫn áp dụng.

Bản source nằm trên `feature/new-version`. Chưa build lại EXE/bộ cài. Để dùng source mới, đóng phiên dev cũ và chạy:

```powershell
npm.cmd run tauri:dev
```

Chạy riêng Vite bằng trình duyệt không có backend Rust để chuyển giọng.

## Chuẩn bị model và runtime

1. Vào **AI Voice Studio**, cài runtime bằng các nút quản lý runtime hiện có.
2. Dùng **Get official Tsukuyomi-chan voice** sau khi đọc và chấp nhận điều khoản chính chủ, hoặc **Import ONNX model** để chọn model được phép dùng. Model ONNX tương thích cần có metadata/tensor đúng với engine RVC đang dùng.
3. Có thể chọn `.onnx` trực tiếp hoặc chọn `.json` rồi chọn ONNX khớp manifest. Kiểm tra nguồn model, quyền giọng, điều khoản và attribution trước khi đăng ký.
4. `RVC candidate` chỉ là đạt kiểm tra cấu trúc sơ bộ. Chỉ sau khi runtime load/inference thật mới biết model có chạy được hay không. Không dùng file checkpoint/PyTorch không rõ nguồn gốc.

## Gán giọng cho Agent

1. Mở **Voice Lab → Agent Voices**.
2. Chọn **Import Model** trên slot để mở Voice Pack Manager và chọn PTH hoặc ONNX. Nếu đã nhập model trong AI Voice Studio, chọn **Validate an installed model**.
3. Nhập nguồn, điều khoản, ghi công và xác nhận quyền dùng giọng. Với **Hugging Face URL**, chạy **Check repository metadata** trước. Hai ứng viên Jett/Sage hiện thiếu giấy phép nên không thể tải. Bạn có thể dùng model giọng gốc được phép.
4. Chọn **Install, validate & assign** và đợi **Ready**. Lần nhập PTH đầu tiên tự chuẩn bị bộ chuyển đổi CPU riêng. Nếu hủy, chạy lại cùng nguồn để dùng lại phần tải còn trong cache.
5. Chọn **Preview Voice** nếu muốn thử bằng WAV cục bộ. Dùng **RVC neural** cho đầu ra neural thuần hoặc **Hybrid** để thêm hiệu ứng sau neural.
6. Chọn **Activate Voice** hoặc **Load neural voice**. Dùng **Stop AI** trước khi đổi model hoặc cấu hình runtime.

19 slot không đồng nghĩa với 19 giọng đã cài. Slot thiếu weights hiện **Not Installed**. Cùng file/weights không thể được gán cho nhiều Agent rồi chỉ đổi tên hoặc pitch để coi là các giọng độc lập.

## Đưa giọng vào Discord

1. Trong Voice Lab, chọn **microphone vật lý** làm input, tai nghe vật lý làm output nghe thử. Không chọn CABLE Output làm microphone nguồn của VoiceMorph.
2. Bật **Monitor** ở thanh trạng thái dưới cùng. Trong phiên bản hiện tại, công tắc này khởi động cả audio engine/capture; load model AI riêng chưa tự bật microphone. Cần có tín hiệu trên meter microphone.
3. Trong Settings/phần mic ảo hoặc Discord integration, chọn endpoint phát **CABLE Input** rồi **Start routing**. Thanh trạng thái phải hiện **Virtual mic: live**.
4. Trong Discord, chọn thiết bị thu **CABLE Output**. Input/output của VB-Cable có tên ngược theo góc nhìn của ứng dụng: VoiceMorph phát vào Input, Discord thu từ Output.
5. Kiểm tra khi chưa trong cuộc gọi. Nếu không muốn nghe chính mình, dùng thiết lập nghe chính mình hiện có; không tắt audio engine khi vẫn cần chuyển giọng vào Discord.

Nếu AI ở Ready nhưng vẫn chưa có tiếng, kiểm tra lần lượt: Monitor/audio engine đã chạy, microphone vật lý có tín hiệu, routing đang active, Discord chọn đúng CABLE Output. Nếu AI faulted, đầu ra Agent mặc định mute để không vô tình phát giọng gốc. Dùng **Use Classic DSP** chỉ khi chủ động muốn chuyển sang DSP.

## So sánh và hiệu chỉnh

1. Chuẩn bị WAV được phép dùng, tốt nhất lấy bản gốc từ Recordings. Neural comparison hiện cần WAV 48 kHz; chỉ đọc tối đa 5 giây.
2. Dừng live AI, vào **A/B voice quality lab**, chọn WAV và xác nhận quyền xử lý cục bộ.
3. **Render four-way comparison** tạo Original / DSP only / Neural AI / Hybrid AI + DSP từ cùng input. Chỉ bấm nghe khi muốn phát âm thanh.
4. Dùng cùng WAV cho các model khác nhau. Nghe độ rõ chữ, âm sắc, hơi thở, sự tự nhiên và khác biệt danh tính; số đo tự động không thay cho đánh giá bằng tai.
5. **Calibrate input for this voice** phân tích F0 ước lượng, mức tín hiệu và clipping, lưu thống kê theo target model. Bản thu không được upload hoặc sao chép vào profile calibration. Khuyến nghị gain được áp dụng thủ công ở Voice Lab.

## Hiểu đúng kết quả đo

- RTF nhỏ hơn 1: xử lý file nhanh hơn thời gian thực trong lần đo; chưa chứng minh latency cuộc gọi thấp.
- Budget misses lớn hơn 0: có chunk xử lý quá ngân sách; có nguy cơ giật khi chạy live.
- Với model chính chủ đã thử trên RTX 3080, Low Latency chưa đạt thời gian thực; High Quality ổn định hơn trong mẫu ngắn nhưng có buffering/context lớn hơn.
- CPU Balanced đã chạy inference thật nhưng chậm hơn thời gian thực trong lần đo.
- Chưa có đánh giá nghe nam/nữ tiếng Việt/Anh đầy đủ hoặc nghiệm thu mic ảo end-to-end ở lượt này.

Xem số đo, nguồn và quyền model, file source thay đổi và giới hạn trong [báo cáo Voice Pack Manager](PHASE5-28-VOICE-PACKS-REPORT.md) và [báo cáo chất lượng Phase 5](PHASE5-NEURAL-REPORT.md).
