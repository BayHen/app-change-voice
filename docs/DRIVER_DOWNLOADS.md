# Microsoft SDK/WDK Downloads cho Phase 11.2.1–11.2.2

Ngày xác minh: 2026-10-23.

## Bộ công cụ đã tải

Các file được lưu local tại:

`D:\app\app-change-voice\.downloads\microsoft-driver-dev\installers`

| File                              | Product version   | SHA-256                                                            | Authenticode                 |
| --------------------------------- | ----------------- | ------------------------------------------------------------------ | ---------------------------- |
| `winsdksetup-10.0.28000.2526.exe` | `10.1.28000.2526` | `02988EA51EAB2A2DB53E19735E51C97A6D221ADA74B9174FA0868870B9403BA0` | Valid, Microsoft Corporation |
| `wdksetup-10.0.28000.2526.exe`    | `10.1.28000.2526` | `C80040A9DBA50460CAE149580A41C05B32742FB0463683945905F55949CAEF78` | Valid, Microsoft Corporation |

Đây là bootstrap installer chính thức, chưa phải offline layout đầy đủ. SDK/WDK và Visual Studio WDK Build Tools
integration đã được cài thành công ngày 2026-10-24. Không commit `.downloads/` vào Git.

## Source và tài liệu offline

Thư mục `.downloads/microsoft-driver-dev/windows-driver-samples-bb68ed5e` chứa bản sparse của:

- `audio/sysvad`
- `audio/simpleaudiosample`
- Microsoft Public License

Revision nguồn đã ghim:

`bb68ed5e770bce04dc5395ec9258dafab034184d`

Thư mục `.downloads/microsoft-driver-dev/docs` chứa:

- WDK download/setup.
- WaveRT architecture.
- Driver signing và test-signing.
- InfVerif.
- SysVAD README.
- Simple Audio Sample README.

## Trình tự cài đề xuất

1. Đóng Visual Studio/VS Code build tasks đang dùng Windows Kits.
2. Chạy SDK installer, xác nhận version `10.1.28000.2526`.
3. Chạy WDK installer cùng version.
4. Mở Visual Studio Installer và xác nhận component `Windows Driver Kit`/VSIX cho Visual Studio 2026.
5. Xác minh SDK/WDK build number cùng là `28000`; QFE chỉ cần khớp khi phụ thuộc header ở QFE mới.
6. Xác minh `Include\10.0.28000.0\km`, `Lib\10.0.28000.0\km\x64`, WDF, InfVerif, Inf2Cat và StampInf.
7. Chạy lại `build-driver.ps1`; không cài driver lên máy chính.

Việc cài bộ kit không yêu cầu và không được dùng để tự động tắt Secure Boot hoặc driver signature enforcement.
