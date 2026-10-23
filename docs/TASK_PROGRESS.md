# Tiến độ Task 01

## Task 15 - Automated Testing & Performance Optimization

- Thêm Audio Diagnostics panel và export report privacy-safe qua temporary file.
- PASS 72 Rust automated + 9 Vitest; 7 ignored Rust test chạy riêng (1 benchmark, 6 Windows hardware).
- Hardware PASS: microphone start/stop, DSP thực tới VB-CABLE, endpoint open/stop, stable IDs và sample-rate mismatch.
- Benchmark release: 10 giây/143.34 ms (ratio 0.014); algorithmic latency 58.00 ms.
- `npm audit`: 0 vulnerability. `cargo audit`: 0 vulnerability, 4 warning.
- Release EXE và NSIS x64 tạo thành công; MSI BLOCKED bởi WiX `LGHT0217/LGHT0216`/Windows Installer service.
- Chưa đo loopback, session/memory 30 phút–2 giờ, sleep/resume hoặc Discord/OBS/VALORANT.
- Release: NO-GO production đến khi đóng `RELEASE_READINESS.md`; không triển khai Task 16.

## Task 14 - Preset Sync, Backup & Restore

- Local backup schema v1: custom presets, hotkeys, gain/noise, profiles và portable Soundboard metadata.
- Export/Import All Presets, preview, conflict detection và Restore Merge/Replace.
- Safety snapshot tại `backup-safety/latest.json`; rollback state khi restore lỗi.
- Không backup recording/raw audio, secret/token, WASAPI ID, driver hoặc absolute Windows path.
- File giới hạn 4 MiB; kiểm tra schema, range và duplicate IDs.
- `SyncProvider` local-only. Cloud sync DEFERRED; chưa có server/login/token/upload.

## Task 13 - Plugin Architecture & Soundboard

- Hoàn thành registry/manifest/factory schema v1 và validation trusted/platform/API/realtime/parameter.
- Built-in Trim factory triển khai `AudioProcessor`; không load DLL hoặc native code không tin cậy.
- Soundboard có add/remove/rename, favorite, volume, category, route, search, Play/Stop và global hotkey.
- Metadata ghi atomic tại `app_data_dir/soundboard/library.json`; mixer bounded 16 voices và SPSC commands.
- Frontend build/Vitest: PASS 9/9. Rust: PASS 63 test tự động; 4 manual/ignored.
- Hardware playback, hotkey ngoài focus và virtual cable/Discord: NOT TESTED, cần kiểm thử manual thực tế.
- Hiện chỉ hỗ trợ WAV; bank realtime nạp lại khi Start Monitoring. Third-party binary plugin chưa được thực thi.

## Đã hoàn thành

- Scaffold Tauri 2 + React + TypeScript tại root repository hiện tại.
- Cấu hình Tailwind CSS, ESLint, Prettier, Vitest và rustfmt.
- Tạo UI VoiceMorph Studio với sidebar, Dashboard, Voice Effects, Voice Studio, Settings và version.
- Tạo command `get_app_status` để xác minh React ↔ Rust.
- Tách module Rust `audio`, `effects`, `platform`, `presets` và thêm unit test.
- Cấu hình app version `0.1.0`, tên hiển thị và build Windows x64.
- Kiểm tra toolchain Windows và ghi nhận dependency còn thiếu.

## Kết quả lệnh xác minh

| Lệnh                              | Kết quả                                                |
| --------------------------------- | ------------------------------------------------------ |
| `npm.cmd install`                 | Thành công — 193 package, 0 vulnerability              |
| `npm.cmd run build`               | Thành công — TypeScript và Vite production build       |
| `npm.cmd run test`                | Thành công — 1 test file, 1 test                       |
| `npm.cmd run lint`                | Thành công                                             |
| `npm.cmd run format`              | Thành công                                             |
| `npm.cmd run dev`                 | Thành công — Vite trả HTTP 200 tại port 1420           |
| `cargo check --all-targets`       | Thành công — không có compiler warning                 |
| `cargo clippy -- -D warnings`     | Thành công — không có Clippy warning                   |
| `cargo test`                      | Thành công — 4/4 Rust unit test                        |
| `cargo fmt -- --check`            | Thành công                                             |
| `npm.cmd run tauri:dev`           | Thành công — chạy `target/debug/voicemorph-studio.exe` |
| `npm.cmd run tauri:build:windows` | Thành công — tạo NSIS và MSI x64                       |

