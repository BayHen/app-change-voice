use crate::{
    AudioRuntimeState,
    hotkeys::config::HotkeyAction,
    routing::{AudioRoutingService, RoutingState},
};
use tauri::{
    AppHandle, Manager, WebviewWindow,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

pub struct TrayManager {
    show: MenuItem<tauri::Wry>,
    preset: MenuItem<tauri::Wry>,
    mute: MenuItem<tauri::Wry>,
    effects: MenuItem<tauri::Wry>,
    routing: MenuItem<tauri::Wry>,
}
pub type TrayState = std::sync::Mutex<TrayManager>;

pub fn install(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "tray-show", "Hide Window", true, None::<&str>)?;
    let preset = MenuItem::with_id(
        app,
        "tray-preset",
        "Preset: Natural Voice",
        false,
        None::<&str>,
    )?;
    let mute = MenuItem::with_id(app, "tray-mute", "Mute Microphone", true, None::<&str>)?;
    let effects = MenuItem::with_id(app, "tray-effects", "Disable Effects", true, None::<&str>)?;
    let routing = MenuItem::with_id(
        app,
        "tray-routing",
        "Start Virtual Microphone",
        true,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(app, "tray-settings", "Open Settings", true, None::<&str>)?;
    let exit = MenuItem::with_id(
        app,
        "tray-exit",
        "Exit VoiceMorph Studio",
        true,
        None::<&str>,
    )?;
    let separator = PredefinedMenuItem::separator(app)?;
    let separator_two = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[
            &show,
            &preset,
            &separator,
            &mute,
            &effects,
            &routing,
            &settings,
            &separator_two,
            &exit,
        ],
    )?;
    app.manage(std::sync::Mutex::new(TrayManager {
        show,
        preset,
        mute,
        effects,
        routing,
    }));
    TrayIconBuilder::with_id("voicemorph-tray")
        .icon(app.default_window_icon().expect("application icon").clone())
        .tooltip("VoiceMorph Studio")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(handle_menu)
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_window(tray.app_handle());
            }
        })
        .build(app)?;
    refresh(app.handle());
    Ok(())
}

fn handle_menu(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "tray-show" => toggle_window(app),
        "tray-mute" => {
            crate::application::dispatch(app, HotkeyAction::ToggleMicrophoneMute);
        }
        "tray-effects" => {
            crate::application::dispatch(app, HotkeyAction::ToggleVoiceEffects);
        }
        "tray-routing" => {
            crate::application::dispatch(app, HotkeyAction::ToggleVirtualMicrophone);
        }
        "tray-settings" => {
            show_window(app);
            let _ = tauri::Emitter::emit(app, "voicemorph://navigate", "settings");
        }
        "tray-exit" => {
            crate::application::cleanup(app);
            app.exit(0);
        }
        _ => {}
    }
}

pub fn toggle_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            restore(&window);
        }
    }
    refresh(app);
}
pub fn show_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        restore(&window);
    }
    refresh(app);
}
fn restore(window: &WebviewWindow) {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

pub fn refresh(app: &AppHandle) {
    let Some(state) = app.try_state::<TrayState>() else {
        return;
    };
    let Ok(tray) = state.lock() else { return };
    let visible = app
        .get_webview_window("main")
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false);
    let _ = tray.show.set_text(if visible {
        "Hide Window"
    } else {
        "Show Window"
    });
    if let Ok(runtime) = app.state::<AudioRuntimeState>().lock() {
        let _ = tray.preset.set_text(format!(
            "Preset: {}",
            runtime.active_preset_id().unwrap_or("Custom")
        ));
        let _ = tray
            .mute
            .set_text(if runtime.volume_settings().input_muted {
                "Unmute Microphone"
            } else {
                "Mute Microphone"
            });
        let _ = tray
            .effects
            .set_text(if runtime.effect_parameters().enabled {
                "Disable Effects"
            } else {
                "Enable Effects"
            });
        let active = runtime.routing_status().state == RoutingState::Active;
        let _ = tray.routing.set_text(if active {
            "Stop Virtual Microphone"
        } else {
            "Start Virtual Microphone"
        });
    }
}

pub fn next_visibility(currently_visible: bool) -> bool {
    !currently_visible
}

#[cfg(test)]
mod tests {
    #[test]
    fn hide_restore_decision_is_reversible() {
        assert!(!super::next_visibility(true));
        assert!(super::next_visibility(false));
    }
}
