# OpenVoice V2 offline prototype

Prototype chạy ngoài Tauri để tránh Python/PyTorch blocking audio callback. Nó chỉ nhận file WAV do
người dùng chủ động cung cấp và bắt buộc cờ xác nhận quyền sử dụng.

## Môi trường

- Python 3.10 hoặc 3.11 x64 (Python 3.14 hiện chưa phải target hỗ trợ của bộ dependency này).
- PyTorch CPU hoặc CUDA phù hợp với driver/máy thử.
- OpenVoice được khóa tại commit `74a1d147b17a8c3092dd5430504bd83ef6c7eb23`.
- Checkpoint V2 tải thủ công từ nguồn chính thức và giải nén; không commit weights.

```powershell
py -3.11 -m venv .venv-ai
.venv-ai\Scripts\python.exe -m pip install --upgrade pip
.venv-ai\Scripts\python.exe -m pip install -r ai-prototype\requirements.txt
.venv-ai\Scripts\python.exe -m pip install "git+https://github.com/myshell-ai/OpenVoice.git@74a1d147b17a8c3092dd5430504bd83ef6c7eb23"
```

PyTorch nên được cài theo selector chính thức của PyTorch cho CPU/CUDA đang dùng. Không cài một
wheel CUDA chỉ dựa vào việc máy có GPU.

## Chạy

```powershell
.venv-ai\Scripts\python.exe ai-prototype\openvoice_offline.py `
  --source path\source.wav `
  --reference path\reference.wav `
  --checkpoint-dir path\checkpoints_v2\converter `
  --output ai-prototype\output\converted.wav `
  --metrics ai-benchmarks\results\openvoice.json `
  --acknowledge-rights
```

`source.wav` và `reference.wav` phải do người dùng sở hữu hoặc có sự đồng ý hợp lệ. Prototype không
upload audio. Kết quả benchmark là offline inference latency, không phải end-to-end realtime latency.
