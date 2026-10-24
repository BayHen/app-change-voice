# Driver sources — reserved

Phase 11.2.1 chưa thêm `.cpp/.h` kernel implementation. Phase 11.2.2 cần tạo project WDK x64 từ
SysVAD capture WaveRT subset, thay tone generator bằng bounded bridge consumer, và chứng minh IRQL,
locking, lifetime, cancellation, cleanup và silence-on-underrun bằng Driver Verifier/HLK tests.
