use serde::{Deserialize, Serialize};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32, Ordering},
};

pub const MIN_PITCH_SEMITONES: f32 = -12.0;
pub const MAX_PITCH_SEMITONES: f32 = 12.0;
pub const MIN_FORMANT_SEMITONES: f32 = -6.0;
pub const MAX_FORMANT_SEMITONES: f32 = 6.0;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EffectParameters {
    pub pitch_semitones: f32,
    pub formant_semitones: f32,
    pub enabled: bool,
    pub bypassed: bool,
    pub makeup_gain_db: f32,
    pub ring_mod_frequency_hz: f32,
    pub ring_mod_mix: f32,
    pub radio_high_pass_hz: f32,
    pub radio_low_pass_hz: f32,
    pub radio_drive: f32,
    pub radio_mix: f32,
    pub tremolo_rate_hz: f32,
    pub tremolo_depth: f32,
    pub bass_db: f32,
    pub treble_db: f32,
}

impl Default for EffectParameters {
    fn default() -> Self {
        Self {
            pitch_semitones: 0.0,
            formant_semitones: 0.0,
            enabled: true,
            bypassed: false,
            makeup_gain_db: 0.0,
            ring_mod_frequency_hz: 72.0,
            ring_mod_mix: 0.0,
            radio_high_pass_hz: 280.0,
            radio_low_pass_hz: 3_400.0,
            radio_drive: 1.0,
            radio_mix: 0.0,
            tremolo_rate_hz: 5.0,
            tremolo_depth: 0.0,
            bass_db: 0.0,
            treble_db: 0.0,
        }
    }
}

impl EffectParameters {
    pub fn validated(self) -> Self {
        Self {
            pitch_semitones: finite_clamp(
                self.pitch_semitones,
                MIN_PITCH_SEMITONES,
                MAX_PITCH_SEMITONES,
                0.0,
            ),
            formant_semitones: finite_clamp(
                self.formant_semitones,
                MIN_FORMANT_SEMITONES,
                MAX_FORMANT_SEMITONES,
                0.0,
            ),
            enabled: self.enabled,
            bypassed: self.bypassed,
            makeup_gain_db: finite_clamp(self.makeup_gain_db, -12.0, 6.0, 0.0),
            ring_mod_frequency_hz: finite_clamp(self.ring_mod_frequency_hz, 20.0, 2_000.0, 72.0),
            ring_mod_mix: finite_clamp(self.ring_mod_mix, 0.0, 1.0, 0.0),
            radio_high_pass_hz: finite_clamp(self.radio_high_pass_hz, 40.0, 1_000.0, 280.0),
            radio_low_pass_hz: finite_clamp(self.radio_low_pass_hz, 1_500.0, 12_000.0, 3_400.0),
            radio_drive: finite_clamp(self.radio_drive, 1.0, 4.0, 1.0),
            radio_mix: finite_clamp(self.radio_mix, 0.0, 1.0, 0.0),
            tremolo_rate_hz: finite_clamp(self.tremolo_rate_hz, 0.1, 20.0, 5.0),
            tremolo_depth: finite_clamp(self.tremolo_depth, 0.0, 0.8, 0.0),
            bass_db: finite_clamp(self.bass_db, -12.0, 12.0, 0.0),
            treble_db: finite_clamp(self.treble_db, -12.0, 12.0, 0.0),
        }
    }
}

fn finite_clamp(value: f32, min: f32, max: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        fallback
    }
}

#[derive(Clone)]
pub struct SharedEffectParameters(Arc<EffectParameterAtoms>);

struct EffectParameterAtoms {
    pitch: AtomicU32,
    formant: AtomicU32,
    enabled: AtomicBool,
    bypassed: AtomicBool,
    makeup_gain_db: AtomicU32,
    ring_mod_frequency_hz: AtomicU32,
    ring_mod_mix: AtomicU32,
    radio_high_pass_hz: AtomicU32,
    radio_low_pass_hz: AtomicU32,
    radio_drive: AtomicU32,
    radio_mix: AtomicU32,
    tremolo_rate_hz: AtomicU32,
    tremolo_depth: AtomicU32,
    bass_db: AtomicU32,
    treble_db: AtomicU32,
}