Rust, MSVC Build Tools và Windows SDK đã được nhận diện. Rust compile, cửa sổ Tauri dev và hai bộ
cài Windows x64 đã được xác nhận trong quality gate ngày 22/10/2026.

## Task 02 - Desktop UI & Navigation

### Đã hoàn thành

- Xây dựng dark desktop shell responsive từ 900x600 với sidebar và mobile navigation fallback.
- Hoàn thiện Dashboard, Voice Effects, Voice Studio và Settings.
- Tạo component dùng lại: `AppSidebar`, `AppHeader`, `DeviceSelector`, `AudioMeter`,
  `VolumeSlider`, `EffectCard`, `VoiceControls`, `ToggleSwitch`, `StatusIndicator` và
  `NotificationToast`.
- Tách UI state khỏi backend audio state bằng TypeScript interfaces.
- Thêm loading, error, disabled, empty state và keyboard focus states.
- Giữ meter ở zero khi audio offline; không dùng waveform hoặc dữ liệu realtime giả.
- Đánh dấu toàn bộ action cần audio engine là disabled hoặc Coming Soon.
- Thêm test DOM cho navigation, render, microphone offline, preset search, slider và toggle.
- Chụp screenshot kiểm tra responsive tại 900x600 trong `docs/screenshots/`.

### Kết quả kiểm thử

- `npm.cmd run build`: thành công, TypeScript không có lỗi.
- `npm.cmd run lint`: thành công.
- `npm.cmd run format`: thành công.
- `npm.cmd run test`: thành công, 2 test files và 5 tests.
- Resize smoke test: thành công tại 1440x900 và 900x600, không tràn ngang.
- `cargo check`, `cargo test`, `cargo fmt -- --check` và `cargo clippy --all-targets -- -D warnings`:
  thành công; 4/4 Rust unit tests đạt và không có compiler warning.
- Tauri runtime smoke test: thành công; `target/debug/voicemorph-studio.exe` mở cửa sổ
  `VoiceMorph Studio`, tiến trình phản hồi và command `get_app_status` cung cấp trạng thái backend.

## Ngoài phạm vi

- Thu/phát âm thanh, DSP và voice effects thực tế.
- macOS adapter.
- Code signing và phát hành.
- Push GitHub hoặc commit tự động.

## Task 11 - Windows Virtual Microphone Integration

### Đã triển khai

- Chọn phương án virtual cable được người dùng cài riêng; không cài/chỉnh kernel driver tự động.
- Thêm abstraction `AudioRoutingService`, `VirtualDeviceManager`, `VirtualOutputSink`, cấu hình và status.
- Fan-out processed audio từ cùng input/DSP pipeline sang bounded SPSC routing queue; không capture mic hoặc
  chạy DSP lần hai.
- Thêm WASAPI virtual output sink, conversion f32/i16/u16, mono-to-multichannel, streaming sample-rate
  conversion, limiter, meter và underrun/overflow/fault counters.
- Thêm trang Virtual Microphone: discovery/selection, Start/Stop, trạng thái thật, meter, Windows Sound
  Settings, hướng dẫn endpoint ghép cặp và cảnh báo feedback.
- Persist selection tại `app_config_dir/audio-routing.json`; không tự động mở microphone/routing khi restart.
- Thêm `docs/VIRTUAL_MIC_WINDOWS.md`.

### Giới hạn nghiệm thu

- Cần VB-CABLE hoặc virtual cable tương đương được cài riêng để chạy test end-to-end.
- `Routing active` xác nhận playback stream đã mở, không giả định capture endpoint đang được Discord/OBS dùng.
- Chất lượng resampling hiện dùng converter streaming nhẹ, phù hợp voice; chưa phải resampler chất lượng
  mastering. Độ trễ end-to-end phải đo loopback trên thiết bị thật.
- Driver riêng (phương án B), installer driver, code signing và Task 12 nằm ngoài phạm vi.

### Kết quả xác minh Task 11 (23/10/2026)

- PASS — `npm.cmd run format`, `npm.cmd run lint`, `npm.cmd run test` (8/8) và `npm.cmd run build`.
- PASS — `cargo fmt -- --check`, `cargo check --all-targets`,
  `cargo clippy --all-targets -- -D warnings` và `cargo test` (55 pass, 4 manual/benchmark ignored).
