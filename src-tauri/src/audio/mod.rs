//! Nền tảng audio thuần Rust, không phụ thuộc API riêng của Windows.

/// Hợp đồng tối thiểu của audio engine. Implementation thật sẽ được thêm sau Task 01.
pub trait AudioEngine {
    fn is_ready(&self) -> bool;
}

/// Engine rỗng giúp UI và backend giao tiếp mà chưa xử lý âm thanh.
#[derive(Debug, Default)]
pub struct NullAudioEngine;

impl AudioEngine for NullAudioEngine {
    fn is_ready(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_engine_is_not_ready() {
        assert!(!NullAudioEngine.is_ready());
    }
}
