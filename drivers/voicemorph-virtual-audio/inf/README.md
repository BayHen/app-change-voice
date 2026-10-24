# INF package — reserved

Chưa có INF có thể cài. Phase sau cần tạo componentized INF với:

- Provider, class, catalog, DriverVer và hardware ID/GUID riêng của VoiceMorph.
- Capture/Audio/Realtime interfaces cho đúng WaveRT endpoint.
- Device interface riêng cho bridge control/data plane và ACL tối thiểu.
- CopyFiles/Service/AddReg chỉ cho file thực sự thuộc package.
- `PnpLockdown=1`, architecture x64, kiểm tra InfVerif/Inf2Cat.

Không tái sử dụng GUID, hardware ID, tên Contoso hoặc certificate của sample SysVAD.
