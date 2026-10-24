# Quy trình kiểm thử tương thích audio ngoài ứng dụng

## Precondition gate

Chỉ bắt đầu compatibility test khi tất cả điều kiện sau PASS:

- Package SYS/INF/CAT là production package Microsoft-signed.
- Service `VoiceMorphVirtualAudio` chạy bình thường và Device Manager không báo lỗi.
- Windows có capture endpoint `VoiceMorph Virtual Microphone`.
- VoiceMorph status đạt BridgeConnected rồi Streaming.
- Windows recorder độc lập thu được PCM processed thật; silence fallback được phân biệt với mất tín hiệu.
- Không chọn hoặc route qua VB-CABLE.

Nếu một điều kiện thất bại, các test Discord/VALORANT/OBS phải là BLOCKED, không phải FAIL của ứng dụng đích.

## Evidence bắt buộc

Mỗi lần chạy ghi: ngày giờ, Windows build, application/file version, input/output device, sample rate, preset,
noise setting, gain/mute, thời lượng, headphone/interface và file log. Không đưa raw recording vào repository.

PASS cần bằng chứng audio đầu thu: Discord Mic Test nghe được, OBS recording nghe được, hoặc VALORANT loopback/voice
chat nghe được. Việc endpoint xuất hiện hoặc meter chuyển động một mình không đủ.

## Trình tự

1. Windows recorder sanity: Natural, Deep, Robot, mute, preset switching.
2. Discord riêng; restart Discord khi routing vẫn active.
3. OBS riêng; recording 30 giây và kiểm tra duplicate/echo.
4. VALORANT trong môi trường thử phù hợp; loopback trước Party/Team Voice.
5. Discord + OBS, sau đó VALORANT + Discord nếu endpoint cho phép multi-client.
6. Failure tests: microphone unplug, VoiceMorph restart, client restart, driver unavailable và sleep/resume.
7. Long run tối thiểu 30 phút; lấy metrics đầu/cuối và memory working set định kỳ.

## Đo hiệu năng

- End-to-end latency: dùng loopback vật lý hoặc timestamped acoustic/electrical capture; không dùng queue estimate.
- CPU/memory: đo process VoiceMorph trong cùng workload và khoảng thời gian xác định.
- Dropped/underrun/overflow: lấy counters trước và sau test.
- Ghi rõ buffer size/sample rate. Không công bố `<50 ms` nếu chưa có loopback measurement.

## Kết quả precondition ngày 24/10/2026

Máy `admin\admin`, Windows registry ProductName `Windows 10 Pro`, DisplayVersion `25H2`, build `26200`, x64.
Không có service, PnP device, AudioEndpoint hoặc Driver Store entry VoiceMorph. Release CAT/SYS local là `NotSigned`.
Discord `1.0.9265` và OBS `32.1.0` được phát hiện; VALORANT và Zoom không được phát hiện. Compatibility run bị
BLOCKED trước khi mở/capture microphone.
