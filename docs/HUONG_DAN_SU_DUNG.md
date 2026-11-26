# Hướng dẫn sử dụng VoiceMorph Studio

Áp dụng cho bản Windows 0.1.0 đang phát triển trên `feature/new-version`.
Cập nhật: 26/11/2026. Tài liệu ưu tiên trường hợp **Discord không có tiếng hoặc vẫn nghe giọng gốc**.

## Audio Quality Lab: kiểm tra tiếng méo, rè hoặc ngắt quãng

Vào **Settings → Audio Quality Lab**. Để thử nền sạch, dừng AI, chọn **Use Classic DSP**,
rồi dùng **Reset audio levels / DSP / noise** nếu muốn đưa DSP/gain/noise/profile về cấu hình chuẩn.
Reset sẽ dừng Monitor, đặt gain 100%, tắt noise/gate và bật limiter −1 dBFS; nó không tự đổi AI
thành giọng gốc. Bật Monitor và Start routing lại bằng thao tác của bạn. Nên đeo tai nghe.

- Bắt đầu bằng **Balanced**; nếu bị ngắt, thử **High Quality**. Apply profile dừng âm thanh.
- Low Latency chỉ có lợi khi driver hỗ trợ buffer nhỏ; app có thể nâng prefill để tránh thiếu mẫu.
- Xem **Underruns / overruns**, callback max, input/output peak và RMS khi nói. Nếu input đã
  clipping, giảm gain mic tại Windows/mic trước; limiter cuối không sửa được tín hiệu đã méo ở đầu vào.
- Sau khi nghe rõ Classic DSP, mới thử preset; kiểm tra từng hiệu ứng để tìm cấu hình gây vấn đề.
- Với RTX 3080/model đã thử, bắt đầu AI CUDA chunk 8.192 hoặc 16.384. 4.096 chỉ thử sau warmup
  và theo dõi budget. CPU chưa đủ realtime trong benchmark này; không tự chuyển sang CPU rồi coi là đã ổn.
- Con số latency của panel là **ước tính pipeline**, không phải đo trễ đến Discord.

**Ghi chẩn đoán là tùy chọn**: chọn rõ tầng cần ghi, thời lượng 1–30 giây, tích xác nhận rồi
bấm **Record selected diagnostic stages**. Raw input có thể chứa giọng nói riêng tư. Khi ghi có
badge **Local diagnostic recording**; dùng Stop để dừng sớm. WAV/manifest lưu cục bộ theo đường
dẫn panel hiển thị, không tự upload. Tầng AI/mic ảo chưa chạy sẽ không có WAV. Kiểm tra dropped
samples/lỗi incomplete trước khi dùng bản ghi phân tích. Mở app hoặc vào tab không tự ghi âm.
Sau khi chia sẻ bản ghi với người khác bằng cách của bạn, bạn tự quyết định xóa các file local này.

Báo cáo đo và các giới hạn: [AUDIO_QUALITY_REPORT.md](AUDIO_QUALITY_REPORT.md).

## 1. Cấu hình đúng cho máy của bạn

### Bộ giọng nữ

Vào **Voice Lab → Female collection**. Có 20 preset **Classic DSP** và một model **AI/RVC**
được rà soát quyền dùng giọng; đây không phải 20 model AI riêng biệt.

1. Tìm giọng hoặc lọc AI/Classic DSP. Bấm card để xem, bấm **Use DSP voice** để áp dụng.
   Dấu tim lưu favorite; mục Recent ghi các lựa chọn gần đây. Chỉ xem card không bật mic.
2. Pitch/Formant/Intensity/Warmth/Brightness và Advanced vocal dynamics điều chỉnh DSP thật.
   Chỉnh từng mức nhỏ với giọng của bạn. Deep/Mature khác với Anime/Kawaii cách điệu.
