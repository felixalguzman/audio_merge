use tauri::State;
use crossbeam_channel::Sender;

mod audio;

pub mod config;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};
use config::AppConfig;

struct AppState {
    tx: Sender<audio::AudioCommand>,
}

#[tauri::command]
fn get_audio_devices() -> Vec<audio::AudioDeviceInfo> {
    audio::get_output_devices()
}

#[tauri::command]
fn get_default_audio_device() -> String {
    audio::get_default_device_name()
}

#[tauri::command]
fn start_audio(state: State<'_, AppState>) -> Result<(), String> {
    state.tx.send(audio::AudioCommand::StartLoopback).map_err(|e| e.to_string())
}

#[tauri::command]
fn add_device_to_mix(state: State<'_, AppState>, device_name: String) -> Result<(), String> {
    state.tx.send(audio::AudioCommand::AddOutput(device_name)).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_device_volume(state: State<'_, AppState>, device_name: String, volume: f32) -> Result<(), String> {
    state.tx.send(audio::AudioCommand::SetVolume(device_name, volume)).map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_device_from_mix(state: State<'_, AppState>, device_name: String) -> Result<(), String> {
    state.tx.send(audio::AudioCommand::RemoveOutput(device_name)).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_device_mute(state: State<'_, AppState>, device_name: String, muted: bool) -> Result<(), String> {
    state.tx.send(audio::AudioCommand::SetMute(device_name, muted)).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_audio_state() -> String {
    "TodoState".to_string() 
}

#[tauri::command]
fn set_input_volume(state: State<'_, AppState>, volume: f32) -> Result<(), String> {
    state.tx.send(audio::AudioCommand::SetInputVolume(volume)).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_input_mute(state: State<'_, AppState>, muted: bool) -> Result<(), String> {
    state.tx.send(audio::AudioCommand::SetInputMute(muted)).map_err(|e| e.to_string())
}

#[tauri::command]
fn start_capture(state: State<'_, AppState>) -> Result<(), String> {
    state.tx.send(audio::AudioCommand::StartLoopback).map_err(|e| e.to_string())
}

#[tauri::command]
fn stop_capture(state: State<'_, AppState>) -> Result<(), String> {
    state.tx.send(audio::AudioCommand::StopLoopback).map_err(|e| e.to_string())
}

// Config Commands
#[tauri::command]
fn save_app_config(app: tauri::AppHandle, config: AppConfig) -> Result<(), String> {
    config::save_config(&app, config)
}

#[tauri::command]
fn load_app_config(app: tauri::AppHandle) -> AppConfig {
    config::load_config(&app)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let tx = audio::spawn_audio_thread();
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState { tx })
        .setup(|app| {
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>).unwrap();
            let show_i = MenuItem::with_id(app, "show", "Show", true, None::<&str>).unwrap();
            let menu = Menu::with_items(app, &[&show_i, &quit_i]).unwrap();
            
            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "quit" => app.exit(0),
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                     if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png")).unwrap())
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_audio_devices,
            get_default_audio_device,
            start_audio,
            add_device_to_mix,
            set_device_volume,
            remove_device_from_mix,
            set_device_mute,
            set_input_volume,
            set_input_mute,
            start_capture,
            stop_capture,
            save_app_config,
            load_app_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
