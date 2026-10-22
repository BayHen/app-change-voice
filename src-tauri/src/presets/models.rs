use crate::effects::parameters::EffectParameters;
use serde::{Deserialize, Serialize};

pub const PRESET_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum PresetCategory {
    Natural,
    Character,
    Creative,
    Utility,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PresetGain {
    pub makeup_db: f32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PresetEffect {
    RingModulator {
        frequency_hz: f32,
        mix: f32,
    },
    RadioFilter {
        high_pass_hz: f32,
        low_pass_hz: f32,
        drive: f32,
        mix: f32,
    },
    Tremolo {
        rate_hz: f32,
        depth: f32,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VoicePreset {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: PresetCategory,
    pub pitch_semitones: f32,
    pub formant_semitones: f32,
    pub gain: PresetGain,
    #[serde(default)]
    pub effects: Vec<PresetEffect>,
    pub built_in: bool,
}

impl VoicePreset {
    pub fn effect_parameters(&self) -> EffectParameters {
        let mut parameters = EffectParameters {
            pitch_semitones: self.pitch_semitones,
            formant_semitones: self.formant_semitones,
            makeup_gain_db: self.gain.makeup_db,
            ..EffectParameters::default()
        };
        for effect in &self.effects {
            match *effect {
                PresetEffect::RingModulator { frequency_hz, mix } => {
                    parameters.ring_mod_frequency_hz = frequency_hz;
                    parameters.ring_mod_mix = mix;
                }
                PresetEffect::RadioFilter {
                    high_pass_hz,
                    low_pass_hz,
                    drive,
                    mix,
                } => {
                    parameters.radio_high_pass_hz = high_pass_hz;
                    parameters.radio_low_pass_hz = low_pass_hz;
                    parameters.radio_drive = drive;
                    parameters.radio_mix = mix;
                }
                PresetEffect::Tremolo { rate_hz, depth } => {
                    parameters.tremolo_rate_hz = rate_hz;
                    parameters.tremolo_depth = depth;
                }
            }
        }
        parameters
    }
}
