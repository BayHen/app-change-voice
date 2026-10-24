# Driver tests — plan only

Phase 11.2.2+ phải bổ sung:

- Protocol ABI/version/invalid packet tests.
- Producer disconnect, underrun, overflow, cancellation và process crash.
- WaveRT state transitions STOP/ACQUIRE/PAUSE/RUN và position monotonicity.
- Format negotiation 48 kHz mono, concurrent capture clients và sleep/resume.
- Static Driver Verifier, CodeQL, Driver Verifier, InfVerif, Inf2Cat và audio HLK playlist.
- Separate test machine/VM; không deploy driver thử nghiệm lên máy phát triển chính.

`protocol_layout_test.cpp` hiện chỉ kiểm tra ABI header user-mode và có thể compile bằng MSVC mà không cần WDK.