- PASS — `npm.cmd run tauri:dev` compile và mở `target/debug/voicemorph-studio.exe`; phiên được dừng chủ
  động bằng Ctrl+C sau smoke test.
- PASS — release binary x64 và NSIS installer được tạo.
- FAIL (môi trường đóng gói) — MSI/WiX dừng tại ICE validation với `LGHT0217/0x643` vì Windows Installer
  service không truy cập được. File MSI cũ trong `target` không được tính là artifact mới.
- NOT TESTED — routing qua VB-CABLE, Discord microphone test, OBS capture, disconnect, preset/recording khi
  route, sample-rate mismatch thực tế và latency end-to-end; máy test chưa xác minh có virtual cable endpoint.

### Xác minh lại Task 11 sau khi cài driver (23/10/2026)

- PASS — Windows PnP báo `CABLE Input` playback và `CABLE Output` capture của VB-Audio Virtual Cable 3.3.1.7
  (signed, INF `oem0.inf`) ở trạng thái OK. Một instance VB-Audio thứ hai/16-channel báo Error và không được
  dùng cho kiểm thử.
- PASS — CPAL enumerate thiết bị, mở/dừng input/output mặc định hai lần và giải phóng stream.
- PASS — mở/dừng VB-CABLE playback endpoint thật; virtual sink không fault.
- PASS — nguồn 44.1 kHz được virtual sink chấp nhận/chuyển đổi theo cấu hình endpoint.
- PASS — một microphone pipeline duy nhất fan-out processed signal sang virtual routing, trạng thái Active,
  dừng sạch và không fault trong smoke test 500 ms.
- FAIL — release executable x64 build thành công nhưng cả NSIS và MSI hiện dừng sớm tại bước patch bundle
  metadata với `Access is denied (os error 5)`. WiX chưa được gọi nên lỗi hiện tại không còn là kết quả
  `LGHT0217`; cần kiểm tra antivirus/Controlled Folder Access hoặc policy chặn sửa PE executable.
- NOT TESTED — Discord Mic Test, OBS capture, device disconnect khi đang route và đo latency loopback.

## Task 11.1 - Discord Virtual Microphone Integration

- Settings có Discord Integration dùng trực tiếp Task 11 routing service: endpoint selector, refresh,
  Start/Stop, trạng thái, processed meter, signal detection và hướng dẫn Mic Test.
- Không có Discord API/bot, không tự sửa Discord settings và không tạo audio/DSP pipeline thứ hai.
- Endpoint được lưu bằng ID + fingerprint fallback; UI không tuyên bố Discord Connected chỉ từ Routing Active.
- Tài liệu cấu hình và checklist manual nằm trong `docs/DISCORD_INTEGRATION.md`.
- Backend hardware PASS: VB-CABLE 3.3.1.7, stream open/stop, 44.1 kHz adaptation và live processed fan-out.
- Discord Mic Test/preset/mute/hotkey/reconnect thực tế: NOT TESTED cho tới khi người dùng xác minh trong Discord.

## Task 11.2 - System-Wide Voice Changer

- Trang Games & Apps dùng cùng device service, `AudioStreamManager`, processed meter và virtual routing Task 11.
- Profile schema v1 có CRUD/persistence và default VALORANT, Discord, OBS; người dùng apply thủ công.
- Apply profile cập nhật preset, noise suppression và limiter; không restart engine, không dò/hook process.
- Hướng dẫn riêng cho system-wide routing, VALORANT, Discord, OBS và troubleshooting đã được tạo.
- Soundboard preference/volume được lưu trong profile để UI mô tả cấu hình; master soundboard enable/gain realtime
  chưa có control chung nên chưa được apply vào mixer trong phiên bản này.
- VALORANT/Discord/OBS đồng thời, disconnect, sleep/resume và long-running session: cần manual acceptance.

## Task 12 - Global Hotkeys & System Tray

### Đã triển khai

- Tích hợp `tauri-plugin-global-shortcut` 2.4.1 và Tauri `tray-icon` với Tauri 2.12.5.
- Hotkey cho mute microphone, effects, preset kế/ trước, Hear Myself, virtual routing và recording.
- Editor ghi tổ hợp phím, remove/reset, duplicate/reserved validation, OS conflict rollback và persistence.
- Windows tray với trạng thái backend thật, Show/Hide, Settings và Exit cleanup.
- Tùy chọn minimize/close to tray; mặc định close-to-tray bật, minimize-to-tray tắt.
- Hotkey/tray đi qua application service; audio callback không nhận IPC, mutex hoặc UI work.
- Cấu hình lưu tại `app_config_dir/hotkeys.json`; restart không tự capture microphone.

