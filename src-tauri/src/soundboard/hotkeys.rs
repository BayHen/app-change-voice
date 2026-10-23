use super::SoundEntry;
use std::{collections::HashSet, str::FromStr};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

pub struct SoundboardHotkeys {
    registered: Vec<(Shortcut, String)>,
}
impl SoundboardHotkeys {
    pub fn new() -> Self {
        Self {
            registered: Vec::new(),
        }
    }
    pub fn sound_for(&self, shortcut: &Shortcut) -> Option<String> {
        self.registered
            .iter()
            .find_map(|(s, id)| (s == shortcut).then(|| id.clone()))
    }
    pub fn apply(&mut self, app: &AppHandle, entries: &[SoundEntry]) -> Result<(), String> {
        let mut requested = Vec::new();
        let mut seen = HashSet::new();
        for entry in entries {
            if let Some(value) = entry.hotkey.as_deref().filter(|v| !v.trim().is_empty()) {
                let shortcut = Shortcut::from_str(value)
                    .map_err(|e| format!("Invalid sound hotkey '{value}': {e}"))?;
                if !seen.insert(shortcut) {
                    return Err(format!("Duplicate sound hotkey: {value}"));
                }
                requested.push((shortcut, entry.id.clone()));
            }
        }
        let previous = self.registered.clone();
        for (s, _) in &previous {
            let _ = app.global_shortcut().unregister(*s);
        }
        let mut registered = Vec::new();
        for (shortcut, id) in requested {
            if let Err(e) = app.global_shortcut().register(shortcut) {
                for (s, _) in &registered {
                    let _ = app.global_shortcut().unregister(*s);
                }
                for (s, _) in &previous {
                    let _ = app.global_shortcut().register(*s);
                }
                self.registered = previous;
                return Err(format!("Sound hotkey conflict for {shortcut}: {e}"));
            }
            registered.push((shortcut, id));
        }
        self.registered = registered;
        Ok(())
    }
}
impl Default for SoundboardHotkeys {
    fn default() -> Self {
        Self::new()
    }
}
pub type SoundboardHotkeyState = std::sync::Mutex<SoundboardHotkeys>;
pub fn sound_from_event(app: &AppHandle, shortcut: &Shortcut) -> Option<String> {
    use tauri::Manager;
    app.state::<SoundboardHotkeyState>()
        .lock()
        .ok()?
        .sound_for(shortcut)
}
