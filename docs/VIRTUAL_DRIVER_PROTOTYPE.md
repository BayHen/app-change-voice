# VoiceMorph Virtual Microphone — Prototype 11.2.2

## Trạng thái

Phase 11.2.2 cung cấp source overlay và package metadata cho một WaveRT capture endpoint x64. Prototype hiện chỉ
phát silence đã khởi tạo an toàn. Nó **chưa nhận PCM từ VoiceMorph Studio** và chưa được cài trên Windows test
target. Build Debug x64 bằng WDK 10.0.28000.2526 đã PASS ngày 2026-10-24.

## Nền tảng Microsoft được sử dụng

Prototype dùng `audio/simpleaudiosample` từ repository `microsoft/Windows-driver-samples`, commit
`bb68ed5e770bce04dc5395ec9258dafab034184d`. Sample này nhỏ hơn SysVAD đầy đủ nhưng vẫn sử dụng
WDM/PortCls/WaveRT và có capture miniport phù hợp để thu hẹp thành một microphone ảo.

Source upstream không được copy cố định vào repository. `build-driver.ps1` thực hiện:

1. Xác minh revision upstream đã ghim.
2. Xác minh WDK kernel headers có mặt.
3. Copy sample vào `obj/simpleaudiosample`.
4. Copy source VoiceMorph và áp dụng overlay.
5. Thay INF mẫu bằng INF VoiceMorph.
6. Build x64, tắt signing tự động và thu thập `.sys`, `.inf`, `.cat` nếu được tạo.

Mã Microsoft giữ nguyên Microsoft Public License. Tên Microsoft không được dùng để chứng thực sản phẩm.

## Capture endpoint prototype

| Thuộc tính     | Giá trị                            |
| -------------- | ---------------------------------- |
| Device name    | `VoiceMorph Virtual Microphone`    |
| Provider       | `VoiceMorph Studio`                |
| Hardware ID    | `ROOT\VoiceMorphVirtualMicrophone` |
| Service        | `VoiceMorphVirtualAudio`           |
| Architecture   | x64                                |
| Driver version | `0.1.0.0`                          |
| Endpoint       | capture-only                       |
| Format         | 48 kHz, mono, signed PCM 32-bit    |
| Khi thiếu PCM  | silence                            |

Overlay đặt số render endpoint bằng 0, đổi format capture và thay tone generator trong đường capture bằng
`VoiceMorphFillSilence`. Hàm này kiểm tra con trỏ/kích thước rồi zero toàn bộ vùng được driver cung cấp; không đọc
bộ nhớ chưa khởi tạo và không cấp phát bộ nhớ.

## Ranh giới với Phase 11.2.3

`voicemorph_bridge_protocol.h` mới định nghĩa version/format/header contract. Prototype chưa tạo shared memory,
IOCTL, ring buffer, synchronization hay user-mode client. Vì vậy endpoint, nếu build và cài hợp lệ, chỉ có thể
trả silence. Đây là hành vi chủ ý, không phải bằng chứng đã routing giọng thật.

DSP vẫn nằm trong Rust AudioEngine. Kernel driver về sau chỉ nhận PCM đã xử lý, quản lý clock/position/underrun và
cấp dữ liệu cho WaveRT capture client. Không đưa pitch, formant, noise suppression hoặc preset vào kernel.

## Build và validation

Kiểm tra contract không cần WDK:

```powershell
powershell -ExecutionPolicy Bypass -File `
  drivers/voicemorph-virtual-audio/tests/validate-package.ps1
```

Kiểm tra contract chỉ xác minh nhất quán manifest/INF/overlay. Build hiện đã chạy compilation, ApiValidator,
Inf2Cat và InfVerif `/w`; nó vẫn **không thay thế** driver signing, installation hoặc HLK.

Không cài prototype lên máy làm việc chính. Test startup, enumeration, stream start/stop và uninstall phải diễn ra
trên máy/VM thử nghiệm có snapshot/recovery, package được ký theo chính sách Windows và không bypass Secure Boot.

## Hạn chế đã biết

- Chưa có bridge PCM nên không truyền voice thật.
- Các source render của sample vẫn có thể được compile thành library, nhưng render endpoint không được đăng ký.
- Chưa có kiểm chứng clock accuracy, multi-client, power transitions, recovery hoặc long-session stability.
- `.sys`, generated `.inf` và `.cat` đã được tạo tại `drivers/voicemorph-virtual-audio/artifacts/Debug/x64`.
- `.sys` và `.cat` chưa ký; package chưa thể dùng để tuyên bố driver load hợp lệ trên Windows production.
- Catalog được tạo không đồng nghĩa catalog đã được ký hoặc được Windows tin cậy.