### Kiểm thử

- Unit test: default shortcut parsing, duplicate/reserved validation, save/load sau restart và hide/restore
  decision; audio stop idempotence tiếp tục là regression test cho cleanup.
- Frontend test: editor assign, duplicate warning, remove và save; navigation/regression tests giữ nguyên.
- Conflict với ứng dụng bên ngoài, hotkey khi app mất focus, tray click và Windows restart cần smoke test desktop
  thực tế; không được đánh dấu PASS chỉ từ unit test.

### Kết quả xác minh Task 12 (23/10/2026)

- PASS — `npm.cmd run format`, `npm.cmd run lint`, `npm.cmd run test` (9/9) và `npm.cmd run build`.
- PASS — `cargo fmt -- --check`, `cargo check --all-targets`,
  `cargo clippy --all-targets -- -D warnings` và `cargo test` (59 pass, 4 manual/benchmark ignored).
- PASS — `npm.cmd run tauri:dev`: Tauri window mở, plugin đăng ký default shortcuts và tray khởi tạo không lỗi;
  phiên được dừng chủ động bằng Ctrl+C.
- PASS — release x64 và NSIS installer được tạo bằng `tauri build --bundles nsis`.
- NOT TESTED — nhấn từng global hotkey khi app mất focus, menu tray bằng chuột, conflict với ứng dụng bên
  ngoài và restart Windows; cần thao tác người dùng/desktop interactive để xác nhận end-to-end.
- NOT TESTED — start/stop virtual routing từ tray với cable thật do Task 11 chưa có hardware acceptance trên
  máy hiện tại. Error path vẫn trả notification thay vì giả báo thành công.

## Task 03 - Audio Device Management

### Đã hoàn thành

- Tích hợp CPAL 0.18.2 với WASAPI mặc định trên Windows.
- Liệt kê input/output, nhận diện default và trả metadata sample rate, channels, sample format,
  buffer range khi driver hỗ trợ.
- Dùng CPAL `DeviceId` làm ID chính; dùng fingerprint metadata khi driver đổi ID.
- Thêm các command `get_input_devices`, `get_output_devices`, `get_default_devices`,
  `select_input_device`, `select_output_device` và `refresh_audio_devices`.
- Lưu lựa chọn tại `app_config_dir/audio-devices.json`; fallback về default có thông báo.
- Dashboard dùng dữ liệu backend thật, hỗ trợ refresh và tự kiểm tra ngắt thiết bị mỗi 5 giây.
- Không tạo hoặc khởi động audio stream.

### Phân loại kiểm thử

- Unit test tự động kiểm tra nhiều input, ID thay đổi, thiết bị biến mất, default thay đổi và không có
  microphone.
- Test frontend kiểm tra render metadata và refresh command.
- Hai hardware test được đánh dấu `#[ignore]`; chỉ được báo PASS khi chạy thủ công trên máy có thiết bị.

### Kết quả xác minh

- `npm.cmd run test`: thành công, 6/6 frontend tests.
- `npm.cmd run build`, `npm.cmd run lint`, `npm.cmd run format`: thành công.
- `cargo test`: thành công, 9 unit tests; 2 hardware tests được bỏ qua theo mặc định.
- `cargo test manual_hardware_tests -- --ignored --nocapture`: thành công 2/2 trên máy Windows hiện
  tại; WASAPI báo ít nhất một input và CPAL ID không trùng trong snapshot.
- `cargo check`, `cargo fmt -- --check`, `cargo clippy --all-targets -- -D warnings`: thành công.

## Task 04 - Real-Time Audio Pipeline

### Đã hoàn thành

- Thêm `AudioProcessor`, `PassthroughProcessor`, `AudioPipeline`, `AudioStreamManager` và
  `AudioMetrics`.
