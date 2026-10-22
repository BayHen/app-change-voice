use super::models::{PRESET_SCHEMA_VERSION, PresetEffect, VoicePreset};
use crate::effects::parameters::{
    MAX_FORMANT_SEMITONES, MAX_PITCH_SEMITONES, MIN_FORMANT_SEMITONES, MIN_PITCH_SEMITONES,
};
use std::collections::HashSet;

pub fn validate_preset(preset: &VoicePreset) -> Result<(), String> {
    if preset.schema_version != PRESET_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported preset schema version: {}",
            preset.schema_version
        ));
    }
    if preset.id.is_empty()
        || !preset
            .id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err("Preset ID must use lowercase ASCII letters, digits, or hyphens".into());
    }
    if preset.name.trim().is_empty() || preset.description.trim().is_empty() {
        return Err("Preset name and description are required".into());
    }
    finite_range(
        "pitchSemitones",
        preset.pitch_semitones,
        MIN_PITCH_SEMITONES,
        MAX_PITCH_SEMITONES,
    )?;
    finite_range(
        "formantSemitones",
        preset.formant_semitones,
        MIN_FORMANT_SEMITONES,
        MAX_FORMANT_SEMITONES,
    )?;
    finite_range("makeupDb", preset.gain.makeup_db, -12.0, 6.0)?;
    let mut effect_kinds = HashSet::new();
    for effect in &preset.effects {
        let kind = match effect {
            PresetEffect::RingModulator { .. } => "ringModulator",
            PresetEffect::RadioFilter { .. } => "radioFilter",
            PresetEffect::Tremolo { .. } => "tremolo",
        };
        if !effect_kinds.insert(kind) {
            return Err(format!("Duplicate effect processor: {kind}"));
        }
        match effect {
            PresetEffect::RingModulator { frequency_hz, mix } => {
                finite_range("ringModulator.frequencyHz", *frequency_hz, 20.0, 2_000.0)?;
                finite_range("ringModulator.mix", *mix, 0.0, 1.0)?;
            }
            PresetEffect::RadioFilter {
                high_pass_hz,
                low_pass_hz,
                drive,
                mix,
            } => {
                finite_range("radioFilter.highPassHz", *high_pass_hz, 40.0, 1_000.0)?;
                finite_range("radioFilter.lowPassHz", *low_pass_hz, 1_500.0, 12_000.0)?;
                if high_pass_hz >= low_pass_hz {
                    return Err("Radio high-pass must be below low-pass".into());
                }
                finite_range("radioFilter.drive", *drive, 1.0, 4.0)?;
                finite_range("radioFilter.mix", *mix, 0.0, 1.0)?;
            }
            PresetEffect::Tremolo { rate_hz, depth } => {
                finite_range("tremolo.rateHz", *rate_hz, 0.1, 20.0)?;
                finite_range("tremolo.depth", *depth, 0.0, 0.8)?;
            }
        }
    }
    Ok(())
}

pub fn validate_registry(presets: &[VoicePreset]) -> Result<(), String> {
    let mut ids = HashSet::new();
    for preset in presets {
        validate_preset(preset)?;
        if !ids.insert(&preset.id) {
            return Err(format!("Duplicate preset ID: {}", preset.id));
        }
    }
    Ok(())
}

fn finite_range(name: &str, value: f32, min: f32, max: f32) -> Result<(), String> {
    if !value.is_finite() || !(min..=max).contains(&value) {
        Err(format!("{name} must be finite and between {min} and {max}"))
    } else {
        Ok(())
    }
}
