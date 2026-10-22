use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::Path};
use tauri_plugin_global_shortcut::Shortcut;

pub const HOTKEY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum HotkeyAction {
    ToggleMicrophoneMute,
    ToggleVoiceEffects,
    NextVoicePreset,
    PreviousVoicePreset,
    ToggleHearMyself,
    ToggleVirtualMicrophone,
    ToggleRecording,
}

impl HotkeyAction {
    pub const ALL: [Self; 7] = [
        Self::ToggleMicrophoneMute,
        Self::ToggleVoiceEffects,
        Self::NextVoicePreset,
        Self::PreviousVoicePreset,
        Self::ToggleHearMyself,
        Self::ToggleVirtualMicrophone,
        Self::ToggleRecording,
    ];
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyBinding {
    pub action: HotkeyAction,
    pub shortcut: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrayPreferences {
    pub minimize_to_tray: bool,
    pub close_to_tray: bool,
}

impl Default for TrayPreferences {
    fn default() -> Self {
        Self {
            minimize_to_tray: false,
            close_to_tray: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HotkeyConfiguration {
    pub schema_version: u32,
    pub bindings: Vec<HotkeyBinding>,
    pub tray: TrayPreferences,
}

impl Default for HotkeyConfiguration {
    fn default() -> Self {
        use HotkeyAction::*;
        Self {
            schema_version: HOTKEY_SCHEMA_VERSION,
            bindings: vec![
                binding(ToggleMicrophoneMute, "Ctrl+Alt+M"),
                binding(ToggleVoiceEffects, "Ctrl+Alt+E"),
                binding(NextVoicePreset, "Ctrl+Alt+Right"),
                binding(PreviousVoicePreset, "Ctrl+Alt+Left"),
                binding(ToggleHearMyself, "Ctrl+Alt+H"),
                binding(ToggleVirtualMicrophone, "Ctrl+Alt+V"),
                binding(ToggleRecording, "Ctrl+Alt+R"),
            ],
            tray: TrayPreferences::default(),
        }
    }
}

fn binding(action: HotkeyAction, shortcut: &str) -> HotkeyBinding {
    HotkeyBinding {
        action,
        shortcut: Some(shortcut.into()),
    }
}

pub fn validate(
    configuration: &HotkeyConfiguration,
) -> Result<Vec<(Shortcut, HotkeyAction)>, String> {
    if configuration.schema_version != HOTKEY_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported hotkey schema version: {}",
            configuration.schema_version
        ));
    }
    let actions: HashSet<_> = configuration
        .bindings
        .iter()
        .map(|value| value.action)
        .collect();
    if actions.len() != HotkeyAction::ALL.len()
        || configuration.bindings.len() != HotkeyAction::ALL.len()
    {
        return Err("Configuration must contain each hotkey action exactly once".into());
    }
    let mut shortcuts = HashSet::new();
    let mut parsed = Vec::new();
    for binding in &configuration.bindings {
        let Some(value) = binding
            .shortcut
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let normalized = value.to_ascii_lowercase().replace(' ', "");
        if !normalized.contains('+')
            || !["ctrl", "control", "alt", "shift", "super", "win", "meta"]
                .iter()
                .any(|modifier| normalized.contains(modifier))
        {
            return Err(format!(
                "Shortcut '{value}' must include at least one modifier key"
            ));
        }
        if is_reserved(&normalized) {
            return Err(format!(
                "Shortcut '{value}' is reserved by Windows or the application shell"
            ));
        }
        if !shortcuts.insert(normalized) {
            return Err(format!("Duplicate shortcut: {value}"));
        }
        let shortcut = value
            .parse::<Shortcut>()
            .map_err(|error| format!("Invalid shortcut '{value}': {error}"))?;
        parsed.push((shortcut, binding.action));
    }
    Ok(parsed)
}

fn is_reserved(value: &str) -> bool {
    matches!(
        value,
        "alt+f4"
            | "ctrl+alt+delete"
            | "ctrl+shift+escape"
            | "ctrl+escape"
            | "super+l"
            | "win+l"
            | "super+d"
            | "win+d"
            | "super+r"
            | "win+r"
            | "super+e"
            | "win+e"
    )
}

pub fn load(path: &Path) -> Result<HotkeyConfiguration, String> {
    match fs::read(path) {
        Ok(bytes) => {
            let value: HotkeyConfiguration =
                serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
            validate(&value)?;
            Ok(value)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(HotkeyConfiguration::default())
        }
        Err(error) => Err(error.to_string()),
    }
}

pub fn save(path: &Path, configuration: &HotkeyConfiguration) -> Result<(), String> {
    validate(configuration)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let temporary = path.with_extension("json.tmp");
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(configuration).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    if path.exists() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }
    fs::rename(temporary, path).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_duplicates_and_reserved_shortcuts() {
        let mut value = HotkeyConfiguration::default();
        value.bindings[1].shortcut = value.bindings[0].shortcut.clone();
        assert!(validate(&value).unwrap_err().contains("Duplicate"));
        value.bindings[1].shortcut = Some("Alt+F4".into());
        assert!(validate(&value).unwrap_err().contains("reserved"));
    }
    #[test]
    fn all_default_shortcuts_parse_and_are_unique() {
        assert_eq!(
            validate(&HotkeyConfiguration::default()).unwrap().len(),
            HotkeyAction::ALL.len()
        );
    }
    #[test]
    fn saves_and_loads_configuration_after_restart() {
        let root = std::env::temp_dir().join(format!("voicemorph-hotkeys-{}", std::process::id()));
        let path = root.join("hotkeys.json");
        let expected = HotkeyConfiguration::default();
        save(&path, &expected).unwrap();
        assert_eq!(load(&path).unwrap(), expected);
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
