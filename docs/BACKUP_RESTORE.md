# Backup và Restore

Backup là JSON manifest schema v1 độc lập hệ điều hành. File chứa custom presets, hotkeys, volume/noise settings,
application profiles và Soundboard metadata. Built-in presets, recordings, raw microphone audio, tokens, WASAPI
IDs, driver data và absolute sound-file paths không được đưa vào backup.

Restore đọc tối đa 4 MiB, kiểm tra schema/range/duplicate ID và hiển thị preview conflict. Merge cập nhật entry
trùng ID và giữ entry không liên quan; Replace thay các collection được quản lý. Trước mutation, snapshot hợp lệ
được ghi tại `app_config_dir/backup-safety/latest.json`. Nếu một bước lỗi, state được rollback.

Soundboard backup chỉ chứa metadata portable. Trên máy mới entry không có audio file được đánh dấu missing;
người dùng phải thêm lại file có quyền sử dụng. Recording không được backup mặc định.

Cloud sync được hoãn. `SyncProvider` và status local-only là seam kiến trúc, nhưng không có network request,
account, token hoặc upload. Provider tương lai phải dùng OS credential storage và không được sync raw audio.
