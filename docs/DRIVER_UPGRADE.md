# Nâng cấp và sửa chữa driver

## Chính sách version

Application dùng SemVer `0.1.0`; Windows driver dùng bốn thành phần `0.1.0.0`. Build script ép WDK StampInf dùng
`DriverVer=10/23/2026,0.1.0.0`, đồng bộ với manifest và version resource. Release staging từ chối mismatch.

- Chưa cài: Install.
- Version thấp hơn: Upgrade bằng PnPUtil `/add-driver ... /install`.
- Cùng version nhưng driver báo lỗi: chạy lại installer production để repair package.
- Version cao hơn: giữ driver, không downgrade tự động.
- PnPUtil trả `3010`: hoàn tất sau khi restart Windows.

`DriverInstallationService` chỉ tạo plan Install/Upgrade/Repair/Keep; nó không tự elevation hoặc chạy arbitrary
command. Mutation thuộc signed installer.

## Rollback và lỗi

Không xóa driver hiện tại trước khi package mới vượt qua validation. Nếu Windows từ chối package, installer báo lỗi
và giữ Windows security policy. Rollback driver production phải dùng package Microsoft-signed cũ đã lưu trữ an toàn,
không dùng test certificate.

Các test clean install, upgrade, repair, invalid signature, UAC denial, cancel, reboot, uninstall và rollback phải
chạy trên Windows test VM/máy riêng sau khi có package production-signed. Không chạy driver prototype unsigned trên
máy làm việc chính.
