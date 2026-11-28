# Wind Assassin và Jade Guardian trên Windows

**Voice Lab → Agent Voices** chỉ hiển thị hai nhân vật gốc. Wind Assassin có định hướng sáng, nhanh và tự tin. Jade Guardian có định hướng ấm, trưởng thành và điềm tĩnh. Đây là mô tả casting. Chất giọng thực tế phụ thuộc người đọc, dữ liệu và kết quả đánh giá.

Hai profile hiện ở **Missing Model**. Chưa có hai bộ WAV độc lập với quyền huấn luyện đã xác minh. Ảnh nhân vật là tạo hình mới cho VoiceMorph. Thư viện cũ được ẩn khỏi màn hình và dữ liệu lưu cục bộ vẫn tương thích.

## 1. Chuẩn bị bộ dữ liệu

- Chọn hai người nói trưởng thành khác nhau. Lưu xác nhận riêng về huấn luyện, đổi giọng và các mục đích chia sẻ nếu có. Dùng mã người nói giả danh để tránh ghi thông tin riêng tư vào manifest.
- Mỗi bộ dùng WAV PCM16 mono, 16–48 kHz theo danh sách validator hỗ trợ, mỗi đoạn 0,5–60 giây. Ưu tiên lời nói sạch và đa dạng phụ âm, ngữ điệu, tốc độ và cảm xúc. Giữ riêng bản thu đánh giá. Để thử cả tiếng Việt và tiếng Anh, cần nội dung và người nghe đủ năng lực ở cả hai ngôn ngữ.
- Không dùng bản thoại, clip trích từ trò chơi hoặc checkpoint Jett/Sage cộng đồng thiếu quyền. Đặt dữ liệu, trọng số và bản ghi kết quả trong thư mục riêng hoặc `.rvc-training/`, đã được Git bỏ qua.
- Mỗi bộ có `dataset.json` nằm cạnh thư mục WAV. Thay các giá trị mẫu bằng dữ liệu và URL có thật. `licenseUrl` có thể dẫn đến tài liệu có kiểm soát truy cập. App kiểm tra khai báo và tính toàn vẹn. Người nhập chịu trách nhiệm xác nhận tài liệu áp dụng cho bộ giọng.

```json
{
  "schemaVersion": 1,
  "profileId": "wind-assassin",
  "speakerId": "pseudonymous-speaker-a",
  "sourceUrl": "https://example.org/private-dataset",
  "licenseUrl": "https://example.org/private-voice-permission",
  "attribution": "Voice credit approved by the speaker",
  "consent": {"training": true, "voiceConversion": true},
  "clips": [
    {"file": "audio/train-01.wav", "sha256": "<64 lowercase hex>", "split": "train", "language": "vi", "transcript": "Nội dung người nói đã đọc."},
    {"file": "audio/held-out-01.wav", "sha256": "<64 lowercase hex>", "split": "holdout", "language": "vi", "transcript": "Câu khác dùng khi đánh giá."}
  ]
}
```

Dùng SHA-256 của chính tệp WAV. Kiểm tra cả cặp trước khi train.

```powershell
(Get-FileHash -Algorithm SHA256 -LiteralPath '.rvc-training/wind/audio/train-01.wav').Hash.ToLowerInvariant()
py -3.12 .\scripts\rvc-original.py validate-pair .rvc-training/wind/dataset.json .rvc-training/jade/dataset.json
```

Validator kiểm tra quyền khai báo, định dạng, thời lượng, tín hiệu, clipping, checksum, tệp trùng giữa các tập và đường dẫn vượt thư mục. Nó không xác minh danh tính người nói, lỗi transcript hay hiệu ứng trong bản thu. Rà soát các mục đó bằng tai trước khi huấn luyện.

## 2. Môi trường huấn luyện

