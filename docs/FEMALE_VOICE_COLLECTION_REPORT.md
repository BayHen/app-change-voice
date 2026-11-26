# Female Voice Collection — triển khai và kiểm chứng

Bộ cài sau checkpoint này có [bản sửa Soundboard](SOUNDBOARD_PLAYBACK_FIX.md);
hash trong dữ liệu benchmark Female Collection xác định đúng bản tại thời điểm kiểm tra đó.

Ngày: 26/11/2026, `feature/new-version`. Các thay đổi chưa commit. Collection nằm trong **Voice Lab → Female collection**, dùng sidebar/status bar và hệ navy/purple hiện có.

## Phạm vi thực tế

Collection có **20 cấu hình Classic DSP khác nhau và 1 danh tính AI đã được rà soát quyền sử dụng**, không phải 20 model AI. DSP dùng Signalsmith pitch/formant, EQ và vocal dynamics chạy thật trong worker hiện có. AI dùng model Tsukuyomi-chan chính thức qua w-okada/ONNX Runtime cục bộ. Không thay backend bằng mock; không tải thêm model hoặc dependencies không cần thiết.

20 preset: Natural Female, Soft Female, Sweet Female, Cute Girl, Young Woman, Mature Female, Deep Female, Elegant Female, Gentle Female, Bright Female, Anime Girl, Kawaii Voice, Korean Style, Japanese Style, Vietnamese Female, Female Streamer, Female Narrator, Female ASMR, Female Singer và Female Studio. [Thông số nguồn](../src-tauri/presets/female.json) và [kết quả render từng giọng](benchmarks/female-collection/dsp-speech.json) ghi thông số, peak/RMS, thời gian và SHA riêng.

Natural/Soft dùng mức nâng pitch/formant vừa phải; Mature/Deep giữ register thấp; Streamer/Narrator/Studio dùng leveling khác nhau; ASMR giảm treble và kiểm soát sibilance, không tạo hơi thở giả; Singer giữ dynamics nhẹ. Cute/Anime/Kawaii là hiệu ứng cách điệu. Korean/Japanese/Vietnamese là tên tonal starting point, **không tạo accent, không dịch và chưa được kiểm định phát âm tương ứng**.

Mỗi preset có vector thông số riêng; test cũng kiểm tra cả 20 WAV render có SHA khác nhau. Điều này chứng minh đường xử lý và tín hiệu khác nhau, không thay cho đánh giá nghe về 20 tính cách giọng hay chứng nhận độ tự nhiên. Classic DSP là phép xấp xỉ, không đủ để bảo đảm chuyển nam→nữ tự nhiên cho mọi người.

## Engine và điều khiển

- **Intensity** trộn dry/wet đã căn delay, mượt theo sample; không trộn giọng gốc vào AI. Pitch/formant độc lập, EQ shelf giữ nguyên bộ xử lý đã có.
- **De-essing** tách dải cao tại khoảng 5,5 kHz, envelope attack 1 ms/release 60 ms, mức tác động 0–1; giảm riêng thành phần cao khi vượt ngưỡng, không thêm noise model thứ hai.
- **Compression** RMS window khoảng 10 ms, threshold −18 dBFS, ratio 1:1–4:1, không auto makeup. Các tham số và gain thay đổi mượt. Đây là compressor đơn giản, chưa phải vocal mastering đa băng được đánh giá nghe.
- Giá trị dynamics bằng 0/1:1 trả tín hiệu nguyên vẹn. Giữ float headroom đến limiter/biên PCM, không clamp DSP sớm. Noise mặc định tắt trong các preset nữ; gate tắt; final limiter −1 dBFS bật.
- Thông số mới có default khi đọc schema/preset cũ; 8 preset trước đây vẫn tồn tại. Built-in registry hiện có 28 giọng DSP. Cập nhật schema dữ liệu tương thích, không yêu cầu xóa preset người dùng.
- Trên AI chỉ mở pitch transpose và profile/chunk thực sự được hỗ trợ. Formant, dry/wet intensity và post-DSP dynamics của AI chưa được hỗ trợ ở đường này, nên UI không đưa ra slider giả. Input/noise và gain monitor dùng engine hiện có; gain tai nghe không điều khiển nhầm output mic ảo.