- Tạo CPAL input/output stream với internal mono float32 và conversion f32/i16/u16.
- Thêm bounded lock-free SPSC ring buffer `rtrb` 0.4.0; underrun phát silence, overflow bỏ sample mới.
- Ưu tiên 48 kHz khi cả hai thiết bị hỗ trợ; fallback sang sample rate chung.
- Hỗ trợ downmix input nhiều channel và nhân mono ra mọi output channel.
- Thêm start, stop, pause, resume, status/metrics Tauri commands; không truyền frame qua IPC.
- Dashboard kết nối monitoring thật, meters từ peak thật, cảnh báo tai nghe và hiển thị metrics.
- Chọn thiết bị trong lúc monitoring sẽ stop/drop stream trước khi thay đổi lựa chọn.
- Không tự động bật monitoring khi khởi động.

### Realtime safety

- Callback không lock mutex, không I/O, không log, không network và không gọi UI/IPC.
- Ring buffer được allocate trước khi stream chạy; callback không cấp phát heap.
- Metrics dùng atomics. Error callback chỉ tăng counter và đặt fault flag.
- Estimated latency không phải kết quả đo loopback và chưa dùng để khẳng định mục tiêu dưới 50 ms.

### Hardware acceptance

- Test mở microphone/output thật được đánh dấu `#[ignore]` và yêu cầu đeo tai nghe trước khi chạy.
- Chưa tuyên bố PASS cho chất lượng nghe hoặc latency thực tế nếu chưa thực hiện manual listening test.

### Kết quả xác minh

- `npm.cmd run test`: thành công, 7/7 frontend tests.
- `npm.cmd run build`, `npm.cmd run lint -- --max-warnings=0`, `npm.cmd run format`: thành công.
- `cargo test`: thành công, 15 tests; 3 hardware tests bị bỏ qua mặc định.
- `cargo check --all-targets`, `cargo fmt -- --check`,
  `cargo clippy --all-targets -- -D warnings`: thành công.
- `npm.cmd run tauri:dev`: thành công ở trạng thái monitoring tắt; cửa sổ phản hồi.
- Manual listening, unplug khi stream đang chạy và loopback latency measurement: chưa kiểm thử vì cần
  tai nghe, thao tác phần cứng và đánh giá trực tiếp của người dùng.

## Task 05 - Volume Control & Audio Meter

### Đã hoàn thành

- Thêm input gain và output monitoring gain độc lập, giới hạn 0–100% tương ứng gain 0.0–1.0.
- Thêm input/output mute và gain smoothing 10 ms để hạn chế click/pop.
- Thêm peak/RMS meter từ audio thật, hiển thị dBFS và clipping indicator.
- Metrics được lấy 25 lần/giây qua status snapshot; callback chỉ cập nhật atomics.
- Lưu/khôi phục gain và mute tại `app_config_dir/volume-settings.json`.
- Slider preview cập nhật audio ngay; chỉ commit mới ghi persistence.
- Không triển khai boost trên 100%, pitch hoặc formant.

### Kết quả xác minh

- `npm.cmd run test`: thành công, 8/8 frontend tests.
- `npm.cmd run build`, `npm.cmd run lint -- --max-warnings=0`, `npm.cmd run format`: thành công.
- `cargo test`: thành công, 23 tests; 3 hardware tests bị bỏ qua mặc định.
- `cargo check --all-targets`, `cargo fmt -- --check`,
  `cargo clippy --all-targets -- -D warnings`: thành công.
- `npm.cmd run tauri:dev`: thành công ở trạng thái monitoring tắt; cửa sổ phản hồi.
- Listening test xác nhận thay đổi gain/mute và meter với microphone thật: chưa thực hiện vì cần tai
  nghe và đánh giá trực tiếp của người dùng.

## Task 06 - Pitch Shifting & Formant Control

### Đã triển khai

- Tích hợp Signalsmith Stretch 0.1.3 (MIT) qua Rust wrapper; không thay sample rate phát để giả lập pitch.
- Pitch −12…+12 semitone, formant −6…+6 semitone độc lập, bước UI 0.1 semitone.
- `PitchShiftProcessor`, `FormantProcessor`, `ProcessorChain` và `EffectParameters` tách khỏi code Windows.
- Parameter smoothing 50 ms và bypass crossfade 10 ms sang dry signal đã căn latency.
- FFT/internal buffer, block 256 frame và delay buffer được cấp phát/warm-up trước khi stream chạy.
- Voice Studio kết nối backend thật; có enable, bypass, reset, giá trị hiện tại và trạng thái live/idle.
- Bass, Treble, Studio gain và Save preset vẫn disabled/Coming Soon; Task 06 không thêm preset.

### Đo và kiểm thử