1. Cài runtime qua **AI Voice Studio**. Runtime suy luận gồm HuBERT và FAISS đã xác minh. `$runtime` phải trỏ đến thư mục release chứa `runtime-manifest.json`, không phải thư mục cha chứa `current.json`.
2. Chuẩn bị Git, Python 3.12 x64, GPU NVIDIA/CUDA và FFmpeg cùng FFprobe trong PATH. Dùng môi trường riêng bên dưới. Các lệnh tải mã và thư viện, không tự tải checkpoint hoặc bản ghi của bên thứ ba. Cặp Torch 2.7.1/CUDA 11.8 dưới đây là lựa chọn của bản trainer ghim cho RTX trước dòng 50. Đối với GPU cần CUDA khác, dùng cặp Torch/Torchaudio tương ứng theo hướng dẫn upstream. [Nguồn RVC được ghim](https://github.com/RVC-Project/Retrieval-based-Voice-Conversion-WebUI/tree/81eed5e8f68b6bed1789f682fe78cdd324495afc)

```powershell
git clone https://github.com/RVC-Project/Retrieval-based-Voice-Conversion-WebUI.git .references/rvc-training
git -C .references/rvc-training checkout --detach 81eed5e8f68b6bed1789f682fe78cdd324495afc
py -3.12 -m venv .rvc-training/venv
$trainingPython = (Resolve-Path .rvc-training/venv/Scripts/python.exe).Path
& $trainingPython -m pip install torch==2.7.1+cu118 torchaudio==2.7.1+cu118 --index-url https://download.pytorch.org/whl/cu118
& $trainingPython -m pip install -r scripts/rvc-training-requirements.txt
& $trainingPython -m pip freeze | Set-Content .rvc-training/dependencies.txt
$runtimeHome = Join-Path $env:APPDATA 'com.voicemorph.studio/ai-runtime'
$release = Get-Content (Join-Path $runtimeHome 'current.json') -Raw | ConvertFrom-Json
$runtime = Join-Path $runtimeHome "releases/$release"
```

Nếu checkout đã có, dùng đường dẫn đó và kiểm tra bằng `git -C .references/rvc-training rev-parse HEAD`. `--init` tùy chọn cho generator và discriminator 40 kHz đã rà soát, theo thứ tự đó. Mỗi lần dùng cung cấp đường dẫn và SHA-256. Khi không cung cấp, trainer khởi tạo từ đầu. Chỉ dùng tệp khởi tạo của nguồn đã xác minh. Các tệp đó được xử lý bởi môi trường huấn luyện riêng.

Để dùng RMVPE khi train, đặt bản RMVPE PT có nguồn và quyền sử dụng đã xác minh tại `.references/rvc-training/assets/rmvpe/rmvpe.pt`, rồi thêm `--rmvpe <sha256>`. Nếu không cung cấp, script dùng Praat/PM và ghi rõ phương pháp. Runtime suy luận quản lý RMVPE ONNX riêng. Khi release đã xác minh chứa asset ONNX tương thích, pack thử RMVPE lúc cài. Runtime hiện tại dùng F0 DIO hoặc Harvest khả dụng vì release thử nghiệm thiếu RMVPE ONNX.

## 3. Huấn luyện và xuất pack

```powershell
& $trainingPython scripts/rvc-original.py train --dataset .rvc-training/wind/dataset.json --trainer .references/rvc-training --python $trainingPython --runtime $runtime --output .rvc-training/wind-run --epochs 200 --batch 4
& $trainingPython scripts/rvc-original.py train --dataset .rvc-training/jade/dataset.json --trainer .references/rvc-training --python $trainingPython --runtime $runtime --output .rvc-training/jade-run --epochs 200 --batch 4
```

Script dùng slicer RVC 40 kHz, HuBERT layer 12 cùng checksum với inference, huấn luyện RVC v2 F0 với speaker ID 0 và tạo index FAISS IndexFlatL2 768 chiều riêng cho mỗi giọng. Log mô hình nằm dưới `.references/rvc-training/logs/<experiment-id>`. Mỗi lần chạy cần thư mục output và experiment mới. Nếu thử lại sau lỗi, giữ lại log trong một thư mục khác trước khi chạy lại. Số epoch là thông số khởi đầu. GPU, driver và thư viện có thể ảnh hưởng tính xác định của trọng số. Lưu bản freeze, provenance và hash tạo được để tái chạy.

Bộ xuất ONNX dùng Python quản lý 3.10 với overlay CPU Torch 2.10.0+cpu của Voice Pack Manager. Trỏ đến thư mục `site-packages` của overlay hiện có, hoặc chuẩn bị overlay cục bộ bằng các wheel SHA-256 cố định.

```powershell
$converter = Join-Path $env:APPDATA 'com.voicemorph.studio/voice-packs/converter-torch210-cpu/python/Lib/site-packages'
# Nếu chưa có cache, dùng Python 3.10 x64 để chuẩn bị overlay riêng.
# $converter = Join-Path (Resolve-Path .rvc-training).Path 'converter-site-packages'
# py -3.10 -m pip install --no-deps --require-hashes --target $converter -r scripts/rvc-converter-requirements.txt
& $trainingPython scripts/rvc-original.py export --run .rvc-training/wind-run --runtime $runtime --converter-site-packages $converter
& $trainingPython scripts/rvc-original.py export --run .rvc-training/jade-run --runtime $runtime --converter-site-packages $converter
```

Mỗi pack có `voice-pack.json`, `voice.pth`, `voice.onnx` và `voice.index`. Giữ bốn tệp cùng thư mục. [Schema model pack](original-voice-pack.schema.json)

## 4. Cài, thử giọng và gỡ

Trong thẻ đúng tên, chọn **Import Pack → Choose voice-pack.json**. Xác nhận quyền và bấm **Install, validate & assign**. App đối chiếu checksum, provenance, hợp đồng tensor, encoder và index, sau đó chạy warm-up và inference. **Ready** là trạng thái kỹ thuật của runtime, chưa phải nghiệm thu chất lượng giọng. **Loading** là lúc khởi động phiên đổi giọng. **Error** hiển thị khi nạp, xác minh hoặc inference lỗi.

Chọn **Preview Voice** để mở phòng A/B. Dùng cùng WAV 48 kHz có quyền cho hai profile và nghe các mẫu Original, DSP, Neural, Hybrid. Chọn **Activate Voice** cho phiên live. Trong VoiceMorph chọn mic vật lý để thu, tai nghe để nghe và định tuyến đầu ra qua **CABLE Input**. Trong Discord chọn **CABLE Output** làm mic. Kiểm tra bằng mic test ngoài cuộc gọi. [Hướng dẫn thiết bị](HUONG-DAN-AGENT-VOICE.md)

**Delete Model** xóa ONNX và index được app quản lý cùng thông tin gán. Giữ tệp xuất và dataset gốc để cài lại. Nếu runtime đổi phiên bản, chọn **Revalidate pack** từ đúng manifest.

## 5. Đánh giá trước khi công bố

- Chạy benchmark với cả hai packs, từng provider và cùng tập WAV tách riêng. Dùng thiết bị và chất lượng đã ghi nhận. Wav đầu ra và báo cáo được giữ cục bộ.

```powershell
& $trainingPython -m pip install psutil
& $trainingPython scripts/benchmark-ai-runtime.py --runtime $runtime --model .rvc-training/wind-run/voice.onnx --original-pack .rvc-training/wind-run/voice-pack.json --input .rvc-training/heldout/source.wav --rights-confirmed --provider cuda --output .rvc-training/bench/wind-01
```

- Đọc p50/p95 IPC, inference và RTF. Đo độ trễ phần cứng riêng từ cùng tín hiệu phát đến tín hiệu thu ở CABLE Output, với vòng đo chung đồng hồ. Ghi thiết bị, buffer, dropout và percentile. Số đo offline không bao gồm đường truyền mic hoặc Discord.
- Ẩn danh tính thật khỏi người nghe bằng mã mẫu và xáo trộn thứ tự. Sau khi cung cấp mẫu làm quen có quyền, để từng người chép lại câu, chấm tự nhiên 1–5 và đoán danh tính. Dùng cùng câu nguồn, hai giọng đích và các ngôn ngữ đã chọn. Mỗi bản ghi trial gồm `listenerId`, `sampleId`, `transcript`, `naturalness` và `guess` (`wind-assassin`, `jade-guardian`, `unsure`). Tệp key là object từ `sampleId` đến `{profileId, language, reference}`. Không gửi key cho người nghe.
- Chạy `py -3.12 scripts/score-original-voices.py key.json trials.json` để tính WER, điểm tự nhiên trung bình và tỷ lệ nhận đúng cho từng giọng/ngôn ngữ. Chốt ngưỡng và cỡ mẫu trước khi thu kết quả. Nghiệm thu khi cả hai giọng qua thử nghe và phiên live qua mic ảo.

## Kết quả đã xác minh

Đã chạy trích HuBERT layer 12 trên WAV kiểm thử, tạo FAISS bằng thư viện thật, kiểm tra gói có giới hạn và nạp index trong adapter CPU/CUDA. Với ONNX Tsukuyomi chính chủ và dữ liệu index kiểm thử số học, CPU xử lý 8 khối với RTF 0,916, CUDA với RTF 0,222. Model này chỉ dùng kiểm tra adapter. Không cài nó vào hai profile gốc. Báo cáo chạy lưu tại `.ai-runtime-build/phase6-index-smoke/{cpu,cuda}.json`.

Chưa chạy huấn luyện hai giọng, chưa tạo preview có thể phát cho hai profile, chưa có số WER, điểm tự nhiên hay độ tách danh tính thực tế của chúng. Chưa nghiệm thu RMVPE trên release hiện có, DirectML hay latency qua mic ảo. Hai profile chưa đạt trạng thái phát hành.

[Ảnh desktop](screenshots/phase6/original-profiles.png) · [Hộp thoại nhập](screenshots/phase6/original-pack-import.png) · [Ảnh cửa sổ nhỏ](screenshots/phase6/original-profiles-compact.png)
