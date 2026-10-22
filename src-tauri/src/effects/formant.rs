/// Smooths formant changes at DSP-block boundaries. Spectral-envelope processing
/// is performed by the shared Signalsmith engine so analysis only runs once.
pub struct FormantProcessor {
    current_semitones: f32,
}

impl FormantProcessor {
    pub fn new(initial_semitones: f32) -> Self {
        Self {
            current_semitones: initial_semitones,
        }
    }

    pub fn smooth_towards(&mut self, target: f32, coefficient: f32) -> f32 {
        self.current_semitones += (target - self.current_semitones) * coefficient;
        if (target - self.current_semitones).abs() < 0.001 {
            self.current_semitones = target;
        }
        self.current_semitones
    }
}
