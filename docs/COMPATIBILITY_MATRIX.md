# Compatibility Matrix — VoiceMorph Virtual Microphone

Ngày kiểm tra precondition: **24/10/2026**. Windows registry báo `Windows 10 Pro 25H2`, build `26200`, x64.

| Application        | Version           | Windows Version  | Test Case                            | Status     | Notes                                                |
| ------------------ | ----------------- | ---------------- | ------------------------------------ | ---------- | ---------------------------------------------------- |
| Windows            | 25H2 / 26200      | 25H2 / 26200 x64 | Driver service installed/running     | BLOCKED    | Không có service `VoiceMorphVirtualAudio`            |
| Windows            | 25H2 / 26200      | 25H2 / 26200 x64 | Virtual capture endpoint enumeration | BLOCKED    | Không có PnP device hoặc AudioEndpoint VoiceMorph    |
| VoiceMorph         | 0.1.0             | 25H2 / 26200 x64 | Rust bridge → installed driver       | BLOCKED    | Local CAT/SYS `NotSigned`; driver không cài          |
| Windows Recorder   | N/A               | 25H2 / 26200 x64 | Capture processed PCM                | BLOCKED    | Precondition endpoint chưa đạt                       |
| Discord            | 1.0.9265          | 25H2 / 26200 x64 | Virtual microphone listed            | BLOCKED    | Không có endpoint để Discord enumerate               |
| Discord            | 1.0.9265          | 25H2 / 26200 x64 | Mic Test / Natural Voice             | BLOCKED    | Không chạy audio test khi precondition fail          |
| Discord            | 1.0.9265          | 25H2 / 26200 x64 | Deep/Robot preset switching          | BLOCKED    | Chưa có processed capture path thực                  |
| Discord            | 1.0.9265          | 25H2 / 26200 x64 | Mute and restart                     | BLOCKED    | Chưa có endpoint/streaming                           |
| OBS Studio         | 32.1.0            | 25H2 / 26200 x64 | Audio Input Capture enumeration      | BLOCKED    | Không có endpoint VoiceMorph                         |
| OBS Studio         | 32.1.0            | 25H2 / 26200 x64 | Meter and processed recording        | BLOCKED    | Không ghi từ thiết bị thay thế                       |
| OBS Studio         | 32.1.0            | 25H2 / 26200 x64 | Preset switching / no echo           | BLOCKED    | Processed path chưa sẵn sàng                         |
| VALORANT           | Not installed     | 25H2 / 26200 x64 | Input enumeration / loopback         | BLOCKED    | Game không được phát hiện; endpoint cũng thiếu       |
| VALORANT           | Not installed     | 25H2 / 26200 x64 | PTT, mute, Party/Team Voice          | BLOCKED    | Không thực hiện game test giả lập                    |
| Zoom               | Not installed     | 25H2 / 26200 x64 | Voice chat compatibility             | NOT TESTED | Ứng dụng không được phát hiện; precondition chưa đạt |
| Discord + OBS      | 1.0.9265 / 32.1.0 | 25H2 / 26200 x64 | Concurrent capture                   | BLOCKED    | Không có virtual endpoint                            |
| VALORANT + Discord | N/A / 1.0.9265    | 25H2 / 26200 x64 | Concurrent capture                   | BLOCKED    | VALORANT và endpoint không sẵn sàng                  |
| VoiceMorph         | 0.1.0             | 25H2 / 26200 x64 | Client close does not stop engine    | BLOCKED    | Không có external client stream để test              |
| VoiceMorph         | 0.1.0             | 25H2 / 26200 x64 | Mic disconnect / driver restart      | BLOCKED    | Driver chưa cài                                      |
| VoiceMorph         | 0.1.0             | 25H2 / 26200 x64 | Sleep/resume / app restart           | BLOCKED    | Driver lifecycle chưa khả dụng                       |
| VoiceMorph         | 0.1.0             | 25H2 / 26200 x64 | End-to-end latency                   | NOT TESTED | Không có loopback path; không công bố latency        |
| VoiceMorph         | 0.1.0             | 25H2 / 26200 x64 | CPU/memory/30-minute stability       | BLOCKED    | Workload external-app thật chưa thể chạy             |

Không có compatibility case nào được đánh dấu PASS chỉ từ unit test, endpoint discovery hoặc meter UI.
