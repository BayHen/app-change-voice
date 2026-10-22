use crate::effects::parameters::EffectParameters;
use serde::{Deserialize, Serialize};

pub const PRESET_SCHEMA_VERSION: u32 = 2;

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

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct PresetParameters {
    pub input_gain_percent: f32,
    pub output_gain_percent: f32,
    pub bass_db: f32,
    pub treble_db: f32,
    pub noise_suppression: bool,
    pub suppression_strength: f32,
    pub noise_gate: bool,
    pub gate_threshold_dbfs: f32,
    pub limiter_enabled: bool,
    pub limiter_ceiling_dbfs: f32,
    pub effect_bypassed: bool,
}

impl Default for PresetParameters {
    fn default() -> Self {
        Self {
            input_gain_percent: 100.0,
            output_gain_percent: 100.0,
            bass_db: 0.0,
            treble_db: 0.0,
            noise_suppression: false,
            suppression_strength: 0.8,
            noise_gate: true,
            gate_threshold_dbfs: -52.0,
            limiter_enabled: true,
            limiter_ceiling_dbfs: -1.0,
            effect_bypassed: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginRequirement {
    pub id: String,
    pub compatibility_version: String,
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
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub parameters: PresetParameters,
    #[serde(default)]
    pub plugin_requirements: Vec<PluginRequirement>,
}

impl VoicePreset {
    pub fn effect_parameters(&self) -> EffectParameters {
        let mut parameters = EffectParameters {
            pitch_semitones: self.pitch_semitones,
            formant_semitones: self.formant_semitones,
            makeup_gain_db: self.gain.makeup_db,
            bypassed: self.parameters.effect_bypassed,
            bass_db: self.parameters.bass_db,
            treble_db: self.parameters.treble_db,
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
