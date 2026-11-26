# VoiceMorph Studio — kiểm tra chất lượng âm thanh

Artifact/hash trong báo cáo này thuộc checkpoint sửa engine trước Female Collection.
Bản EXE/bộ cài mới nhất và các giới hạn kiểm thử giọng nữ: [Female Voice Collection](FEMALE_VOICE_COLLECTION_REPORT.md).

Ngày kiểm tra: 26/11/2026. Nhánh `feature/new-version`. Thay đổi đang ở working tree; không tạo commit mới. Đây là báo cáo kiểm chứng trên máy này, chưa phải chứng nhận chất lượng cho mọi thiết bị/Discord.

## Nguyên nhân và bản sửa

| Tầng            | Phát hiện                                                                                         | Thay đổi                                                                                                                                                |
| --------------- | ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Gain/DSP/mixer  | Clamp ±1 trước limiter làm mất đỉnh sóng khi tăng makeup gain                                     | Giữ float headroom; chỉ giới hạn tại biên xuất PCM/limiter. Meter hiển thị được quá full scale                                                          |
| Capture         | DSP và RNNoise từng chạy trong callback thiết bị                                                  | Callback chuyển định dạng/downmix/gain/meter và đẩy SPSC; worker riêng chạy DSP/mixer và chuyển PCM tới worker AI                                       |
| Noise           | RNNoise vẫn tốn CPU khi tắt; gate theo từng sample đóng gần zero crossing; dry/wet lệch frame     | Bỏ inference noise khi mix bằng 0, dry delay đồng bộ, crossfade mượt; gate dùng RMS frame, hysteresis và hold 100 ms                                    |
| Resampling      | Linear/zero-order conversion không lọc alias, không bù hai clock độc lập                          | Sinc có trạng thái, buffer cấp trước; reader điều chỉnh ratio mượt tối đa ±3.000 ppm; prefill theo profile và tối thiểu bốn kỳ buffer được thương lượng |
| PCM             | Chuẩn hóa i16 không đối xứng; chưa dispatch i32; upstream AI cast float sang int16 không saturate | Thang 32768/2147483648, finite guards, i32 dispatch; chặn tràn trước cast upstream và sau resampling AI                                                 |
| Output          | Thiếu headroom cuối, thiếu điều tiết starvation                                                   | Limiter lookahead 1 ms, attack/release; fade khi thiếu dữ liệu và khi prime lại; bỏ dữ liệu cũ có đếm khi queue quá đầy                                 |
| Privacy/routing | Cấu hình mic ảo từng trỏ vào tai nghe vật lý; queue có thể giữ giọng cũ khi đổi mode              | Chỉ cho chọn endpoint ảo, đồng bộ AudioSession; generation/flush khi đổi chế độ/route. AI lỗi mặc định mute, không tự phát giọng gốc                    |
| Diagnostic      | Race khi bắt đầu capture sau thời gian idle                                                       | Publication/finalization kiểm tra generation dưới state lock; lỗi ghi/finalize được đánh dấu incomplete                                                 |

Luồng hiện tại: native capture → chuẩn hóa/downmix → input gain → bounded SPSC → DSP worker (noise → pitch/formant/creative EQ → Classic hoặc PCM AI → mixer) → hai bounded queues → monitor/virtual readers độc lập → output gain ở monitor → limiter → thiết bị. AI inference chạy trong process riêng, stdin/stdout cục bộ; không microphone upload, không public server. Recording thông thường và diagnostic dùng writer riêng.

Callbacks không chạy inference, pip, file I/O, network hoặc chờ mutex. Sinc playback vẫn chạy trong callback bằng `process_into_buffer` với bộ nhớ cấp trước; các số callback max bên dưới là thời gian thực đo, không phải bằng chứng hard realtime. Command/control có mutex và việc tạo/dừng stream có join worker; chúng ở ngoài data callback. Không tuyên bố đã kiểm chứng mọi allocation của driver/Signalsmith bằng profiler.

## Đo trước/sau bằng cùng tín hiệu

Nguồn: sine 997 Hz, 48 kHz, WAV float32, lấy đoạn ổn định 1 giây. `scripts/analyze-audio-quality.py` fit sin/cos least squares và tính năng lượng phần dư. [JSON đối chiếu](benchmarks/audio-quality/comparison.json). Không dùng SNR của tiếng nói để suy diễn độ tự nhiên.