Đã sửa việc chỉnh lần hai vô tình áp lại preset gốc sau khi backend xóa active preset ID. Collection giữ bản chỉnh đang hoạt động và preview dùng thông số đang chỉnh nếu giọng đó được áp dụng. Quay lại Voice Lab sẽ tải lại các giá trị live.

## Model và quyền sử dụng

| Mục                    | Giá trị                                                                                                                                               |
| ---------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| Danh tính              | Tsukuyomi-chan official RVC Normal 1, CV. Rei Yumesaki                                                                                                |
| Nguồn                  | [Trang tác giả](https://tyc.rei-yumesaki.net/work/software/rvc/)                                                                                      |
| Giấy phép/quyền giọng  | [Điều khoản RVC của chủ giọng](https://tyc.rei-yumesaki.net/work/software/rvc/terms/), `LicenseRef-TsukuyomiChan-Official`, không gán MIT cho weights |
| SHA-256 ONNX           | `d9caedd48ea81672946ce4896022da5e302996e1f5c1bf0ae2c141c2f7d3afe6`                                                                                    |
| Model / transport rate | 40 kHz model, PCM bridge 48 kHz mono s16le                                                                                                            |
| Format/runtime         | ONNX RVC; w-okada pin `7e2ce5c4800010bbc7604968bd29b802534fac05`; ORT GPU 1.21.0; Torch 2.6.0+cu124                                                   |
| Model local            | `C:\Users\Admin\AppData\Roaming\com.voicemorph.studio\ai-models` (đường dẫn file được manager giải quyết khi load)                                    |
| Runtime local          | `C:\Users\Admin\AppData\Roaming\com.voicemorph.studio\ai-runtime\releases\69c600bd252be1fd47f5bf9246294c976c32e27dfb4796b1712e92312d6504f7`           |

Quyền giọng đến từ chính nhà phát hành/chủ giọng, không suy diễn từ nhãn license code hoặc một model mirror. Điều khoản yêu cầu credit model và nguồn giọng được chuyển; việc tái phân phối/bundle có điều kiện riêng. Bản EXE này **không đóng kèm weights hay giọng preview**; nút tải hiển thị điều khoản và yêu cầu xác nhận trước khi chuẩn bị model. Người phân phối cần đọc đầy đủ điều khoản cho cách sử dụng của mình. File được kiểm tra hash/metadata; UI phân biệt “file verified” với model đã load thành công.

Runtime đã có từ tác vụ trước được tái sử dụng; không đổi bridge/runtimes nhiều GB chỉ để thêm collection. CPython 3.10.11 trong runtime đã hết hỗ trợ: vẫn là giới hạn cần giải quyết trước phát hành rộng, xem [báo cáo engine](AUDIO_QUALITY_REPORT.md). Các model nữ khác chưa có đủ hồ sơ quyền/kiểm định tương thích không được đưa vào catalog giả.

## Preview, favorites và lịch sử

File ONNX thực tế đã kiểm tra tại:
`C:\Users\Admin\AppData\Roaming\com.voicemorph.studio\ai-models\tsukuyomi-official-normal1-tsukuyomi-normal1.onnx`.

Tìm theo tên/mô tả, filter AI/DSP, favorites và recent hoạt động trên dữ liệu thật. Favorites/history chỉ lưu ID trong local WebView storage, không lưu âm thanh; không đồng bộ cloud. Collection không tự bật mic, tự download model hoặc tự phát preview khi mở.

Người dùng chọn WAV local và xác nhận có quyền sử dụng trước khi render. Classic chấp nhận mono/stereo PCM int16/24/32 hoặc float32, 16–96 kHz, file tối đa 32 MiB, đọc tối đa 5 giây. Preview Classic dùng vocal settings đã chọn, noise tắt và limiter an toàn; không thay live session. AI preview hiện yêu cầu WAV 48 kHz và live AI đã dừng, dùng model thực trong một private slot `0` riêng đúng yêu cầu upstream, kiểm tra hash trước/sau staging và dọn slot khi process kết thúc. Warmup bằng silence trước khi đưa câu nói vào; giữ SOLA upstream, không thêm crossfade trùng. Preview dùng đầu ra hệ thống, không tự route vào mic ảo.

Không coi tổng thời gian render offline (bao gồm load/verify AI) là latency realtime. Người dùng bấm Play mới phát; Stop dừng playback. Blob preview được thu hồi khi đổi giọng/thoát collection. Lỗi đọc file, quyền chưa xác nhận, runtime/model thiếu, warmup hoặc inference lỗi đều được báo.

## Dữ liệu và số đo

Mẫu tiếng Nhật chính thức của chủ giọng, 48 kHz mono PCM16, khoảng 1,68 giây; SHA `607d1dbb201330d017308fbc841abacbe12d672688af21d047b49be2c44dedd5`. [Điều kiện nguồn sample](https://tyc.rei-yumesaki.net/material/voice/sample-voice/) cho phép demo voice changer/effects. Credit: **Tsukuyomi-chan official sample voice (CV. Rei Yumesaki)**. Chỉ lưu WAV gốc/kết quả ở `.ai-runtime-build`, không đưa vào Git/bộ cài.

20 render Classic có peak khoảng **0,3977–0,4948**, full-scale sample **0**, output hash **20/20 khác nhau**. Render debug khoảng **196–274 ms** cho mỗi mẫu 1,68 s trên máy này; không phải tổng CPU process hay hardware round-trip. JSON ghi RMS và thông số riêng từng giọng. Test chuyển liên tiếp cả 28 preset giữ cùng pipeline, finite output và frame accounting theo hai mức batching RNNoise/DSP.

Benchmark inference thật, cùng model/sample, có warmup và instrument:

| Provider/profile | Inference median | IPC median | RTF gồm warmup |
| ---------------- | ---------------: | ---------: | -------------: |
| CUDA / 8.192     |         65,87 ms |   70,58 ms |          0,855 |
| CPU / 16.384     |        316,74 ms |  324,34 ms |          1,166 |

Nguồn: [CUDA](benchmarks/female-collection/ai-cuda-8192.json), [CPU](benchmarks/female-collection/ai-cpu-16384.json). Hai profile có chunk khác nhau; không dùng bảng để so latency cùng cấu hình. CPU chưa được qualified realtime: RTF gồm warmup >1, median IPC gần budget 341 ms; các lượt trước còn chậm hơn. CUDA 4.096 chỉ là gợi ý sau warmup từ đo trước; Balanced 8.192 và High 16.384 ưu tiên ổn định. Không đặt cùng pitch cho mọi người: input register và model cần chỉnh riêng, UI cho phép transpose thủ công.

CUDA lượt này: CPU process trung bình khoảng 5,10% của 16 logical CPU, RSS peak 2,35 GB (thập phân); Torch allocator peak allocated 381.396.480 byte, reserved 463.470.592 byte. CPU lượt kia: khoảng 18,65% CPU máy, RSS peak 2.316.918.784 byte. CUDA allocator không bao gồm toàn bộ ORT; nvidia-smi trong JSON là GPU toàn thiết bị, không gán cho riêng VoiceMorph. Máy có tác vụ build/desktop khác; không coi là benchmark phòng lab.

[Audio regression](benchmarks/female-collection/audio-regression.json): neutral THD+N −136,31 dB, bypass −150,09 dB, +6 dB trước limiter −134,32 dB, peak sau limiter 0,891251. Kết quả tín hiệu và chống alias bằng lượt sửa engine trước; không bỏ sinc/drift/prefill/PCM guards. THD+N của sine không phải chỉ số naturalness của speech.

## Rà soát nguồn tham khảo

- [w-okada](https://github.com/w-okada/voice-changer): tái sử dụng provider/private process và SOLA hiện có, không mở web server/public port.
- [RVC Project](https://github.com/RVC-Project/Retrieval-based-Voice-Conversion-WebUI): đánh giá pipeline/F0 và khả năng model; không cài WebUI/training stack thứ hai, license code không cấp quyền cho mọi giọng/model.
- [MASK](https://github.com/guilhermeeng99/mask): giữ kiến trúc DSP/mixer/routing hiện có, không thay các sửa lỗi audio vừa được đo.
- [ONNX Runtime CUDA](https://onnxruntime.ai/docs/execution-providers/CUDA-ExecutionProvider.html): giữ runtime đã qualified thay vì đổi CUDA/driver hệ thống; provider thực được kiểm tra khi load.
- [rubato](https://github.com/HEnquist/rubato): giữ sinc có trạng thái và buffer cấp trước.
- [FunDSP](https://github.com/SamiPerttu/fundsp): không thêm graph song song; hai dynamics đơn giản được thực hiện bằng trạng thái cố định trong chain hiện có.

Đã đọc `frontend-design` và `ui-ux-pro-max`, chạy tìm kiếm design system, rồi ưu tiên token/design system dự án thay vì áp palette/landing-page mẫu. Cards dùng dấu hiệu âm thanh và nhãn engine, không dùng chân dung giả để ngụ ý danh tính model. Inspector có scroll riêng; sidebar/status bar giữ chung.

Hướng thiết kế: background `#10121f`, sidebar `#121423`, surface `#191d2e`, accent purple
`#9360ff`, cyan `#22b5ff`, foreground `#f5f5fa`; giữ Segoe UI Variable và type scale dự án.
Header/search/filter nằm trên lưới, inspector của giọng đang chọn nằm bên phải. Ưu tiên nhãn engine,
trạng thái thật và kiểm soát nghe thử thay vì rating chất lượng hoặc chân dung không có căn cứ.
Desktop tối thiểu 900 px được kiểm tra; không tuyên bố đã chứng nhận giao diện mobile/OS Dynamic Type.

## Giới hạn chưa được xác minh

- Người dùng cung cấp link TikTok `@lchin.n/video/7707231758042860806` **chỉ để tham chiếu phong cách**. Công cụ không đọc được video; không tải, không đưa vào kiểm thử, training hoặc tạo model người nói. Không tuyên bố đã nghe hay phân tích clip.
- Chưa có mẫu tiếng Việt nam/nữ ba miền với quyền dùng đã xác minh. Không khẳng định tối ưu thanh điệu, phụ âm, accent Bắc/Trung/Nam hoặc độ dễ hiểu tiếng Việt.
- Mẫu đã kiểm thử là giọng nữ Nhật của chủ giọng. Chưa qualified nam→nữ tự nhiên trên người nói nam thực; không suy ra từ nhãn model hay một sine test.
- Không có đánh giá nghe mù/naturalness, MOS, WER hoặc đo hardware microphone-to-app latency. Automated test không được gọi là listening test. Cần đánh giá bằng người nghe/mẫu hợp lệ trước khi quảng bá chất lượng premium cho mọi giọng.
- Discord/OBS/Zoom thực tế và chạy nhiều giờ chưa được kiểm chứng. Không đưa kết quả test muted routing/file inference thành cam kết cuộc gọi hay zero dropout trong mọi điều kiện. Các counter thực và ảnh/bản native test được lưu riêng.

## Kiểm chứng và build

Đã qua **115 Rust tests**, 14 bài ignored theo mặc định; **25 frontend tests**, ESLint,
TypeScript/Vite, rustfmt và **6 Python tests** (protocol/PCM guards). Bài speech render
ignored được chạy riêng có chủ đích, không tự thu microphone. Các lỗi fixture/automation ban đầu
được sửa trước khi ghi nhận kết quả cuối; test `ok` không thay cho việc đọc counters.

Có test chống phát trễ sau khi rời collection: backend preview trả kết quả muộn thì không tạo
Audio/player mới. Đổi card cũng vô hiệu hóa kết quả preview cũ. Native command preview AI
đã trả 98.304 frame PCM ở 48 kHz, peak khoảng 0,148, không full-scale sample; tổng render/load/
verify khoảng 38,0 s ở lượt release cuối, **không phải latency realtime**. Giữ cả thời gian cold
load dài trong báo cáo thay vì chỉ công bố inference median nhanh.

[Native report](screenshots/female-collection/native-report.json) khớp SHA EXE cuối; đã kiểm tra
áp dụng đủ 20 preset, chỉnh pitch rồi formant không làm mất pitch, quay lại Voice Lab giữ giá trị,
consent trước đọc WAV, preview DSP/AI thật, model CUDA ready, monitor/routing với input/output mute,
layout 900/1280/1600 px và cả năm workspace có persistent status bar. Không ghi/thu microphone
hoặc phát WAV preview trong automated test, không giả vờ đã nghe hay đã gọi Discord.

Mã chính: `src/components/FemaleVoiceCollection.tsx`, `src/pages/VoiceLab.tsx`,
`src-tauri/presets/female.json`, `src-tauri/src/presets/{loader,models,validation,preview}.rs`,
`src-tauri/src/effects/{parameters,chain,dynamics}.rs`, `src-tauri/src/audio/pipeline.rs`,
Tauri commands trong `src-tauri/src/lib.rs`, hooks/types/services và tests/harness liên quan.

[VB-Cable dưới tải build, 60 s](benchmarks/female-collection/loopback-build-load-60s.json)
nhận khoảng 996,414 Hz cho tone 997 Hz; có **1 virtual underrun**, **0 virtual overflow**,
**1 capture overrun notification**, monitor 0 underrun; không stream fault, không diagnostic drop,
không sample full-scale/non-finite. Đây là tín hiệu tổng hợp với mic/headphone mute, không phải
giọng người qua cuộc gọi. Không bỏ lượt này để tuyên bố zero dropout hoặc kết luận không có
regression trong mọi điều kiện. Cần tiếp tục stress test hệ thống/thiết bị và đánh giá nghe.

[Lượt đối chiếu 60 s khi không còn build](benchmarks/female-collection/loopback-idle-60s.json)
được giữ riêng: khoảng **996,816 Hz**, **0 virtual underrun/overflow**, **0 capture overrun**
và **0 monitor underrun**. Các counter là số quan sát cho từng lượt, không phải bảo đảm cho
mọi điều kiện hay bằng chứng rằng sự kiện dưới tải build chưa từng xảy ra.

Kiểm tra Rust gồm registry, dynamics neutral/compression/de-essing, privacy/PCM/resampler/rings, switching và consent trước đọc file. Frontend test bao gồm search/filter/favorites, không tự start audio, áp dụng DSP và giữ lần chỉnh trước. Lệnh chính: `cargo test --manifest-path src-tauri/Cargo.toml --offline`, `npm.cmd test`, `npm.cmd run lint`, `npm.cmd run tauri:build:windows -- --bundles nsis`.

Render speech có chủ đích: `cargo test --manifest-path src-tauri/Cargo.toml --offline renders_every_female -- --ignored --nocapture`. Native harness: `scripts/verify-female-collection-native.mjs`; ảnh và số liệu tại [screenshots/female-collection](screenshots/female-collection). Hash/kích thước bộ cài cuối tại [build-evidence](benchmarks/female-collection/build-evidence.json). Không publish release, không tạo commit, không thay chính sách bảo mật/driver.
