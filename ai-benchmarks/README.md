# AI benchmarks

`benchmark.py` chạy lại prototype nhiều lần và tổng hợp wall-clock latency/RSS từ file metrics.
Kết quả trong `results/` là dữ liệu máy cục bộ và bị Git ignore. Không gọi chỉ số này là end-to-end
latency; phép đo realtime phải dùng loopback thiết bị thật.