3. Với AI Tsukuyomi-chan: cài runtime nếu thiếu, đọc/xác nhận điều khoản rồi tải model chính thức.
   **File verified** chỉ nói file hợp lệ; bấm **Use AI voice** và đợi model load thành công.
   Bật Monitor/Start routing riêng khi bạn muốn gửi mic sang Discord. **Stop AI** giữ mute;
   chọn một giọng DSP và bấm Use DSP voice để chủ động quay lại Classic.
4. Để nghe thử file, bấm **Choose preview WAV**, chọn file local bạn có quyền dùng, tích xác nhận
   rồi **Play voice preview**. App chỉ đọc tối đa 5 giây, không tự thu mic/upload. Preview dùng
   vocal settings của giọng đang chọn, noise tắt và limiter an toàn; gain live không phải volume
   preview. AI preview cần WAV 48 kHz và AI live đã dừng; lần đầu có thể mất thời gian load.
5. AI không có formant/intensity/de-essing hậu kỳ trong đường hiện tại; các slider đó chỉ dành
   cho DSP. Profile CUDA Balanced 8.192 là điểm bắt đầu trên RTX 3080 đã thử. CPU chưa qualified
   realtime. Đừng dùng tiếng click/ngắt làm lý do giảm buffer thấp hơn khả năng driver.

Vietnamese/Korean/Japanese Style chỉ là preset màu giọng, không đổi accent/ngôn ngữ.
Chưa kiểm chứng tiếng Việt ba miền hoặc độ tự nhiên nam→nữ trên giọng nam thực.
Credit khi dùng AI: **Tsukuyomi-chan official RVC model (CV. Rei Yumesaki)** và nguồn giọng
được chuyển theo điều khoản chủ giọng. [Báo cáo collection](FEMALE_VOICE_COLLECTION_REPORT.md).

Trong lần rà soát này, cấu hình lưu của mic ảo đã bị trỏ vào **Headphones (High Definition Audio Device)**.
Mình đã sửa lựa chọn lưu của VoiceMorph sang **CABLE Input** và dừng phiên kiểm thử.
Tai nghe là đầu ra để nghe, không phải đầu vào của cáp ảo. Đóng bản app cũ, mở bản EXE mới
rồi đối chiếu cả VoiceMorph lẫn Discord theo bảng sau:

| Vị trí                                                                       | Thiết bị cần chọn                                             |
| ---------------------------------------------------------------------------- | ------------------------------------------------------------- |
| VoiceMorph → Voice Lab → Microphone input                                    | **Microphone (HyperX QuadCast)** — mic thật                   |
| VoiceMorph → Voice Lab → Audio output                                        | **Headphones (High Definition Audio Device)** — tai nghe thật |
| VoiceMorph → Settings → Virtual microphone → Virtual cable playback endpoint | **CABLE Input (VB-Audio Virtual Cable)**                      |
| Discord → User Settings → Voice & Video → Input Device                       | **CABLE Output (VB-Audio Virtual Cable)**                     |
| Discord → Output Device                                                      | Tai nghe thật, cùng tai nghe bạn đang dùng                    |

Luồng âm thanh:

```text
HyperX QuadCast → VoiceMorph xử lý giọng → CABLE Input
                                             ↓ VB-CABLE
Tai nghe ← Discord phát âm thanh       CABLE Output → Discord nhận mic
```

**CABLE Input là đầu phát vào cáp; CABLE Output là đầu thu từ cáp.** Tên Input/Output
ở đây được đặt theo phía cáp, nên trông ngược với ô microphone của Discord.
Không trộn cặp thường với cặp `16ch`: `CABLE Input` phải đi với `CABLE Output`;
nếu dùng cặp 16ch thì cả hai đầu phải thuộc cùng cặp đó. Để thử lần đầu, dùng cặp thường.

## 2. Thử nhanh với Discord, theo đúng thứ tự

