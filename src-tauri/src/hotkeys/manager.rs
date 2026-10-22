use super::config::{HotkeyAction, HotkeyConfiguration, validate};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

pub struct HotkeyManager {
    configuration: HotkeyConfiguration,
    registered: Vec<(Shortcut, HotkeyAction)>,
}

impl HotkeyManager {
    pub fn new(configuration: HotkeyConfiguration) -> Self {
        Self {
            configuration,
            registered: Vec::new(),
        }
    }
    pub fn configuration(&self) -> &HotkeyConfiguration {
        &self.configuration
    }
    pub fn action_for(&self, shortcut: &Shortcut) -> Option<HotkeyAction> {
        self.registered
            .iter()
            .find_map(|(registered, action)| (registered == shortcut).then_some(*action))
    }
    pub fn apply(
        &mut self,
        app: &AppHandle,
        configuration: HotkeyConfiguration,
    ) -> Result<(), String> {
        let requested = validate(&configuration)?;
        let previous = self.registered.clone();
        for (shortcut, _) in &previous {
            let _ = app.global_shortcut().unregister(*shortcut);
        }
        let mut registered = Vec::new();
        for (shortcut, action) in requested {
            if let Err(error) = app.global_shortcut().register(shortcut) {
                for (value, _) in &registered {
                    let _ = app.global_shortcut().unregister(*value);
                }
                for (value, _) in &previous {
                    let _ = app.global_shortcut().register(*value);
                }
                self.registered = previous;
                return Err(format!(
                    "Shortcut conflict: {shortcut} could not be registered: {error}"
                ));
            }
            registered.push((shortcut, action));
        }
        self.registered = registered;
        self.configuration = configuration;
        Ok(())
    }
    pub fn unregister_all(&mut self, app: &AppHandle) {
        for (shortcut, _) in self.registered.drain(..) {
            let _ = app.global_shortcut().unregister(shortcut);
        }
    }
}

pub type HotkeyState = std::sync::Mutex<HotkeyManager>;

pub fn action_from_event(app: &AppHandle, shortcut: &Shortcut) -> Option<HotkeyAction> {
    app.state::<HotkeyState>().lock().ok()?.action_for(shortcut)
}
