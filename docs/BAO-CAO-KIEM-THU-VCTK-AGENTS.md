# Kiểm thử p231 và p238 cho VoiceMorph Studio

Ngày 28/11/2026 · Nhánh `feature/new-version`

Hai model pretrained đã được dùng trong phạm vi thử nghiệm cục bộ. Wind Assassin dùng p231 và Jade Guardian dùng p238. Đây là hai danh tính mẫu từ VCTK, dùng để thử profile phần mềm.

## Nguồn và tính toàn vẹn

Kho Nekochu khai báo Apache-2.0 và ghi VCTK là dữ liệu nguồn. University of Edinburgh công bố VCTK 0.92, corpus cho English Multi-speaker Voice Cloning Toolkit, theo CC BY 4.0. Thử nghiệm giữ mã loa ẩn danh và lưu trọng số, index và WAV kết quả tại máy cục bộ. Giấy phép riêng của từng người nói chưa được đối chiếu. Model card cũng không công bố checksum riêng của tập ghi âm huấn luyện hay trọng số HuBERT huấn luyện. Manifest ghi rõ revision và nguồn phát hành, checksum encoder của runtime suy luận và SHA của index nguồn, thay vì điền thông tin huấn luyện chưa biết. [Kho model](https://huggingface.co/Nekochu/RVC-VCTK_Voice-sample/tree/005c2f948ee9dafd7e3aa7f261b4c3a24beebeef) · [VCTK 0.92](https://doi.org/10.7488/ds/2645)

| Profile | Checkpoint SHA-256 | Index IVF-Flat gốc SHA-256 |
| --- | --- | --- |
| Wind Assassin / p231 | `12f819e129c6e48779439e491b85593b8819d094f751c3515ea13be6bd8b5a59` | `14c3a6fb421ed0c77654bb9ed5778307ff0204d7b047b6bbe37922af42f601ab` |
| Jade Guardian / p238 | `a32d7a2311a6ce484a497197d3f749ed3ec08d7e748ba5e88a6c8dc191f3cffc` | `8be675ea57ea3059c8e0edf82034024259f375baadde7fb1c52f8451672f7592` |

PyTorch 2.10.0+cpu dùng `weights_only=True` để đọc ZIP checkpoint trong worker riêng. Cả hai là RVC v2 F0, 40 kHz, 768 chiều, trọng số FP16 và bảng speaker embedding có 109 hàng. Exporter chọn SID 0, giữ đúng hàng của speaker này và xuất đồ thị FP32 một speaker. Manifest có checkpoint gốc cùng SHA. Không có bước huấn luyện. Các tensor được kiểm tra trước khi nạp và chạy trong ONNX Runtime.

Index đầu vào là IVF-Flat, có 47.446 vector cho p231 và 63.065 cho p238. Mỗi index thử lấy 16.000 vector gốc trải đều theo thứ tự ID, sau đó dựng IndexFlatL2 riêng. Retrieval chạy trên tập vector giữ lại. Vì tập và thuật toán tìm kiếm thay đổi, kết quả có thể khác khi sử dụng toàn bộ index IVF của tác giả. [Biên nhận p231](../.ai-runtime-build/phase6-vctk-test/p231/index-conversion.json) · [Biên nhận p238](../.ai-runtime-build/phase6-vctk-test/p238/index-conversion.json)

RMVPE ONNX lấy từ kho GPL-3.0 do w-okada dùng, SHA `84f0586308e36157f75b77c8591bf636d6719c0c4ba95f8faf3df479e7566219`. Runtime thử riêng có manifest digest `07d414cf85ce66907eb1926e8ffa122d38e497040fa0b9f7a37e98cd3bebccec`. Python 3.10.11, Torch 2.6.0+cu124, ONNX Runtime GPU 1.21.0 và FAISS 1.7.4. Tài sản GPL chỉ dùng ở máy cục bộ và không đưa vào bản phân phối. [RMVPE ONNX](https://huggingface.co/wok000/weights_gpl/blob/d1b578cfcdf944dd3910786f7604754c79234b36/rmvpe/rmvpe_20231006.onnx)

## WAV và cách đo

[WAV gốc giống nhau](../.ai-runtime-build/phase6-vctk-test/original_input.wav) · [Wind Assassin](../.ai-runtime-build/phase6-vctk-test/wind_assassin_output.wav) · [Jade Guardian](../.ai-runtime-build/phase6-vctk-test/jade_guardian_output.wav) · [WAV giọng trầm](../.ai-runtime-build/phase6-vctk-test/lower_input.wav)

Nguồn là hai trích đoạn LibriSpeech CC BY 4.0 do Heather Barnett và Garth Comira đọc, phân phối qua librosa, được resample sang PCM16 mono 48 kHz. Hai WAV chính dùng cùng đầu vào Heather Barnett, CUDA, pitch 0, index rate 0,5, Protect 0,33 và Balanced (4096 frame, crossfade 1024, context 8192). Đầu ra gần 13,91 giây. Kiểm thử neural thuần, không áp formant/DSP. [Nguồn audio](https://librosa.org/doc/dev/recordings.html) · [Giấy phép LibriSpeech](https://www.openslr.org/12/)

p50 inference đo trong sidecar. p95 IPC đo cả gửi và nhận block. RTF gồm khởi động block và thời gian chạy các block, tách khỏi thời gian load model. Ngân sách block Balanced là 85,33 ms. CPU % dùng thang 100% mỗi lõi. RSS là đỉnh quan sát của tiến trình trong lượt. GPU % và bộ nhớ toàn GPU là số đo thiết bị có thể gộp tiến trình khác. VRAM PyTorch thiếu phần cấp phát của ONNX Runtime. Không diễn giải số đo file là độ trễ mic đến Discord.

## Lỗi đã tìm và sửa

Khi bật cả RMVPE và Protect 0,33, pipeline w-okada tái sử dụng lịch sử F0 ngắn hơn số frame đặc trưng. Wind Assassin ở lần CUDA đầu có 81 block thành công trong 163 block. Log ghi mismatch 28/22 khi trộn Protect. Bản sửa căn lịch sử F0 với frame âm thanh, bảo vệ frame vô thanh bằng đặc trưng HuBERT trước retrieval và đưa cảnh báo upstream vào log của sidecar. Hai lượt CUDA chính sau sửa đều có 162 block hoàn tất với một block khởi động. Dữ liệu trước sửa nằm riêng trong thư mục `runs`. Số liệu sau sửa nằm trong `runs-fixed`.

## Chất lượng nghe

Đo hỗ trợ bằng Whisper tiny.en chạy offline. Bản chép của WAV gốc là tham chiếu tự động, chưa phải ground truth do người chép. Ở hai WAV CUDA chính, sau khi nâng mức âm cho phép đo ASR, cả hai vẫn sai khác 46 chỉnh sửa từ trên 47 từ tham chiếu (khoảng 97,9%). Các WAV chính giữ nguyên biên độ do runtime ghi. Không thấy mẫu chạm rail PCM16 ở cả hai. RMS khoảng −35,9 dBFS (Wind) và −36,1 dBFS (Jade). Ước lượng F0 trung vị lần lượt khoảng 241 Hz và 211 Hz trên cùng đầu vào. Khác biệt cao độ là quan sát âm học và chưa chứng minh người nghe phân biệt được danh tính. Chưa có điểm nghe mù hoặc MOS. Kết quả nhận dạng hiện không đủ để đề xuất đổi chỗ p231 và p238.

[Đo ASR và tín hiệu trên hai WAV chính](../.ai-runtime-build/phase6-vctk-test/asr-quality-level-matched.json) · [Whisper](https://github.com/openai/whisper/tree/v20250625)

## Kết quả hiệu năng

| Profile | Provider | Input / pitch | Index | p50 inference (ms) | p95 IPC (ms) | RTF | Block tốt |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| Jade | CPU | Giọng nữ / +0 | 0.5 | 333.4 | 434.8 | 4.54 | 57/58 |
| Jade | CUDA | Giọng nữ / +0 | 0.3 | 81.9 | 90.2 | 1.78 | 45/46 |
| Jade | CUDA | Giọng nữ / +0 | 0.5 | 94.2 | 150.9 | 1.51 | 162/163 |
| Jade | CUDA | Giọng nữ / +0 | 0.65 | 83.4 | 93.9 | 1.80 | 45/46 |
| Jade | CUDA | Giọng nữ / +0 | 0.8 | 81.9 | 95.2 | 1.77 | 45/46 |
| Jade | CUDA | Giọng trầm / +12 | 0.5 | 83.2 | 99.5 | 1.79 | 45/46 |
| Jade | CUDA | Giọng trầm / +6 | 0.5 | 81.7 | 90.0 | 1.77 | 45/46 |
| Jade | CUDA | Giọng trầm / +9 | 0.5 | 44.9 | 53.4 | 1.29 | 45/46 |
| Wind | CPU | Giọng nữ / +0 | 0.5 | 335.0 | 469.6 | 4.41 | 57/58 |
| Wind | CUDA | Giọng nữ / +0 | 0.3 | 86.3 | 117.4 | 1.86 | 45/46 |
| Wind | CUDA | Giọng nữ / +0 | 0.5 | 81.8 | 93.9 | 1.22 | 162/163 |
| Wind | CUDA | Giọng nữ / +0 | 0.65 | 84.0 | 100.7 | 1.64 | 45/46 |
| Wind | CUDA | Giọng nữ / +0 | 0.8 | 97.3 | 173.5 | 2.15 | 45/46 |
| Wind | CUDA | Giọng trầm / +12 | 0.5 | 82.0 | 96.6 | 1.77 | 45/46 |
| Wind | CUDA | Giọng trầm / +6 | 0.5 | 83.5 | 92.8 | 1.80 | 45/46 |
| Wind | CUDA | Giọng trầm / +9 | 0.5 | 82.8 | 88.5 | 1.77 | 45/46 |

### Tài nguyên ở bốn lượt chính

| Profile | Provider | CPU (mỗi lõi = 100%) | RSS đỉnh (MiB) | VRAM Torch đỉnh (MiB) | GPU toàn máy đỉnh (%) | VRAM toàn máy đỉnh (MiB) | Load RPC (s) |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Jade | CPU | 294.6% | 2907 | 0 | 3 | 1185 | 12.27 |
| Jade | CUDA | 162.9% | 2665 | 364 | 21 | 2503 | 12.72 |
| Wind | CPU | 304.1% | 2949 | 0 | 2 | 1056 | 9.97 |
| Wind | CUDA | 262.6% | 2974 | 364 | 25 | 2384 | 9.96 |

Các lượt CUDA chính dài gần 13,91 giây. Hai lượt CPU dùng cùng đoạn đầu gần 4,95 giây. Các biến thể CUDA dùng cùng đoạn đầu gần 3,93 giây của từng WAV, nên tỷ trọng chi phí lượt đầu lớn hơn.

JSON từng lượt giữ dưới thư mục cục bộ `runs-fixed/`.

## Độ rõ qua toàn bộ 16 lượt

ASR sau chuẩn hoá mức âm có sai khác tương đối từ 97,9% đến 172,7%. Chỉ số có thể vượt 100% khi ASR chèn nhiều từ. Các đoạn giọng trầm ở +6/+9/+12 không cho thấy sự phục hồi ổn định về nội dung. Không có lượt nào chạm rail PCM16. [Dữ liệu ASR cho từng cấu hình](../.ai-runtime-build/phase6-vctk-test/asr-quality-level-matched.json)

Đầu ra và code kiểm thử dùng các tham số neural. Không có DSP thay thế, không retrain và không thêm profile mới. Kết quả cho thấy hai checkpoint chạy kỹ thuật được, nhưng chưa đủ độ rõ hay tốc độ để đề xuất bật live hoặc đổi thứ tự gán profile.

## Tái chạy cục bộ

Từ gốc repository, dùng `.ai-runtime-build/python310/python.exe .ai-runtime-build/phase6-vctk-test/run-fixed.py` để tiếp tục các lượt nào còn thiếu JSON. Lệnh đo thực tế được ghi trong script và dùng pack, WAV, runtime đã cố định SHA ở thư mục đó. Bộ kiểm thử backend dùng:

```powershell
cargo test --offline --manifest-path src-tauri/Cargo.toml authorized_vctk_pair_pack_preview_and_realtime -- --ignored --nocapture
```

Kiểm tra thiết bị Windows bằng `virtual_cable_accepts_sample_rate_mismatch` đã qua. Bài này chỉ mở cấu hình stream ở 44,1 kHz và không phát tín hiệu, nên chưa xác minh thu lại WAV ở CABLE Output.

## Nhập pack và thử backend ứng dụng

Pack Manager đã nhập đúng hai pack vào thư viện thử riêng `verified-library`. Receipt có model SHA, runtime digest, profile, provenance, RMVPE và FAISS. Backend của Voice Preview tạo WAV từ đoạn đầu tối đa 5 giây. Backend Load Neural Voice mở model CUDA và chuyển PCM từ WAV giống nhau qua bộ đệm thời gian thực. Hai engine kết thúc bằng lỗi `Real-time budget: AI cannot keep up with audio input` và mute theo chính sách đã cấu hình.

| Profile | Preview RTF | Block preview quá ngân sách | Mẫu PCM nghe được ở realtime ring | Block realtime hoàn tất | Kết quả live |
| --- | ---: | ---: | ---: | ---: | --- |
| Wind | 0,981 | 6 | 82.300 | 21 | Faulted, vượt ngân sách |
| Jade | 0,612 | 0 | 57.077 | 18 | Faulted, vượt ngân sách |

Thời hạn của block đầu trong worker được tăng để khởi tạo resampy/Numba và các kernel trong warm-up. Hạn cho các block thông thường không đổi. Lần đóng gói đầu trước sửa hỏng ở khởi động nguội. Lần sau cả hai pack được đăng ký và xuất preview thành công. Đo ở realtime ring là dữ liệu trong backend, tách khỏi phép thử thiết bị. Hồ sơ ứng dụng đang dùng không bị thay đổi trong quá trình kiểm thử.

[Biên nhận hai pack và trạng thái engine](../.ai-runtime-build/phase6-vctk-test/verified-library/app-verification.json) · [Preview Wind](../.ai-runtime-build/phase6-vctk-test/verified-library/wind-assassin-app-preview.wav) · [Preview Jade](../.ai-runtime-build/phase6-vctk-test/verified-library/jade-guardian-app-preview.wav)

## Loopback VB-Cable trên Windows

Đã thành công qua đường phát CPAL của VoiceMorph. Trước khi phát, Core Audio báo 0 phiên capture đang hoạt động. Test phát đoạn 3 giây từ mỗi WAV neural vào **CABLE Input**, thu lại từ **CABLE Output** ở PCM16 mono 48 kHz, rồi đóng hai stream. Cả hai thu được 174.720 mẫu. Peak đạt 0,09616 (Wind) và 0,08026 (Jade), số overrun của queue phát là 0. Sau khi căn trễ riêng theo cửa sổ 0,5 giây, hệ số tương quan lần lượt đạt 0,898–0,993 và 0,932–0,992. Căn một độ trễ duy nhất cho cả clip cho tương quan thấp hơn, do đó phép thử chỉ xác nhận tín hiệu đến thiết bị, chưa đo độ trễ đồng bộ hay độ liên tục mic live.

Các lần dùng PortAudio và lần mở capture trong sandbox thất bại trước phát. Lần chạy CPAL được cấp quyền truy cập thiết bị đã qua. Kiểm thử không đổi thiết bị mặc định và không gửi âm thanh qua Discord. Luồng app realtime bằng hai pack vẫn lỗi ngân sách, dù đường phát lại WAV đã kiểm chứng qua cable.

[Capture Wind](../.ai-runtime-build/phase6-vctk-test/wind_assassin_cable_capture.wav) · [Capture Jade](../.ai-runtime-build/phase6-vctk-test/jade_guardian_cable_capture.wav) · [Đối chiếu tín hiệu](../.ai-runtime-build/phase6-vctk-test/cable-loopback.json)

Kiểm thử hồi quy đạt 126 ca Rust, 35 ca Vitest, 4 ca bridge Python và 5 ca converter Python. TypeScript, ESLint và kiểm tra diff đạt. Kiểm thử phần cứng và kiểm thử nhập/preview/nạp hai pack chạy riêng. EXE phát hành chưa được build lại.