| Trường hợp           | Trước THD+N |                Sau THD+N | Kết luận                                                                                          |
| -------------------- | ----------: | -----------------------: | ------------------------------------------------------------------------------------------------- |
| Neutral, biên độ 0,2 |  −130,56 dB |               −136,31 dB | Không thấy méo đáng kể trong bài sine                                                             |
| Bypass               |  −150,09 dB |               −150,09 dB | Giữ tham chiếu số học                                                                             |
| +6 dB, đầu vào 0,6   |   −22,75 dB | −134,32 dB trước limiter | Sửa đúng hard clipping: 17.786 sample bằng rail trước; sau 0 sample bằng rail, peak float 1,19707 |

Sau limiter −1 dBFS: peak 0,891251, không vượt full scale, THD+N −85,33 dB trên sine tăng gain. Limiter có thay đổi envelope có chủ đích; đây không phải output tuyến tính. `fullScaleSamples`/trường cũ `clippedSamples` trong diagnostic chỉ đếm sample ≥ full scale: float headroom >1 không đồng nghĩa đã hard clip. Báo cáo baseline cũ dùng estimator khác; dùng `comparison.json` để so trực tiếp.

Sine 18 kHz khi đổi 48→16 kHz: trước alias RMS 0,141421; sau RMS 0,00005709 kể cả transient khởi động (giảm khoảng 67,9 dB). Tail streaming được giữ giữa chunk; bài 1 giây cho 15.978 output frame vì còn delay/partial block, không reset/flush mỗi chunk. Không sử dụng THD+N của tín hiệu bị lọc bỏ làm chỉ số chất lượng.

Mô phỏng hai clock lệch ±1.000 ppm: mỗi chiều 180 giây, queue hữu hạn, không underrun/discard; sai số correction trung bình 30 giây cuối <250 ppm. Đây là thời gian mô phỏng, không phải ba phút chạy thiết bị.

## Profile và độ trễ

| Profile           | Buffer yêu cầu tại 48 kHz | Prefill tối thiểu profile | Sinc taps | AI CUDA gợi ý ban đầu |
| ----------------- | ------------------------: | ------------------------: | --------: | --------------------: |
| Low Latency       |                       128 |                     10 ms |        64 |           4.096 frame |
| Balanced mặc định |                       256 |                     20 ms |       128 |           8.192 frame |
| High Quality      |                       512 |                     40 ms |       256 |          16.384 frame |

Driver có quyền giới hạn buffer. Máy kiểm tra thương lượng capture/playback 480 frame, 48 kHz f32 stereo; mono DSP. Prefill còn bị nâng lên ít nhất bốn kỳ buffer của endpoint: khoảng 40 ms ở thiết bị này. Vì thế không hứa Low Latency đạt 128 frame trên máy này. Apply profile dừng Monitor và routing; người dùng chủ động bật lại. AI chunk chỉ đổi khi người dùng áp dụng gợi ý và AI đã dừng.

Audio Quality Lab hiển thị format/rate/buffer, peak/RMS/clipping, limiter, queue, underrun/overflow, callback max, drift, AI budget/missed deadlines và trạng thái route. Pipeline estimate cộng các term cấu hình DSP, buffer, queue và FIR/limiter; không đo được ADC/DAC, Windows mixer, Discord hay trễ mạng. Chế độ AI có thêm context/chunk/SOLA/prefill riêng; không dùng con số Classic làm tổng latency AI. Không có số đo hardware round-trip thì hiển thị chưa đo.

Các thử nghiệm trước khi tăng prefill được lưu tại [prefill-experiments](benchmarks/audio-quality/prefill-experiments): Low có 20 virtual underrun/15 s; High 0/15 s; Balanced 3/60 s, có lúc playback callback debug lên 32 ms trong lúc máy build. Không xóa các kết quả không đạt. Chi tiết lượt chạy cuối nằm trong JSON kiểm tra thiết bị, không suy diễn từ trạng thái test `ok` rằng không có dropout.

## AI thật, model và tài nguyên

