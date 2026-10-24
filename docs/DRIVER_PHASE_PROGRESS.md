# Phase 11.2.1 Progress

## PASS

- Review AudioEngine, pipeline/processors, CPAL/WASAPI streams, effects, recording, existing virtual routing,
  Tauri commands, installer/workflows và tests.
- Xác định seam: processed fan-out hiện tại -> bounded audio bridge -> WaveRT capture endpoint; không duplicate DSP.
- Review SysVAD commit `bb68ed5e770bce04dc5395ec9258dafab034184d` và phân loại phần reuse/replace.
- Thêm project scaffolding `drivers/voicemorph-virtual-audio` và Rust contracts `virtual_audio`.
- Rust/frontend build và regression tests không bị thay đổi logic audio runtime.

## BLOCKED

- Build SysVAD/driver: WDK kernel headers/libs, WDF, Inf2Cat và StampInf chưa có.
- ATL/Spectre ATL chưa được phát hiện; cần xác nhận/cài component đúng toolset.
- Driver package/signing/installation: chưa có implementation, INF, catalog, certificate hoặc test target.

## NOT TESTED

- Kernel build, Static Driver Verifier, Driver Verifier, HLK audio tests.
- PCM transfer user mode -> driver, WaveRT capture, multi-client behavior, device install/uninstall.
- Sleep/resume, crash recovery, clock drift, long-session latency/underrun.
- Discord/OBS/game capture qua VoiceMorph driver riêng.

## Phase 11.2.2 entry criteria

1. Matching SDK/WDK pair và required ATL components được xác minh.
2. Upstream SysVAD x64 Debug build PASS nguyên bản.
3. Có test machine/VM, recovery path và signing plan.
4. Chốt transport threat model, protocol ABI và performance budget.
5. Chốt product GUID/hardware ID/provider naming; không dùng sample identity.

Phase 11.2.1 không cài driver, không đổi security settings và không tuyên bố có virtual microphone hoạt động.

## Phase 11.2.2 — Prototype

### PASS

- Chọn Microsoft Simple Audio Sample/WaveRT tại commit đã ghim làm upstream tối giản và giữ MS-PL notices.
- Tạo overlay capture-only: vô hiệu hóa đăng ký render endpoint, format 48 kHz mono PCM32.
- Thay đường tone generator bằng `VoiceMorphFillSilence`; không cấp phát và không đọc dữ liệu chưa khởi tạo.
- Tạo INF/manifest với identity ổn định `VoiceMorph Virtual Microphone`, provider `VoiceMorph Studio`, x64,
  version `0.1.0.0` và hardware ID `ROOT\VoiceMorphVirtualMicrophone`.
- Tạo build script không install/sign và package contract validation.

### BLOCKED

- Installation/startup/enumeration: package đã build nhưng chưa ký và không được phép cài lên máy làm việc chính.

### BUILD VERIFIED — 2026-10-24

- WDK/SDK `10.0.28000.2526` và Visual Studio WDK Build Tools integration đã cài.
- Debug x64 build PASS: 0 warnings, 0 errors.
- ApiValidator PASS; Inf2Cat PASS với 0 errors/0 warnings; InfVerif `/w` và `/info` PASS.
- Artifacts: unsigned `.sys`, stamped `.inf` và generated unsigned `.cat`.

### NOT TESTED

- WaveRT stream start/stop, endpoint enumeration, silence capture qua Windows audio client.
- Reinstall/uninstall, sleep/resume, device recovery, multi-client và Driver Verifier/HLK.
- PCM routing thật; phần này cố ý để lại cho Phase 11.2.3.

Prototype đã sẵn sàng cho bước ký/test trên test environment. Phase 11.2.2 chưa đạt nghiệm thu toàn bộ cho đến
khi kiểm thử installation, endpoint enumeration và stream lifecycle đều PASS.

## Phase 11.2.3 — Rust-to-Driver Audio Bridge

### PASS