impl SharedEffectParameters {
    pub fn new(parameters: EffectParameters) -> Self {
        let p = parameters.validated();
        Self(Arc::new(EffectParameterAtoms {
            pitch: atom(p.pitch_semitones),
            formant: atom(p.formant_semitones),
            enabled: AtomicBool::new(p.enabled),
            bypassed: AtomicBool::new(p.bypassed),
            makeup_gain_db: atom(p.makeup_gain_db),
            ring_mod_frequency_hz: atom(p.ring_mod_frequency_hz),
            ring_mod_mix: atom(p.ring_mod_mix),
            radio_high_pass_hz: atom(p.radio_high_pass_hz),
            radio_low_pass_hz: atom(p.radio_low_pass_hz),
            radio_drive: atom(p.radio_drive),
            radio_mix: atom(p.radio_mix),
            tremolo_rate_hz: atom(p.tremolo_rate_hz),
            tremolo_depth: atom(p.tremolo_depth),
            bass_db: atom(p.bass_db),
            treble_db: atom(p.treble_db),
        }))
    }

    pub fn store(&self, parameters: EffectParameters) -> EffectParameters {
        let p = parameters.validated();
        for (target, value) in [
            (&self.0.pitch, p.pitch_semitones),
            (&self.0.formant, p.formant_semitones),
            (&self.0.makeup_gain_db, p.makeup_gain_db),
            (&self.0.ring_mod_frequency_hz, p.ring_mod_frequency_hz),
            (&self.0.ring_mod_mix, p.ring_mod_mix),
            (&self.0.radio_high_pass_hz, p.radio_high_pass_hz),
            (&self.0.radio_low_pass_hz, p.radio_low_pass_hz),
            (&self.0.radio_drive, p.radio_drive),
            (&self.0.radio_mix, p.radio_mix),
            (&self.0.tremolo_rate_hz, p.tremolo_rate_hz),
            (&self.0.tremolo_depth, p.tremolo_depth),
            (&self.0.bass_db, p.bass_db),
            (&self.0.treble_db, p.treble_db),
        ] {
            target.store(value.to_bits(), Ordering::Relaxed);
        }
        self.0.enabled.store(p.enabled, Ordering::Relaxed);
        self.0.bypassed.store(p.bypassed, Ordering::Relaxed);
        p
    }

    pub fn load(&self) -> EffectParameters {
        EffectParameters {
            pitch_semitones: load(&self.0.pitch),
            formant_semitones: load(&self.0.formant),
            enabled: self.0.enabled.load(Ordering::Relaxed),
            bypassed: self.0.bypassed.load(Ordering::Relaxed),
            makeup_gain_db: load(&self.0.makeup_gain_db),
            ring_mod_frequency_hz: load(&self.0.ring_mod_frequency_hz),
            ring_mod_mix: load(&self.0.ring_mod_mix),
            radio_high_pass_hz: load(&self.0.radio_high_pass_hz),
            radio_low_pass_hz: load(&self.0.radio_low_pass_hz),
            radio_drive: load(&self.0.radio_drive),
            radio_mix: load(&self.0.radio_mix),
            tremolo_rate_hz: load(&self.0.tremolo_rate_hz),
            tremolo_depth: load(&self.0.tremolo_depth),
            bass_db: load(&self.0.bass_db),
            treble_db: load(&self.0.treble_db),
        }
    }
}

fn atom(value: f32) -> AtomicU32 {
    AtomicU32::new(value.to_bits())
}
fn load(atom: &AtomicU32) -> f32 {
    f32::from_bits(atom.load(Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_non_finite_and_out_of_range_parameters() {
        let value = EffectParameters {
            pitch_semitones: f32::NAN,
            formant_semitones: 99.0,
            ..EffectParameters::default()
        }
        .validated();
        assert_eq!(value.pitch_semitones, 0.0);
        assert_eq!(value.formant_semitones, MAX_FORMANT_SEMITONES);
    }
}
