# Windows Driver Development Setup

## Trạng thái máy kiểm tra — 2026-10-23

| Thành phần                 | Trạng thái                                                                                         |
| -------------------------- | -------------------------------------------------------------------------------------------------- |
| Windows                    | Windows 10 Pro label, DisplayVersion 25H2, kernel build `10.0.26200.9711`, x64                     |
| Rust                       | `rustc 1.99.1`, host `x86_64-pc-windows-msvc`; Cargo 1.99.1                                        |
| Node/npm                   | Node 24.13.0, npm 11.6.2                                                                           |
| Visual Studio Build Tools  | VS Build Tools 2026 18.11.1, complete/launchable                                                   |
| MSVC                       | 14.51.36231; `cl.exe` 19.51.36260 x64                                                              |
| MSBuild                    | 18.11.1.51303 qua Developer Command Prompt                                                         |
| Windows SDK                | Include/bin 10.0.26100.0 và 10.0.28000.0; SignTool có                                              |
| WDK                        | 10.0.28000.2526: KM headers/libs, WDF, Inf2Cat, StampInf, SignTool và InfVerif có đủ               |
| Visual Studio WDK toolset  | `Component.Microsoft.Windows.DriverKit.BuildTools` đã cài; `WindowsKernelModeDriver10.0` hoạt động |
| ATL/MFC/Spectre            | x86/x64 và dependency ARM64 do Microsoft yêu cầu đã cài                                            |
| CMake                      | Không có trong PATH; không bắt buộc cho SysVAD/MSBuild nhưng hữu ích cho tooling phụ               |
| PnPUtil                    | Có sẵn cùng Windows; chưa dùng để thay đổi driver                                                  |
| WinDbg/driver test tooling | Chưa xác minh/cài đầy đủ                                                                           |

SDK/WDK 28000 matching pair hiện đã hoạt động và build được driver x64 bằng Visual Studio Build Tools 2026.
OS build 26200 không bắt buộc kit phải trùng tuyệt đối; Microsoft hướng dẫn dùng latest released WDK để target
Windows 10/11. Với VS 2026 hiện tại, ưu tiên stable WDK/SDK 28000 matching pair; nếu tool/sample compatibility
buộc dùng VS 2022 thì dùng WDK/SDK 26100.6584 matching pair. Không trộn KM headers của version này với libs
version khác.

## Cài dependency còn thiếu

Chỉ tải từ Microsoft:

1. Mở Visual Studio Installer, Modify Build Tools.
2. Giữ `Desktop development with C++`; thêm ATL x64 và ATL Spectre-mitigated libraries tương ứng toolset mà
   SysVAD project pin. Microsoft sample README hiện vẫn nêu v142 ATL/Spectre requirements; phase sau nên thử
   latest toolset trước và chỉ cài v142 khi project thực sự yêu cầu.
3. Cài matching Windows SDK và Windows Driver Kit từ Microsoft WDK download page. Xác nhận VSIX integration.
4. Xác minh các path sau tồn tại cho cùng version:
   - `Windows Kits\10\Include\<version>\km`
   - `Windows Kits\10\Lib\<version>\km\x64`
   - WDF headers/libraries
   - `Inf2Cat.exe`, `stampinf.exe`, `signtool.exe`, `InfVerif.exe`
5. Cài WinDbg từ Microsoft Store/SDK nếu cần kernel debugging.
6. Clone Windows Driver Samples với submodule WIL: `git submodule update --init`.

Không bật TESTSIGNING, tắt Secure Boot, tắt BitLocker hay thay boot configuration trên máy này trong phase này.
Driver test nên chạy trên máy/VM tách biệt và có snapshot/recovery.

## Gói đã tải local

SDK và WDK bootstrap `10.1.28000.2526`, source samples và tài liệu offline đã được tải vào
`.downloads/microsoft-driver-dev`. Cả hai installer có chữ ký Authenticode hợp lệ của Microsoft Corporation.
WDK/SDK và Visual Studio WDK Build Tools integration đã được cài ngày 2026-10-24. Xem
`docs/DRIVER_DOWNLOADS.md` để kiểm tra SHA-256 và vị trí file.

## Lệnh xác minh sau khi cài

Chạy từ x64 Native Tools/Developer Command Prompt:

```powershell
where cl
where msbuild
where inf2cat
where stampinf
where signtool
```

Sau đó build upstream SysVAD x64 Debug trước khi fork. Package phải có `.sys`, generated INF và `.cat`; chạy
InfVerif/Inf2Cat. Không install package cho đến khi ký test đúng cách và có test target riêng.

Microsoft khuyến nghị PnPUtil thay DevCon cho project mới. Mọi install/remove cần người dùng chủ động xác nhận,
elevated process riêng và log SetupAPI; Tauri process thường không giữ quyền administrator.
