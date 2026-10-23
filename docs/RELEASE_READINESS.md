# Release Readiness

## Kết luận

**NO-GO cho production.** Build/test tự động ổn định, nhưng chưa đủ bằng chứng cho end-to-end latency, session dài, ứng dụng đích và installer. Có thể dùng như development/internal QA build.

| Gate                      | Trạng thái       | Bằng chứng                                           |
| ------------------------- | ---------------- | ---------------------------------------------------- |
| React/Vitest              | PASS             | 9/9                                                  |
| Rust automated            | PASS             | 72/72; 7 ignored chạy riêng                          |
| Hardware integration      | PASS             | 6/6 Windows/VB-CABLE; microphone cần ngoài sandbox   |
| DSP benchmark             | PASS có cảnh báo | 10 giây/143.34 ms; latency 58.00 ms                  |
| TypeScript/frontend build | PASS             | `tsc --noEmit` + Vite                                |
| Rust compile/lint         | PASS             | Cargo test; Clippy `-D warnings`                     |
| npm advisory              | PASS             | 0 vulnerability/309 dependency                       |
| RustSec                   | PASS có cảnh báo | 0 vulnerability, 4 warning                           |
| End-to-end loopback       | NOT TESTED       | Thiếu fixture                                        |
| Long-session/memory leak  | NOT TESTED       | Chưa chạy 30 phút/2 giờ                              |
| Discord/OBS/VALORANT      | NOT TESTED       | Cần app thật                                         |
| Release EXE + NSIS        | PASS             | EXE 6,760,448 bytes; NSIS 2,246,299 bytes            |
| MSI installer             | BLOCKED          | WiX LGHT0217/LGHT0216, Windows Installer unavailable |

## Security review

- Tauri capability chỉ có `core:default`, dialog open/save; frontend không có shell permission.
- Import preset/backup có validation, giới hạn dung lượng và atomic/safety-backup; path traversal có regression test.
- Plugin chỉ built-in/trusted declarative effect, không load DLL tùy ý.
- Diagnostics không chứa audio, recording, token hoặc secret.
- Cloud sync deferred/local-only nên không có credential cloud.
- Update signing chưa có (RR-006).

## Điều kiện GO

1. Xử lý RR-001 và đo loopback end-to-end.
2. Chạy đủ 30 phút và 2 giờ, không memory growth/leak nghiêm trọng hay counter bất thường.
3. PASS Discord + OBS; VALORANT nếu nằm trong release claim.
4. PASS disconnect/reconnect, sleep/resume và các buffer driver hỗ trợ.
5. Tạo/cài installer trên máy sạch và quyết định signing/update policy.
