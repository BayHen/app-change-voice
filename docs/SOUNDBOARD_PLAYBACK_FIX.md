# Soundboard: engine idle và thông báo [object Object]

Ngày 26/11/2026, nhánh `feature/new-version`.

Ảnh lỗi cho thấy audio engine đang idle. Soundboard phát qua mixer của audio engine,
nhưng UI vẫn cho bấm Play. Backend trả object `{code, message}`; `String(error)` làm
mất nội dung và hiển thị `[object Object]`. Backend còn tra clip trong active session trước
khi kiểm tra engine đã chạy, nên có thể báo nhầm cần restart library khi thực tế chưa start.

Bản sửa:

- Dùng nội dung `message` của lỗi Tauri trong Soundboard, có fallback an toàn cho lỗi không rõ dạng.
- Chặn Play/Stop khi audio engine dừng; hiển thị hướng dẫn và nút **Start audio engine**.
  Nút này mở microphone/output đã chọn bằng thao tác rõ ràng của người dùng; Play không tự bật mic.
- Backend kiểm tra trạng thái engine trước khi tra clip. Stop khi đã dừng là thao tác idempotent.
- Giữ nguyên WAV, thư viện, DSP, AI và routing. Clip gửi vào virtual microphone vẫn cần bật routing.

Nếu chưa phát được: chọn microphone/output trong Voice Lab, bấm Start audio engine (hoặc Monitor),
rồi Play. Nếu nhập/chỉnh thư viện khi engine đang chạy và nhận thông báo restart,
tắt/bật Monitor để mixer tải lại thư viện. Chọn Headphones/Both nếu muốn nghe tại máy;
chọn Virtual microphone/Both và Start routing nếu muốn gửi sang Discord.

Kiểm tra: test frontend chặn phát khi idle và chỉ start sau thao tác người dùng; test formatter
giữ lỗi có cấu trúc; test Rust xác nhận lỗi idle đúng thứ tự và Stop không làm mở stream.
Native harness: `scripts/verify-soundboard-native.mjs`, dùng clip có sẵn với microphone và output
mute, không ghi âm hay sửa/xóa WAV/thư viện. Bằng chứng native tại `docs/screenshots/soundboard-fix`.

Build: `npm.cmd run tauri:build:windows -- --bundles nsis`.
EXE/bộ cài mới ghi tại `src-tauri/target/x86_64-pc-windows-msvc/release`.
Kiểm chứng cuối: **116 Rust tests passed**, **27 frontend tests passed**, ESLint,
TypeScript/Vite, rustfmt và build optimized EXE/NSIS thành công.
Native đã dùng clip có sẵn `mixkit-air-woosh-1489`, xác nhận Start → Play → Stop thành công;
chạy với mic/output mute, không bật routing, không ghi âm và khôi phục volume sau test.
Không coi phép thử mute là đánh giá nghe chất lượng clip.

- [Native report](screenshots/soundboard-fix/report.json)
- [Ảnh engine dừng](screenshots/soundboard-fix/engine-stopped.png)
- [Ảnh engine chạy](screenshots/soundboard-fix/engine-running.png)

EXE: `src-tauri/target/x86_64-pc-windows-msvc/release/voicemorph-studio.exe`.
SHA-256: `b2d212cdef29117773209e908bfd5e591dc4ca477099de3ed072434e087a7b96`.

Bộ cài: `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/VoiceMorph Studio_0.1.0_x64-setup.exe`.
SHA-256: `7bcd598fe73b07053554af22c25f1cd72707afde2ee098974525605cf7c64c8d`.

Đây là checkpoint sau Female Collection; không dùng hash bộ cài cũ để đối chiếu file mới.
Code vẫn chưa commit trên `feature/new-version`.
