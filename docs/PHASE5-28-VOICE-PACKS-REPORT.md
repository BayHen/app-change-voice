# Phase 5.28 — Voice Pack Manager

Ngày kiểm tra 28/11/2026 · Nhánh `feature/new-version`

Voice Pack Manager nhận RVC PTH, ONNX, URL tệp công khai từ Hugging Face và model đã đăng ký. App kiểm tra quyền sử dụng, chuyển PTH sang ONNX bằng tiến trình riêng, chạy inference thật rồi mới tự gán model vào Agent. Trong lần kiểm thử cục bộ, model Tsukuyomi chính chủ được gán vào một slot thử nghiệm mang tên Jett-inspired. Danh tính hiển thị vẫn là Tsukuyomi. Bản cài của người dùng và các slot phát hành không chứa model thử nghiệm.

## Kết quả thực tế

| Hạng mục | Kết quả |
| --- | --- |
| Checkpoint | Tsukuyomi-chan official RVC, Normal 1, CV Rei Yumesaki |
| Nguồn và quyền | [Trang chính chủ](https://tyc.rei-yumesaki.net/work/software/rvc/) và [Điều khoản](https://tyc.rei-yumesaki.net/work/software/rvc/terms/). Phạm vi thử nghiệm theo điều khoản của chủ giọng. |
| Checkpoint SHA-256 | `cd4996435d0e9c9f93858a13d9ddf5442a011388478daab1f732e0ac2b2c4020` |
| Phát hiện | RVC v2, 40 kHz, 768 chiều embedding, F0 bật, FP16, bảng embedding có 109 hàng. Adapter hiện dùng speaker ID 0. |
| ONNX đầu ra | FP32, kiểm tra ONNX đầy đủ và kiểm tra hợp đồng tensor của adapter thành công. SHA-256 `035efead352999aa19ad27aa47a2496aaeb985a5a10ec7c77a29215b2d1ca72c`. |
| Inference | RTX 3080, CUDA, DIO, 48 kHz đầu vào, 8 khối × 8192 mẫu sau warm-up, peak 3037/32768. |
| Hiệu năng | 223,9 ms tổng thời gian inference do worker báo. RTF theo thời gian thực thi toàn đoạn 0,190 cho 1,365 giây tín hiệu tổng hợp. Lần chạy tích hợp gồm chuyển đổi, kiểm tra và đăng ký mất 47,76 giây. |
| Đăng ký và tự gán | Đăng ký thành công, gán slot kiểm thử, biên nhận kiểm tra đúng SHA model và digest runtime. Biên nhận bị từ chối khi thay digest runtime. |

Kết quả gốc nằm trong [result.json](../.ai-runtime-build/pack-test-1795844738082/result.json). Phép thử dùng tín hiệu tổng hợp và không mở thiết bị âm thanh, vì vậy độ trễ mic tới Discord, độ rõ phụ âm, hiện tượng metallic và sự khác biệt giữa hai danh tính chưa được xác minh. Báo cáo giai đoạn trước ghi kết quả trên mẫu âm thanh chính chủ của một danh tính và kiểm tra CPU fallback. [Báo cáo Phase 5](PHASE5-NEURAL-REPORT.md)

Luồng ONNX trực tiếp cũng đã được xác minh bằng bản export chính chủ có SHA-256 `d9caedd48ea81672946ce4896022da5e302996e1f5c1bf0ae2c141c2f7d3afe6`. CPU xử lý 8 khối tổng hợp, tổng inference 1.090,4 ms, peak 2866/32768 và RTF 0,826. App đã đăng ký và gán model trong một thư viện thử nghiệm riêng. Đây là cùng danh tính Tsukuyomi, được kiểm tra riêng để bao phủ phương thức nhập ONNX. [Biên nhận CPU](../.ai-runtime-build/onnx-pack-test-1795844971411/result.json)

## Jett và Sage

[Model card](https://huggingface.co/fox3000foxy/valorant_rvc) mô tả cả hai là RVC v2, 40 kHz, FP16, có F0. API Hugging Face xác nhận các tệp sau tồn tại tại revision `e60b8e7c2cbfba551f4632b9e855b67cb27e1969`. SHA dưới đây là metadata LFS do kho cung cấp. [Metadata của kho](https://huggingface.co/api/models/fox3000foxy/valorant_rvc/revision/e60b8e7c2cbfba551f4632b9e855b67cb27e1969?blobs=true)

| Tệp | Byte | SHA-256 |
| --- | ---: | --- |
| jett/jett.pth | 55.223.738 | `83745f80f560c727367f7e172a35605fbb5ca2f50749953e4a847c0111062bc7` |
| jett/jett.index | 72.629.619 | `d014f9dc0b5f4649ad84f5c9070a71b64b604738f234666d21cfa3587edfd8ed` |
| sage/sage.pth | 55.192.782 | `ef95e555298bac728d637174ebe7263fc1b12e7f6ed60b66b411ad96883045d2` |
| sage/sage.index | 71.794.939 | `35d408eea2a70e4f7e4f54bb4093f7c0fd9499246accdc7d6c076560e7d88db7` |

Kho thiếu giấy phép phân phối và không có bằng chứng cho phép dùng giọng. Cả hai ứng viên hiển thị **License Unverified**. Chỉ truy vấn metadata, chưa tải checkpoint hay index, chưa đối chiếu kiến trúc trên trọng số, chưa cài hoặc chạy inference cho hai ứng viên. Với model khác, app yêu cầu thông tin nguồn, điều khoản, ghi công và xác nhận quyền sử dụng từ người nhập. Giấy phép kho và quyền dùng giọng được kiểm tra riêng.

## Runtime và xử lý lỗi

- Runtime inference đã kiểm tra nằm tại `.ai-runtime-build/bootstrap-installed/releases/69c600bd252be1fd47f5bf9246294c976c32e27dfb4796b1712e92312d6504f7`. Phiên bản `w-okada-vm-cuda124-2026.11.26.quality1`, Python 3.10.11, Torch 2.6.0+cu124, ONNX Runtime GPU 1.21.0, NumPy 1.23.5. Nguồn w-okada ghim commit `7e2ce5c4800010bbc7604968bd29b802534fac05`.
- Converter CPU riêng nằm dưới `.ai-runtime-build/pack-manager-e2e/packs/converter-torch210-cpu` trong lần kiểm thử. Trong app, cache nằm dưới thư mục dữ liệu `voice-packs`. Wheel PyTorch 2.10.0+cpu và SymPy 1.14 được ghim SHA-256, lưu lại để dùng chung. Tiến trình giới hạn tài nguyên, chỉ nhận ZIP checkpoint hiện đại, dùng `weights_only=True` và kiểm tra cấu hình, kích thước, kiểu tensor. [Thông báo bảo mật PyTorch](https://github.com/pytorch/pytorch/security/advisories/GHSA-63cw-57p8-fm3p)
- Tải model từ URL được ghim theo revision và SHA LFS, có tiến độ byte, hủy, tải tiếp và kiểm tra dung lượng. Hủy giữ cache. Nếu đăng ký đã hoàn tất trước khi hủy, model có thể còn trong thư viện và có thể chạy **Validate & assign**. Đường dẫn tệp gốc giữ nguyên khi xóa bản quản lý.
- Converter dùng chung HuBERT và mã synthesizer với runtime đã xác minh. Runtime thử nghiệm có `faiss-cpu 1.7.4` nhưng adapter chưa có tích hợp index được kiểm chứng. App từ chối index thay vì áp dụng ngầm. RMVPE chưa có tài sản đủ điều kiện. DirectML chưa được kiểm chứng. Các điều khiển Index Rate và Voice Protection chưa có backend tương ứng.
- Giao diện hiển thị trạng thái cài, chuyển, xác minh, lỗi, thiếu runtime và Ready. Reset khôi phục cấu hình dùng khi xác minh. Cấu hình mặc định của pack sau xác minh dùng High Quality, pitch 0 và tắt DSP sau neural. Đây là điểm bắt đầu để nghe và hiệu chỉnh từng giọng.

## Vì sao trước đây chưa đạt chất lượng

Slot Agent trước đây chỉ có thể gán model thủ công, thiếu quy trình chuyển PTH và suy luận kiểm chứng. Export ONNX thiếu metadata RVC hoặc sai hợp đồng tensor không thể nạp vào adapter hiện hành. Classic DSP chỉnh pitch và formant nên không tạo được danh tính neural độc lập. Warm-up có tín hiệu hữu thanh trong runtime hiện tại giúp xử lý phần khởi tạo khi bắt đầu nói. Chất lượng của từng giọng vẫn phụ thuộc model, F0 và bản ghi kiểm thử.

## Mã nguồn và kiểm thử

- `src-tauri/src/ai/packs.rs`, `converter_runtime.rs`, `bootstrap.rs`, `process.rs`, `mod.rs` và `src-tauri/src/lib.rs` xử lý công việc, cache, xác minh và IPC.
- `ai-sidecar/convert_worker.py` đọc checkpoint hạn chế và xuất ONNX.
- `src/components/VoicePackManager.tsx`, `AgentVoiceCollection.tsx`, `src/styles/studio.css`, `src/types/ai.ts` và `src/services/tauri.ts` nối thao tác giao diện với backend.
- `tests/test_convert_worker.py`, `tests/agent-voices.test.tsx` và các bài kiểm thử trong `packs.rs` bao phủ định dạng, điều kiện cấp quyền, đường dẫn Unicode, trạng thái kích hoạt và lần chạy GPU với model chính chủ.
- 124 test Rust, 36 test giao diện và 5 test Python đạt. Hai test tích hợp model chính chủ chạy riêng và đạt. TypeScript và ESLint đạt.
- Playwright chạy trên React thật với IPC giả lập, kiểm tra 19 slot trống, trạng thái chặn nguồn không giấy phép và bố cục desktop/900×650. [Hộp thoại nhập file](screenshots/phase5-28/local-import.png) · [Trạng thái thiếu giấy phép](screenshots/phase5-28/compact-dialog.png)

## Cài và kiểm tra trên Windows

1. Mở **AI Voice Studio**, cài hoặc sửa runtime quản lý nếu app báo **Runtime required**. Quay lại **Voice Lab → Agent Voices**.
2. Chọn **Import Model** trong slot, chọn checkpoint RVC PTH hoặc ONNX được phép. Nhập nguồn, điều khoản, ghi công và xác nhận phạm vi quyền. Với URL Hugging Face, dùng **Check repository metadata** trước.
3. Chọn **Install, validate & assign**. Chờ **Ready**, dùng **Preview Voice** để mở phòng thử A/B, chọn WAV có quyền và xác nhận quyền xử lý.
4. Chọn **Activate Voice**. Trong VoiceMorph, chọn mic vật lý làm input và **CABLE Input (VB-Audio Virtual Cable)** làm đích virtual mic. Chọn tai nghe vật lý cho monitor. Trong **Discord → Voice & Video**, đặt **CABLE Output (VB-Audio Virtual Cable)** làm Input Device, tai nghe làm Output Device. Dùng mic test của Discord ngoài cuộc gọi để nghe thử và kiểm tra meter.
5. Nếu không có tín hiệu, xem [hướng dẫn sử dụng](HUONG-DAN-AGENT-VOICE.md). Kiểm tra meter capture và virtual mic trong app trước khi kiểm tra Discord.

Nghiệm thu tiếng Việt/Anh, hai danh tính độc lập và loopback mic ảo còn chờ model/mẫu có quyền và phiên kiểm thử thiết bị. EXE chưa được đóng gói lại theo yêu cầu chạy test trước.