- Test tín hiệu tự tạo xác nhận pitch −6/−3/0/+3/+6 theo tần số kỳ vọng, output giữ nguyên số frame
  theo block, silence không sinh NaN/Infinity, output được clamp và đổi tham số nhanh không crash.
- Test formant xác nhận output thay đổi khi pitch giữ 0; đây là kiểm tra DSP, chưa thay thế đánh giá nghe.
- Bản release trên máy hiện tại: algorithm/pipeline latency 48.00 ms ở 48 kHz; xử lý 10 giây tín hiệu
  trong 105.04 ms, realtime CPU ratio 0.011. Đây không phải total round-trip latency hay CPU toàn app.
- Preset mặc định của thư viện đã được đo 145.33 ms nên không dùng cho live monitoring.
- Listening test với microphone thật, đánh giá robotic/cartoon artifact và round-trip loopback:
  **NOT TESTED**, cần tai nghe và đánh giá trực tiếp. Vì vậy chưa tuyên bố chất lượng formant hoàn tất.

### Dependency build

- Wrapper dùng MSVC/C++ và Bindgen; Windows cần LLVM/libclang. Máy kiểm thử đã cài LLVM 23.1.4.

### Kết quả lệnh

- `npm.cmd run test`: PASS, 8/8 frontend tests.
- `npm.cmd run build`, lint và format: PASS; TypeScript không có lỗi.
- `cargo test --all-targets`: PASS; hardware/listening tests được giữ ở nhóm manual/ignored.
- `cargo check --all-targets`, rustfmt và Clippy `-D warnings`: PASS.
- `npm.cmd run tauri:dev`: PASS; Vite và executable Tauri khởi động, monitoring mặc định tắt.
- `npm.cmd run tauri:build:windows`: PASS; tạo NSIS và MSI x64.

## Task 07 - Voice Effects & Built-in Presets

### Đã triển khai

- Tám preset built-in: Natural, Deep, High, Masculine, Feminine, Robot, Radio và Cartoon.
- Preset JSON schema version 1 gồm ID, metadata, category, pitch, formant, makeup gain và tagged effect
  list có thể mở rộng.
- Tách `models`, `loader`, `validation`, `registry`; built-in registry không trộn với user preset.
- Apply/reset qua Tauri command sau validation; cập nhật atomic parameters và active ID mà không restart
  CPAL stream.
- Robot dùng ring modulation; Radio dùng band-limit + soft drive; Cartoon dùng pitch/formant + tremolo.
- Voice Effects lấy catalog từ Rust, hỗ trợ search, category filter, details, apply, active highlight và
  Reset to Natural. Không còn catalog giả trong TypeScript.
- Masculine/Feminine được ghi rõ là acoustic shaping, không phải AI voice conversion.

### Kết quả kiểm thử

- `cargo test --all-targets`: PASS, 35 test tự động; 4 hardware/performance test manual/ignored.
- PASS: load đủ 8 preset, duplicate ID, schema/effect lạ, invalid/non-finite range, chuyển cả 8 preset
  trên cùng pipeline, clipping bound và regression pitch/formant/silence.
- `npm.cmd run test`: PASS, 8/8; kiểm tra search, apply command, notification và active state.
- TypeScript/Vite build, ESLint, Prettier, Cargo check, rustfmt và Clippy `-D warnings`: PASS.
- Chất lượng nghe từng preset trên microphone thật: **NOT TESTED**; cần tai nghe và đánh giá trực tiếp.

### Giới hạn

- Masculine/Feminine chỉ là pitch/formant shaping; không thể đảm bảo tự nhiên như mô hình AI voice conversion.
- Radio dùng filter bậc thấp và Robot dùng ring modulation nên chủ ý có màu lo-fi/metallic.
- Favorite persistence, preset editor, user preset loader và import/export không thuộc Task 07.

## Task 08 - Noise Suppression & Audio Protection

### Đã triển khai

- RNNoise thực qua `nnnoiseless` 0.5.2 (BSD-3-Clause), model nhúng, frame 480 mẫu/48 kHz.
- Bật/tắt realtime, suppression dry/wet mix, latency-aligned bypass, noise gate và output limiter -1 dBFS.
- Sanitization NaN/Infinity, smoothing gate/limiter; callback không blocking, I/O, network hoặc allocation.
- Settings lưu tại `app_config_dir/noise-settings.json`; UI backend thật hiển thị protected peak count.
- Adapter bounded giữ sample rate thiết bị khi khác 48 kHz; pipeline vẫn ưu tiên 48 kHz.

