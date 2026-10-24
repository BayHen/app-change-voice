# Cài đặt VoiceMorph Virtual Microphone

## Luồng cài đặt

1. Người dùng mở NSIS installer đã ký và xác nhận UAC.
2. Installer chỉ tiếp tục trên Windows 10/11 x64.
3. Installer kiểm tra `VoiceMorphVirtualAudio.sys` đang cài và đọc file version.
4. Nếu version bằng hoặc mới hơn package, driver được giữ nguyên; nếu thiếu/cũ, installer gọi:

   ```powershell
   pnputil.exe /add-driver VoiceMorphVirtualAudio.inf /install
   ```

5. Mã thành công được chấp nhận; mã `3010` đánh dấu cần restart. Mã khác làm installation thất bại với thông báo,
   không tắt Secure Boot và không bỏ qua signature enforcement.
6. Sau khi cài, ứng dụng tự probe driver/bridge nhưng không tự mở microphone hoặc tự route audio.

PnPUtil là công cụ có sẵn của Windows. Không phân phối VB-CABLE, DevCon hoặc executable nâng quyền riêng.

## Kiểm tra sau cài đặt

Mở Device Manager và Windows Sound Settings, xác nhận `VoiceMorph Virtual Microphone`, sau đó dùng trang Virtual
Microphone trong ứng dụng để Check Driver → Connect → Start Routing. Trạng thái Installed không đồng nghĩa với
Streaming.

Log cài driver của Windows nằm tại `%SystemRoot%\INF\setupapi.dev.log`. Không gửi file này ra ngoài nếu chưa kiểm
tra thông tin máy có thể chứa trong log.

## Gỡ ứng dụng và driver

App uninstall mặc định **giữ driver**, nhằm tránh ngắt endpoint đang được ứng dụng khác sử dụng hoặc một bản
VoiceMorph khác. Preset, recording và application data cũng không bị hook driver xóa.

Gỡ driver là thao tác quản trị riêng, chỉ thực hiện sau khi dừng routing và đóng audio clients:

1. Trong Device Manager bật **Show hidden devices**.
2. Gỡ `VoiceMorph Virtual Microphone` và chọn xóa driver package nếu Windows cung cấp lựa chọn.
3. Hoặc xác định chính xác `Published Name` bằng `pnputil /enum-drivers`, xác minh Provider là VoiceMorph Studio,
   rồi chạy elevated `pnputil /delete-driver oemNN.inf /uninstall`.

Không dùng `/force` mặc định. Installer hiện không tự parse output PnPUtil theo ngôn ngữ để tìm `oemNN.inf`, tránh
xóa nhầm package.