- Nối đúng processed fan-out sau DSP/gain/limiter; monitoring và recording tiếp tục dùng chung pipeline.
- Thêm SPSC queue không blocking trong callback, worker Rust riêng và chuyển Float32 sang PCM32 có sanitize/clamp.
- Thêm device interface + buffered IOCTL protocol v1, format negotiation, single-producer ownership, bounded kernel ring, counters và cleanup.
- WaveRT capture đọc PCM từ ring và zero-fill an toàn khi underrun.
- Driver Debug x64 build PASS, 0 warnings/0 errors; ApiValidator và Inf2Cat PASS.

### NOT TESTED / BLOCKED

- Driver chưa ký/cài trên test target: chưa xác minh Windows client thu PCM, pitch/noise qua endpoint, reconnect và app restart.
- Chưa chạy long-session, Driver Verifier/HLK hoặc đo loopback end-to-end; không có số latency công bố.
- Nghiệm thu đầy đủ Phase 11.2.3 bị BLOCKED cho đến khi package được ký hợp lệ và test trên máy/VM driver riêng.

## Phase 11.2.4 — Tauri Virtual Microphone Integration

### PASS

- Thêm `VirtualMicrophoneManager`, driver/version probe, bridge connection lifecycle và typed Tauri commands.
- Driver status trả version `0.1.0.0` qua protocol; app reject version/protocol không tương thích.
- Startup probe không mở CPAL stream, không capture microphone, không yêu cầu admin và không đổi default device.
- UI Virtual Microphone có driver/endpoint/bridge/routing states, meters, queue metrics, latency estimate có nhãn,
  Check/Connect/Disconnect/Start/Stop/Test và Windows Sound Settings.
- Driver disconnect được báo lỗi có cấu trúc; routing có thể stop/recheck mà không dừng monitoring output thông thường.

### NOT TESTED / BLOCKED

- Driver unsigned chưa được cài: detection/connect/streaming với kernel interface thật vẫn NOT TESTED.
- Test Microphone hiện mở Windows Sound Settings để người dùng xác minh endpoint; app không tuyên bố client nhận audio.
- App restart, unexpected device removal và capture processed voice cần test target có package ký hợp lệ.
- `npm.cmd run tauri:dev`: PASS khi chạy ngoài sandbox trong phiên desktop Windows tương tác; Vite và executable Tauri khởi động, cửa sổ duy trì hoạt động cho tới khi chủ động dừng. Lỗi WebView2 `0x800700AA` chỉ tái hiện dưới tài khoản automation cô lập `codexsandboxoffline`, không phải lỗi cấu hình ứng dụng.

## Phase 11.2.5 — Windows Installer & Driver Packaging

- Chọn Tauri 2 NSIS `perMachine`; product identifier giữ nguyên. Hook chỉ gọi PnPUtil bằng command cố định.
- Thêm signed-driver staging, kernel signature validation, SHA-256 release manifest và fail-closed release workflow.
- Driver Release x64 build PASS, Inf2Cat PASS, 0 warnings/errors; StampInf đã đồng bộ `0.1.0.0`.
- Package local hiện unsigned và được staging từ chối đúng thiết kế. Không nhúng debug/test-signed binary.
- Production installer, clean install, upgrade/repair/uninstall và endpoint verification: BLOCKED đến khi nhận package Microsoft-signed và test trên Windows target riêng.

## Phase 11.2.6 — External Application Compatibility Testing

- Precondition gate chạy trong phiên desktop `admin\admin`: Windows build `26200` x64; không có service, PnP device, AudioEndpoint hoặc Driver Store entry VoiceMorph.
- Release CAT/SYS local là `NotSigned`; không cài prototype và không dùng VB-CABLE thay thế.
- Phát hiện Discord `1.0.9265` và OBS `32.1.0`; không phát hiện VALORANT/Riot Client hoặc Zoom.
- Discord/OBS/VALORANT, multi-app, failure recovery, long-run và end-to-end latency: BLOCKED/NOT TESTED trước audio capture do precondition không đạt.
- Tạo compatibility matrix và manual evidence procedure; không có compatibility PASS giả từ endpoint list hoặc unit test.