### Xác minh

- Unit test tự tạo: validation, silence, rate fallback, invalid samples, limiter clipping và bypass.
- Benchmark release trên máy kiểm thử: DSP latency 58.00 ms; xử lý 10 giây audio trong 149.93 ms,
  realtime CPU ratio 0.015. Đây là latency thuật toán, không phải round-trip latency đo qua loopback.
- `npm.cmd run tauri:dev`: PASS; Vite và cửa sổ executable khởi động. Rust báo warning dọn incremental
  cache bị Access denied trên Windows, không phải compiler warning trong source và không chặn app.
- Fan/keyboard noise, whisper/loud speech, nghe A/B microphone thật: **NOT TESTED** vì cần thiết bị và
  đánh giá trực tiếp; không dùng dữ liệu giả để tuyên bố chất lượng nghe.

## Task 09 - Voice Testing, Recording & Playback

### Đã triển khai

- Start/stop recording, timer, Hear Myself độc lập, original/processed selector, playback/stop,
  timeline, WAV export và xóa file tạm.
- Hai track PCM16 mono dùng cùng sample rate/frame clock; original được delay theo DSP latency.
- Bounded SPSC record/playback queues; overflow tăng counter và cảnh báo recording không hoàn chỉnh.
- File I/O và WAV decoding/encoding chạy trên worker nền; realtime callback không I/O/lock/allocation.
- Playback và monitoring dùng chung output stream, không tạo double audio; mặc định Hear Myself tắt.
- Tất cả xử lý local; không có upload hoặc network path.

### Phạm vi kiểm thử

- Unit: PCM16 WAV round-trip/delete, bounded queue overflow; regression audio/preset/noise tests.
- UI: controls render/disabled đúng khi không có session; navigation và controls cũ không regression.
- Record/playback 5 giây, 30 giây, đổi pitch/noise suppression, unplug device và nghe A/B bằng microphone
  thật: **NOT TESTED** tự động, cần thao tác phần cứng và đánh giá trực tiếp.
- Quality gate: frontend 8/8 PASS; Rust 44 PASS, 4 manual/ignored; TypeScript build, ESLint, Prettier,
  rustfmt, Cargo check và Clippy `-D warnings` PASS; `tauri:dev` PASS.
- Windows release executable và NSIS x64 được tạo. MSI **BLOCKED** tại WiX `light.exe` LGHT0217 vì
  Windows Installer Service không truy cập được trên máy kiểm thử; đây là lỗi môi trường, không phải compile.

## Task 10 - Custom Voice Preset Management

### Đã triển khai

- Create, edit/rename, duplicate, delete có confirmation, save/load, favorite cho custom preset.
- Import/export JSON qua native Tauri dialog; search/category filter tiếp tục áp dụng cho catalog hợp nhất.
- Editor có Pitch, Formant, input/output gain, Bass/Treble EQ thật, noise suppression/gate, limiter và bypass.
- Preview cập nhật audio backend thật; Reset/Save/Cancel/Duplicate và validation error/status đầy đủ.
- Built-in read-only, chỉ duplicate; custom lưu từng file tại `app_data_dir/presets`.
- Schema v2, migration v1, atomic write + backup rollback, 256 KiB import limit, path-safe ID, reject effect/schema/plugin lạ.
- Repository abstraction không phụ thuộc Windows và có thể thay bằng database/cloud implementation sau này.

### Giới hạn

- Favorite của built-in chưa được lưu vì built-in immutable; cần preference overlay riêng nếu muốn favorite built-in.
- Effect order giữ cố định do chain hiện tại chưa có contract reorder realtime-safe; UI không hiển thị control giả.
- Plugin chỉ có metadata/dependency validation, không load hoặc thực thi binary.

### Quality gate

- `cargo test --all-targets`: PASS, 52 tự động; 4 hardware/performance tests ignored/manual.
- CRUD, persistence restart, corruption isolation, path traversal, v1 migration/future schema, import/export,
  duplicate ID, invalid effect/parameter và Bass/Treble DSP đều có test PASS.
- Frontend Vitest 8/8, TypeScript/Vite build, ESLint, Prettier, rustfmt, Cargo check và Clippy `-D warnings`: PASS.
- `npm.cmd run tauri:dev`: PASS; ứng dụng Tauri khởi động với catalog/editor và native import/export dialog.