1. Đóng các bản VoiceMorph cũ để chỉ chạy **một bản** mới. Dùng EXE mới trong
   `src-tauri/target/x86_64-pc-windows-msvc/release/voicemorph-studio.exe`.
   Bộ cài mới nằm tại `src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/VoiceMorph Studio_0.1.0_x64-setup.exe`.
   Đường dẫn `target/release` của các lượt build trước có thể vẫn chứa EXE cũ; đối chiếu
   [Thông tin bản sửa Soundboard mới nhất](SOUNDBOARD_PLAYBACK_FIX.md) để tránh chạy nhầm.
   Nếu chạy source thay vì EXE, dùng `npm.cmd run tauri:dev`.
   Chạy mỗi Vite/web preview sẽ không có backend audio của desktop.
2. Vào **AI Voice Studio → Use Classic DSP** trước để thử đường âm thanh cơ bản.
   Classic không cần model hoặc tải runtime.
3. Ở **Voice Lab**, chọn mic thật và tai nghe theo bảng trên. Kiểm tra nút mute vật lý
   trên HyperX; để input gain ở mức bình thường. Để thử ban đầu, tắt **Noise gate** và
   **Noise suppression**, giữ **Safety limiter** bật.
4. Vào **Settings → Virtual microphone**. Chọn **CABLE Input (VB-Audio Virtual Cable)**.
   **Đổi thiết bị sẽ dừng phiên audio. Đây là hành vi có chủ đích.** Vì vậy chọn cáp trước,
   rồi mới bật Monitor ở bước tiếp theo. Tai nghe/loa thường không dùng làm mic ảo.
5. Bật **Monitor** ở thanh dưới cùng. Trạng thái phải thành **Audio engine running**.
   Nói vào mic và xem **Input level/Microphone signal** ở Voice Lab có thay đổi.
6. Quay lại **Settings → Virtual microphone**, bấm **Start routing**. Trạng thái phải
   là **Routing active**; thanh dưới cùng hiển thị **Virtual mic: live**.
7. Mở Discord → **User Settings → Voice & Video**. Chọn chính xác **CABLE Output** làm
   Input Device, tai nghe thật làm Output Device. Tránh để `Default` trong lúc chẩn đoán.
8. Bỏ mute/deafen trong Discord. Chọn **Voice Activity** để thử; nếu dùng Push-to-Talk,
   phải giữ đúng phím. Chạy **Mic Test / Let's Check** và nói một câu.
9. Khi đã có tiếng, ở Voice Lab chọn **Robot** hoặc **Deep Voice** để nghe sự khác biệt.
   Kiểm tra **Apply voice effects** đang bật. Sau đó có thể bật lại noise suppression
   và điều chỉnh vừa đủ.

**Hai thao tác khác nhau:** bật **Monitor** chỉ khởi động phiên xử lý audio;
bấm **Start routing** mới gửi tiếng sang cáp ảo. Chỉ làm một thao tác sẽ chưa đủ cho Discord.
`Routing active` xác nhận đầu phát cáp đã mở, không tự chứng minh Discord đã nhận tiếng;
Mic Test của Discord mới xác nhận nửa cuối của đường truyền.

## 3. Vì sao bật Monitor nhưng không nghe tiếng mình?

Trong bản hiện tại, **Monitor** ở thanh dưới cùng là công tắc chạy/dừng audio engine.
Việc nghe giọng mình ở tai nghe có công tắc riêng:

1. Bật Monitor.
2. Vào **Recordings → Hear Myself** và bật nếu muốn nghe giọng đã xử lý.
3. Dùng tai nghe; tắt Hear Myself khi đã thử xong nếu tiếng trễ gây khó chịu.

**Hear Myself tắt không ngăn tiếng đi sang mic ảo.** Bạn vẫn phải bật Start routing.
Tiếng nghe trực tiếp từ cổng tai nghe trên HyperX hoặc tính năng sidetone của phần cứng
có thể là giọng gốc, không phải tín hiệu đã qua VoiceMorph. Dùng bản ghi Processed hoặc
Discord Mic Test để đối chiếu, thay vì chỉ dựa vào sidetone.

## 4. Các trang của ứng dụng

### Voice Lab — đổi giọng bằng DSP