Lượt cuối sau tăng prefill: [Low 15 s](benchmarks/audio-quality/virtual-low-15s.json),
[High 15 s](benchmarks/audio-quality/virtual-high-15s.json),
[Balanced 60 s](benchmarks/audio-quality/virtual-balanced-60s.json).
Cả ba route mic ảo có 0 underrun / 0 overflow; Balanced nhận 997,0204 Hz, peak 0,04011,
WAV mixer/virtual 30 giây có 0 diagnostic drop và 0 non-finite/full-scale sample.
Playback monitor cũng 0 underrun; capture có 1 overrun notification mỗi lượt, chưa ghi timestamp
để quy nó cho startup. Không coi số này là 0 hoặc khẳng định mọi audio path đã hoàn hảo.
Balanced callback max debug: capture 686,5 µs, playback 7.196,4 µs; không thay bằng số CPU tổng process.

Runtime mới: `w-okada-vm-cuda124-2026.11.26.quality1`, manifest SHA-256 `69c600bd252be1fd47f5bf9246294c976c32e27dfb4796b1712e92312d6504f7`. Runtime cũ không khớp bridge guard; phải update bằng Runtime Management. Bản local đã dựng tại `.ai-runtime-build/bootstrap-installed/releases/<SHA>`. Kiểm tra dependency thật nhận RTX 3080 10 GB, Torch CUDA 12.4, ORT CUDA/CPU. Không cài driver/kernel/system CUDA.

Dependencies chính: CPython 3.10.11; Torch 2.6.0+cu124; torchaudio 2.6.0; ONNX Runtime GPU 1.21.0; NumPy 1.23.5; SciPy 1.10.1; fairseq 0.12.2 (inference source, không native training extensions). 69 artifact chính thức có SHA/size pin trong recipe. **Python 3.10 đã hết hỗ trợ: còn blocker phát hành rộng**, cần nâng stack và kiểm định lại; không gọi runtime hiện tại là an toàn dài hạn.

Upstream w-okada pin `7e2ce5c4800010bbc7604968bd29b802534fac05`. Giữ SOLA/crossfade upstream, không chồng overlap mỗi chunk ở Rust. Rust chỉ fade lúc prime/starve và resample hai biên khi cần. Inference quá budget liên tiếp được cảnh báo, rồi fault có giải thích nếu backlog kéo dài; không tự động restart vòng lặp khi máy không đủ nhanh. Crash recovery/fallback có chính sách opt-in; CPU chạy được không đồng nghĩa CPU đủ realtime.

