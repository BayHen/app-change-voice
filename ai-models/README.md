# AI models

Thư mục này chỉ chứa manifest mẫu và tài liệu. Model weights không được commit vào Git.

VoiceMorph Studio chỉ nhập model ONNX có manifest schema v1, giấy phép nằm trong allow-list và
SHA-256 khớp. Ứng dụng không tự tải model, không nhận checkpoint PyTorch pickle từ nguồn tùy ý và
không xem một endpoint GPU là khả dụng nếu chưa probe thành công.

Model OpenVoice V2 được dùng cho prototype offline nằm ngoài binary Tauri. OpenVoice có giấy phép
MIT; người dùng vẫn phải có quyền đối với audio nguồn, audio tham chiếu và mọi checkpoint bổ sung.

`example.openvoice-v2.onnx.json` là mẫu metadata cho bản ONNX tương lai, không phải model hoạt động.
Phải thay `sha256` và `modelFile` bằng artifact đã export, kiểm thử và tin cậy trước khi import.