- Chọn preset như Deep Voice, Robot, Female Lift, Stream Clean hoặc Monster.
- **Pitch** thay đổi độ cao; **Formant** thay đổi màu giọng. Thử mức nhỏ trước.
- **Bass/Treble** chỉnh EQ; **Makeup gain** bù mức âm lượng. Tránh đẩy gain gây clipping.
- **Apply voice effects** bật/bỏ qua hiệu ứng. Giọng Natural hoặc preset ít xử lý có thể
  khác rất nhẹ; dùng Robot khi cần kiểm tra nhanh đường tín hiệu.
- **Save preset/New preset/Export** lưu hoặc xuất cấu hình preset, không xuất âm thanh.
- Microphone signal chỉ là tín hiệu trong ứng dụng; không xác nhận Discord nhận được.

### Soundboard — phát âm thanh

Khi engine idle, Play/Stop bị khóa. Bấm **Start audio engine** trên trang Soundboard hoặc bật
**Monitor** ở thanh dưới để mở mic/output đã chọn, rồi bấm Play. Thao tác này có mở microphone;
app không tự bật mic khi chỉ mở trang hay bấm Play. Nếu chưa chọn thiết bị, vào Voice Lab trước.
Lỗi backend được hiển thị bằng nội dung cụ thể, không phải `[object Object]`.

1. Import file **WAV** hợp lệ.
2. Chọn đường phát cho từng âm: **Headphones**, **Virtual microphone** hoặc **Both**.
3. Bật Monitor trước khi phát. Muốn Discord nghe thì chọn Virtual microphone/Both
   và bật Start routing.
4. Nếu thêm/sửa thư viện nhưng phiên audio đang dùng bản cũ, tắt rồi bật Monitor
   để nạp lại thư viện; sau đó bật lại routing.

Âm chỉ phát ra Headphones sẽ không tự xuất hiện trong mic Discord.

### AI Voice Studio — chuyển giọng bằng model

Hãy kiểm tra Discord chạy được với Classic trước khi thêm AI vào đường truyền.

1. Chọn **Install recommended runtime** nếu chưa cài. Cần khoảng 3,90 GiB tải xuống
   và ít nhất 13 GiB dung lượng trống. Chờ cài/xác minh hoàn tất.
2. Chọn **Detect hardware**; máy RTX 3080 có thể dùng CUDA.
3. Chọn model hợp lệ. Với **Get official Tsukuyomi-chan voice**, đọc điều khoản của
   chủ giọng và xác nhận quyền dùng trước khi tải/chuyển đổi. Đây là model có điều khoản
   riêng, không phải giấy phép MIT cho giọng nói.
4. Chọn voice, chọn compute **Auto/CUDA**, bấm **Apply AI settings** nếu đã sửa,
   rồi **Load & start AI**. Chờ trạng thái ready. Việc này không tự bật microphone.
5. Bật Monitor, rồi Start routing nếu phiên/cáp chưa chạy. Nói và kiểm tra Discord.

**Stop AI, keep voice muted** sẽ giữ tiếng giọng AI bị mute; nó không tự đổi về giọng gốc.
Muốn dùng lại preset DSP, bấm **Use Classic DSP**. CPU fallback phải bật rõ ràng nếu muốn dùng;
CPU đã chuyển đổi được nhưng chưa bảo đảm thời gian thực ổn định trên máy thử.
Với lần thử đầu, ưu tiên CUDA và thiết lập mặc định; không giảm chunk liên tục chỉ để
đuổi một con số latency. Thời gian xử lý chunk không phải độ trễ trọn đường microphone → Discord.

AI inference chạy local sau khi có đủ tài nguyên. Bộ tương thích hiện dùng Python 3.10
hết hỗ trợ; bản AI vẫn cần thêm kiểm định trước phát hành rộng rãi. Xem báo cáo runtime
trong thư mục docs nếu cần các phiên bản và giấy phép chi tiết.

### Recordings — nghe thử và xuất WAV

