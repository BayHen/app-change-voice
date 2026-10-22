//! Model preset không phụ thuộc UI hoặc nền tảng.

#[derive(Debug, Clone, PartialEq)]
pub struct VoicePreset {
    pub id: String,
    pub display_name: String,
}

impl VoicePreset {
    pub fn clean() -> Self {
        Self {
            id: "clean".into(),
            display_name: "Clean Voice".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_preset_has_stable_id() {
        assert_eq!(VoicePreset::clean().id, "clean");
    }
}