Model: Tsukuyomi-chan official RVC Normal1, lấy checkpoint từ tác giả, chuyển ONNX đã xác minh SHA `d9caedd48ea81672946ce4896022da5e302996e1f5c1bf0ae2c141c2f7d3afe6`. Credit: **Tsukuyomi-chan official RVC model (CV. Rei Yumesaki)**. Đây là điều khoản tùy chỉnh của chủ giọng, không gán nhãn MIT cho model. [Điều khoản model](https://tyc.rei-yumesaki.net/work/software/rvc/terms/), [quyền dùng sample](https://tyc.rei-yumesaki.net/material/voice/sample-voice/). Sample WAV chính thức 1,68 giây, SHA `607d1dbb201330d017308fbc841abacbe12d672688af21d047b49be2c44dedd5`. Không dùng sample/model VCClient-only trong ứng dụng khác; không suy diễn quyền giọng người dùng. Weights và speech kết quả chỉ nằm trong thư mục local bị git ignore.

Benchmark runtime guarded mới (chạy tuần tự, sample lặp, có instrument, không live audio):

| CUDA chunk | Inference median | IPC median | RTF gồm warmup |
| ---------- | ---------------: | ---------: | -------------: |
| 4.096      |         52,46 ms |   55,10 ms |          1,039 |
| 8.192      |         61,52 ms |   66,58 ms |          0,911 |
| 16.384     |         64,53 ms |   74,34 ms |          0,563 |

4.096 là gợi ý có điều kiện sau warmup, không bảo đảm cold-start realtime; 2.048 bị loại vì RTF 1,73 trong benchmark trước guard. CPU 16.384 trước guard có RTF 1,52, không đủ realtime trên i5-13400F; nút gợi ý CPU bị tắt. File reports ghi RSS/CPU-time/CUDA allocator và nvidia-smi: allocator chỉ Torch, nvidia-smi là toàn GPU, không gán nhầm thành riêng process. Không có kết luận chất lượng nghe tự nhiên từ các số throughput.

Benchmark CUDA 8.192 ghi process CPU trung bình khoảng 4,71% trên 16 logical CPU,
RSS peak khoảng 2,30 GiB; Torch CUDA peak allocated 363,73 MiB và reserved 442 MiB.
GPU toàn máy được quan sát 1–23%, khoảng 1.108–2.080 MiB trong lượt này; gồm desktop/ORT/
ứng dụng khác nên không coi là mức riêng VoiceMorph. Các [JSON AI](benchmarks/audio-quality/ai)
giữ số liệu gốc và định nghĩa scope. PCM saturation guards có 0 intervention trên sample chính thức.

[Crash recovery thật](benchmarks/audio-quality/crash-recovery.json) qua CUDA → dừng đúng child
→ CPU opt-in: 1 restart, 29 chunk ở snapshot phục hồi; Stop phát zero, Start lại giữ endpoint.
CPU trả được PCM trong test không chứng minh CPU đủ realtime.

## Thư viện đã đánh giá

| Nguồn                                                      | Quyết định                                                                                                     |
| ---------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| [rubato](https://github.com/HEnquist/rubato)               | Thêm duy nhất crate `=0.16.2`, MIT, API preallocated đã kiểm thử; pin có chủ ý, không tuyên bố là bản mới nhất |
| [ringbuf](https://github.com/agerasev/ringbuf)             | Không thêm: `rtrb` hiện có đáp ứng bounded SPSC, tránh thay queue khi chưa có bằng chứng cần                   |
| [MASK](https://github.com/guilhermeeng99/mask)             | Giữ chức năng DSP hiện có, không thay toàn bộ engine bằng một dependency mới                                   |
| [fundsp](https://github.com/SamiPerttu/fundsp)             | Không thêm graph DSP song song với chain đã có                                                                 |
| [RustFFT](https://github.com/ejmahler/RustFFT)             | Không thêm FFT ngoài thuật toán pitch hiện có chỉ để đổi thư viện                                              |
| [dasp](https://github.com/RustAudio/dasp)                  | PCM conversions hiện được kiểm thử trực tiếp; chưa cần abstraction/dependency khác                             |
| [RNNoise](https://github.com/xiph/rnnoise)                 | Giữ `nnnoiseless` hiện có, tùy chọn, sửa routing/mix/gate trước                                                |
| [DeepFilterNet](https://github.com/Rikorose/DeepFilterNet) | Không tải model hay thêm inference noise thứ hai khi chưa có A/B speech hợp lệ chứng minh lợi ích              |
| [w-okada](https://github.com/w-okada/voice-changer)        | Giữ source pin và SOLA, thêm saturation/health/budget guard có kiểm thử                                        |

Không thêm dependency frontend. Shared Radix/Lucide/components và giao diện navy/purple hiện có được giữ. Các repository không được đưa vào sản phẩm không được coi là đã hoàn tất audit giấy phép phân phối.

## Mã nguồn, cách kiểm chứng và giới hạn

Các nhóm thay đổi chính: `audio/{stream,pipeline,noise,resample,quality,quality_tests,metrics,meter,capture,recording}.rs`; `effects/{gain,chain,limiter}.rs`; `routing/{mod,windows}.rs`; `ai/{engine,transport}.rs`; `lib.rs`; `ai-sidecar/{bridge.py,runtime-recipe.json,test_pcm_guard.py}`; `AudioQualityLab`, status bar, session/routing hooks/types/services; scripts test/analyze và tài liệu. Backend DSP/MASK-inspired controls, soundboard, recordings, virtual microphone và AI tiếp tục tồn tại; không thay bằng mock.

Lệnh tái lập: `cargo test --manifest-path src-tauri/Cargo.toml --offline`; `npm.cmd test`; `npm.cmd run lint`; `.ai-runtime-build/python310/python.exe -m unittest discover -s ai-sidecar -p 'test_*.py'`; `npm.cmd run tauri:build:windows -- --bundles nsis`. Các test hardware/ghi WAV đều ignored theo mặc định; chỉ chạy bằng lựa chọn developer. `scripts/verify-audio-quality-native.mjs` kiểm tra bridge WebView2 thật, không thu raw mic, khôi phục volume/profile sau test. Các WAV chẩn đoán được chọn rõ tầng và thời lượng 1–30 giây, không tự ghi khi mở app, có badge toàn app, manifest và đường dẫn local. Bản ghi thiếu/lỗi được đánh dấu incomplete; queue drop được báo.

Chưa hoàn tất: đánh giá nghe mù trên nhiều người/giọng; cuộc gọi Discord/OBS/Zoom thực tế; hardware microphone-to-capture impulse round-trip; chạy liên tục nhiều giờ; profiler allocation mọi thư viện/driver; chất lượng neutral ở mọi rate và thiết bị. Noise path ngoài 48 kHz hiện vẫn có native→48k→native kể cả tắt denoise (cần giữ latency/crossfade); vì thế chưa hứa bit-transparent băng cao ở mọi rate. Dry recording alignment Classic đã sửa theo sample delay, nhưng AI Original/Processed alignment chưa được đo và không thể suy ra từ chunk cấu hình. Không bịa số đo cho những phần này.

## Kết quả cuối và bộ cài

- **112 Rust tests passed**, 13 bài ignored không tự mở thiết bị/tải runtime. Các bài được chọn rõ đã chạy riêng: runtime assembly/probe, CUDA inference và owned-child recovery, synthetic benchmark, loopback Low/High/Balanced. Silence, sine, impulse alignment, sweep/mixed harmonics, 44,1/48/96/16 kHz, PCM f32/i16/u16/i32, noise enable warmup và privacy gate đều có kiểm tra.
- **22 frontend tests passed**, ESLint, TypeScript/Vite, rustfmt và Python **6 tests passed** (protocol + saturation/non-finite).
- [Balanced 120 giây](benchmarks/audio-quality/virtual-balanced-120s.json): mic ảo 0 underrun/overflow, 996,9966 Hz, capture 1 overrun notification, không stream fault/discard. WAV diagnostic 30 giây mỗi tầng mixer/virtual, 0 dropped/non-finite/full-scale. Playback callback max debug 12,47 ms từng vượt kỳ 10 ms, nhưng queue đủ giữ route không underrun trong lượt này; không hứa hard realtime.
- [Native WebView2 evidence](screenshots/audio-quality/native-report.json) và [ảnh viewport](screenshots/audio-quality/measured-viewport.png): native command thật, không mock; kiểm tra consent, WAV mixer/virtual im lặng, profile dừng audio, GPU/model CUDA ready khi audio đã dừng, layout 900 px không tràn ngang. Ảnh được đối chiếu với navy/purple/sidebar/status bar của hệ thống hiện có; không thay giao diện bằng một dashboard mẫu khác.
- Build optimized EXE và **NSIS thành công** với `npm.cmd run tauri:build:windows -- --bundles nsis`. MSI không được xác nhận: WiX ICE báo Windows Installer service không truy cập được; không sửa dịch vụ/registry/bảo mật để vượt lỗi này.

EXE: `src-tauri/target/x86_64-pc-windows-msvc/release/voicemorph-studio.exe`.
Bộ cài: `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/VoiceMorph Studio_0.1.0_x64-setup.exe`.
Mã SHA và kích thước được ghi ở [build-evidence.json](benchmarks/audio-quality/build-evidence.json).
Runtime đã được cài/kích hoạt qua lệnh native tại
`C:\Users\Admin\AppData\Roaming\com.voicemorph.studio\ai-runtime\releases\69c600bd252be1fd47f5bf9246294c976c32e27dfb4796b1712e92312d6504f7`.
Model đã được phê duyệt nằm trong `C:\Users\Admin\AppData\Roaming\com.voicemorph.studio\ai-models`.
Không đưa runtime nhiều GB/model/giọng đã chuyển vào Git hay gửi ra ngoài. Giấy phép Rubato đầy đủ được đóng kèm frontend tại `licenses/rubato-0.16.2-MIT.txt`.

Khi dùng: đóng bản app cũ, mở EXE mới, chọn mic HyperX → tai nghe thật → **CABLE Input** ở VoiceMorph;
Discord nhận **CABLE Output**. Bắt đầu Balanced/Classic, mở Monitor rồi Start routing; thử AI CUDA
sau khi Classic ổn. [Hướng dẫn tiếng Việt](HUONG_DAN_SU_DUNG.md) mô tả từng bước và cách đọc lỗi.
Các test khôi phục volume/profile, dừng Monitor/routing/AI khi kết thúc; không để giọng thử phát vào cuộc gọi.