1. Bật Monitor rồi bấm **Record/Start recording**.
2. Nói câu ngắn, bấm Stop.
3. Chọn **Original** hoặc **Processed** để nghe đối chiếu trong phần playback.
4. Export WAV để lưu âm thanh. Chọn đúng nguồn cần xuất.

Bản hiện tại quản lý phiên/bản ghi hiện hành, không nên coi trang này là kho lịch sử
nhiều bản ghi đã lưu. Hãy xuất file cần giữ trước khi xóa hoặc bắt đầu công việc mới.
Không dùng ghi âm/giọng của người khác khi chưa có quyền.

### Settings — thiết bị và chẩn đoán

- **Audio devices:** làm mới/chọn mic thật và tai nghe. Đổi thiết bị sẽ dừng audio;
  bật lại Monitor và routing sau đó.
- **Virtual microphone:** chọn CABLE Input, Start/Stop routing, xem peak và trạng thái.
- **Performance:** xem chẩn đoán/counters. Bộ đếm underrun tăng lúc tải nặng có thể đi
  cùng tiếng ngắt; nó không đồng nghĩa mọi stream đã chết.
- **AI runtime/Hardware acceleration:** quản lý runtime và xem phần cứng thực tế.
- **Keyboard shortcuts:** kiểm tra phím tắt, nhất là Ctrl+Alt+M nếu ứng dụng khác đã chiếm.
  Có thể dùng các nút trên giao diện khi phím tắt xung đột.
- Một số mục như giao diện sáng/cloud/plugin chưa có triển khai đầy đủ; mục disabled
  hoặc báo deferred không phải thao tác đã hoàn tất.

## 5. Khoanh vùng lỗi Discord

| Hiện tượng                                          | Kiểm tra và cách xử lý                                                                                                                                                                                |
| --------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Monitor bật nhưng Input level không đổi             | Kiểm tra HyperX có mute không, chọn mic thật, input mute/gain và quyền microphone cho ứng dụng desktop của Windows.                                                                                   |
| Voice Lab có tín hiệu, virtual mic vẫn idle         | Chọn CABLE Input trong Settings → Virtual microphone; bật lại Monitor sau khi đổi cáp; bấm Start routing.                                                                                             |
| Đã chọn cáp nhưng Start routing bị mờ               | Phiên audio đang dừng hoặc chọn sai đầu ra. Bật Monitor và chọn đúng thiết bị cáp; tai nghe/loa thường không hợp lệ.                                                                                  |
| Routing active nhưng meter cáp không lên khi nói    | Kiểm tra input mute/gain, noise gate, model AI chưa ready hoặc AI đã Stop nhưng vẫn giữ mute. Thử Use Classic DSP, tắt gate và chọn Robot.                                                            |
| Meter cáp lên nhưng Discord không nhận              | Discord Input phải là đúng CABLE Output cùng cặp; kiểm tra mute, Voice Activity/Push-to-Talk và Input Sensitivity; chạy Mic Test.                                                                     |
| Discord vẫn giọng gốc                               | Discord có thể vẫn chọn HyperX hoặc nguồn mic khác. Chọn CABLE Output rõ ràng; bật Apply voice effects; phân biệt sidetone phần cứng với Processed.                                                   |
| Soundboard nghe ở tai nghe nhưng Discord không nghe | Đổi route của âm sang Virtual microphone/Both, khởi động lại Monitor nếu thư viện đổi, rồi Start routing.                                                                                             |
| Tiếng nhỏ hoặc bị cắt từ                            | Kiểm tra gain và gate trước; trong Discord thử tạm tắt Noise Suppression/Krisp và Automatic Gain Control để đối chiếu, rồi bật lại từng mục phù hợp.                                                  |
| Nghe lặp hoặc tiếng hú                              | Dùng tai nghe, tắt bớt một nguồn nghe thử. Không chọn CABLE Output làm mic của VoiceMorph khi app đang phát ngược vào CABLE Input. Không bật Windows Listen to this device để phát ngược về cùng cáp. |
| Đổi thiết bị xong mất tiếng                         | Đây là thao tác dừng phiên. Chọn xong thì bật lại Monitor và Start routing.                                                                                                                           |
| Lỗi driver/endpoint lost                            | Dừng routing/Monitor, làm mới thiết bị, chọn lại cáp rồi khởi động lại phiên. Giữ nguyên nội dung lỗi input/output để chẩn đoán; không cần tắt Defender hay đổi chính sách driver.                    |
| Tiếng rè/ngắt hoặc độ trễ cao                       | Đóng bớt tác vụ nặng, thử Classic trước; dùng mức gain vừa phải, kiểm tra sample rate các đầu cáp. AI nên ưu tiên CUDA. Đừng chạy hai bản VoiceMorph cùng lúc.                                        |

Trong Windows, nếu cần đồng bộ định dạng, kiểm tra cả hai đầu VB-Cable và mic/tai nghe
ở 48 kHz khi chúng hỗ trợ. Không tự đổi thiết bị mặc định của toàn hệ thống sang cáp
trong lúc chẩn đoán; chọn thiết bị cụ thể trong từng ứng dụng sẽ dễ xác định đường truyền hơn.

## 6. Dừng an toàn và lần mở tiếp theo

1. Stop recording nếu đang ghi và xuất bản cần giữ.
2. Bấm **Stop routing**, sau đó tắt Monitor.
3. Nếu đã dùng AI, Stop AI hoặc chọn Classic theo nhu cầu rồi đóng app.

Cấu hình thiết bị được lưu, nhưng lựa chọn thiết bị không đồng nghĩa audio/routing đã
tự chạy. Ở lần dùng tiếp theo, kiểm tra lại Monitor và Virtual mic: live trước khi vào call.

## 7. Đã kiểm tra gì, còn cần bạn xác nhận gì?

Rà soát 26/11/2026 đã tìm thấy đầu ra mic ảo lưu nhầm vào tai nghe. Bản sửa chặn chọn
loa/tai nghe làm mic ảo, nhắc bật Monitor sau khi đổi cáp và dùng chung trạng thái audio
giữa Settings với thanh dưới cùng. Một phép thử local đã gửi tone tổng hợp qua soundboard
của VoiceMorph → CABLE Input → thu lại ở CABLE Output; mic thật và tai nghe đều mute.
Tín hiệu đã tới đầu thu của cáp. Không ghi PCM microphone hoặc gửi âm thanh qua cuộc gọi.

Phép thử này chưa xác nhận Discord Mic Test/cuộc gọi trên máy bạn. Sau khi làm mục 2,
nếu vẫn lỗi, hãy cung cấp: tên mic thật; tên CABLE Input/CABLE Output được chọn; trạng thái
Monitor/routing; input meter và cable meter có lên khi nói không; nội dung lỗi driver;
và kết quả Discord Mic Test. Không cần gửi giọng thu hoặc dữ liệu cá nhân để kiểm tra cấu hình.

[Ảnh cấu hình mic ảo đã kiểm tra trong ứng dụng native](screenshots/virtual-mic-review/routing-active.png).
Ảnh được chụp khi mic đang mute trong phép thử nên meter bằng 0; khi bạn bỏ mute và nói,
meter mới cần thay đổi. Gain/mute ban đầu đã được khôi phục sau kiểm thử.

## Tài liệu chính thức đối chiếu

- [Sơ đồ và hoạt động VB-CABLE — VB-Audio](https://vb-audio.com/Cable/VBCABLE_ReferenceManual.pdf)
- [Mic Testing — Discord](https://support.discord.com/hc/en-us/articles/360020641332-Mic-Testing)
- [Voice and Video Troubleshooting — Discord](https://support.discord.com/hc/en-us/articles/360045138471-Discord-Voice-and-Video-Troubleshooting-Guide)
- [Voice Input Modes — Discord](https://support.discord.com/hc/en-us/articles/211376518-Voice-Input-Modes-101-Push-to-Talk-Voice-Activated)
