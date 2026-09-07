pub mod agent;
pub mod apo;
pub mod model;
pub mod platform;
pub mod storage;

use model::*;
use notify::Watcher;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Instant,
};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use tauri_plugin_notification::NotificationExt;

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    error: Option<String>,
    shortcut_errors: Vec<String>,
    last_commit_ms: Option<f64>,
}
pub struct Engine {
    activation_ticket: std::sync::atomic::AtomicU64,
    store: Mutex<storage::Store>,
    status: Mutex<RuntimeStatus>,
    devices: Mutex<Vec<platform::Device>>,
    watcher: Mutex<Option<notify::RecommendedWatcher>>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    database: Database,
    devices: Vec<platform::Device>,
    ownership_ok: bool,
    apo_installed: bool,
    data_dir: String,
    status: RuntimeStatus,
}

fn snapshot(app: &tauri::AppHandle) -> Snapshot {
    let e = app.state::<Engine>();
    let s = e.store.lock().unwrap();
    let result = Snapshot {
        database: s.db.clone(),
        devices: e.devices.lock().unwrap().clone(),
        ownership_ok: s.ownership_ok(),
        apo_installed: Path::new(&s.db.integration.config_dir)
            .parent()
            .map(|p| p.join("Editor.exe").exists())
            .unwrap_or(false),
        data_dir: s.dir.display().to_string(),
        status: e.status.lock().unwrap().clone(),
    };
    result
}
fn error(app: &tauri::AppHandle, message: String) {
    app.state::<Engine>().status.lock().unwrap().error = Some(message.clone());
    let _ = app
        .notification()
        .builder()
        .title("Ananda Control • Action failed")
        .body(&message)
        .show();
    publish(app);
}
fn publish(app: &tauri::AppHandle) {
    let s = snapshot(app);
    let _ = app.emit("state-changed", &s);
    if let Err(err) = update_tray(app, &s) {
        app.state::<Engine>().status.lock().unwrap().error =
            Some(format!("Tray update failed: {err}"));
    }
}
fn update_tray(app: &tauri::AppHandle, s: &Snapshot) -> tauri::Result<()> {
    let menu = Menu::new(app)?;
    let connected = s
        .devices
        .iter()
        .find(|d| d.id == s.database.integration.device_id);
    let label = if !s.database.integration.installed {
        "Setup needed".to_string()
    } else if !s.ownership_ok {
        "⚠ Configuration needs attention".into()
    } else if connected.is_some_and(|d| d.connected) {
        format!("● {} connected", connected.unwrap().name)
    } else {
        "○ Output disconnected".into()
    };
    menu.append(&MenuItem::with_id(
        app,
        "status",
        label,
        false,
        None::<&str>,
    )?)?;
    let active = s
        .database
        .profiles
        .iter()
        .find(|p| p.id == s.database.active_profile)
        .unwrap();
    menu.append(&MenuItem::with_id(
        app,
        "current",
        format!("Current: {}", active.name),
        false,
        None::<&str>,
    )?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    for p in &s.database.profiles {
        if p.headphone_id != s.database.selected_headphone && p.id != "stock" && p.id != active.id {
            continue;
        }
        let emoji = match p.icon.as_str() {
            "music" => "♫",
            "film" => "🎬",
            "gamepad" => "🎮",
            "target" => "🎯",
            "mic" => "💬",
            "power" => "🔊",
            _ => "🎧",
        };
        menu.append(&CheckMenuItem::with_id(
            app,
            format!("profile:{}", p.id),
            format!("{emoji} {}", p.name),
            s.database.integration.installed,
            p.id == active.id,
            None::<&str>,
        )?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&CheckMenuItem::with_id(
        app,
        "toggle",
        "EQ enabled",
        s.database.integration.installed,
        active.id != "stock",
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        "open",
        "Open Control Center",
        true,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        "settings",
        "Settings",
        true,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(app, "exit", "Exit", true, None::<&str>)?)?;
    if let Some(tray) = app.tray_by_id("main-tray") {
        tray.set_menu(Some(menu))?;
        tray.set_tooltip(Some(format!(
            "Ananda Control • {}{}",
            active.name,
            if s.ownership_ok {
                ""
            } else {
                " • Setup / attention needed"
            }
        )))?;
    }
    Ok(())
}
fn open_main(app: &tauri::AppHandle, page: &str) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
        let _ = w.emit("navigate", page);
        return;
    }
    let result = WebviewWindowBuilder::new(
        app,
        "main",
        WebviewUrl::App(format!("index.html?page={page}").into()),
    )
    .title("Ananda Control")
    .inner_size(1220.0, 820.0)
    .min_inner_size(860.0, 640.0)
    .center()
    .build();
    if let Err(e) = result {
        error(app, format!("Could not open control center: {e}"));
    }
}
fn open_quick(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("quick") {
        let _ = w.destroy();
        return;
    }
    if let Err(e) =
        WebviewWindowBuilder::new(app, "quick", WebviewUrl::App("index.html?quick=1".into()))
            .title("Ananda quick switcher")
            .inner_size(370.0, 480.0)
            .resizable(false)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .center()
            .build()
    {
        error(app, e.to_string());
    }
}
fn regenerate(next: &mut Database) -> Result<(), String> {
    if next.integration.installed {
        let p = next
            .profiles
            .iter()
            .find(|p| p.id == next.active_profile)
            .ok_or("Profile not found")?;
        next.integration.expected_active = apo::generate(p, &next.integration.device_id)?;
    }
    Ok(())
}
fn next_ticket(app: &tauri::AppHandle) -> u64 {
    app.state::<Engine>()
        .activation_ticket
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        + 1
}
fn activate_inner(
    app: &tauri::AppHandle,
    id: &str,
    notify: bool,
    ticket: u64,
) -> Result<(), String> {
    let timer = Instant::now();
    let e = app.state::<Engine>();
    let (name, show) = {
        let mut s = e.store.lock().unwrap();
        if ticket
            != e.activation_ticket
                .load(std::sync::atomic::Ordering::SeqCst)
        {
            return Ok(());
        }
        if !s.db.integration.installed {
            return Err("Complete the one-time APO setup first".into());
        }
        let mut next = s.db.clone();
        let p = next
            .profiles
            .iter()
            .find(|p| p.id == id)
            .ok_or("Profile not found")?;
        let name = p.name.clone();
        if id != "stock" {
            next.selected_headphone = p.headphone_id.clone();
            next.last_enabled = id.into();
        }
        next.active_profile = id.into();
        regenerate(&mut next)?;
        let show = next.settings.notifications;
        s.save(next, true)?;
        (name, show)
    };
    {
        let mut status = e.status.lock().unwrap();
        status.error = None;
        status.last_commit_ms = Some(timer.elapsed().as_secs_f64() * 1000.0);
    }
    publish(app);
    if show
        && notify
        && ticket
            == e.activation_ticket
                .load(std::sync::atomic::Ordering::SeqCst)
    {
        let _ = app
            .notification()
            .builder()
            .title("Ananda Control")
            .body(format!("{name} activated"))
            .show();
    }
    Ok(())
}
fn register_shortcuts(app: &tauri::AppHandle) {
    let _ = app.global_shortcut().unregister_all();
    let s = snapshot(app);
    let mut errors = vec![];
    let mut bindings = s.database.settings.shortcuts.clone();
    bindings.insert("__quick".into(), s.database.settings.quick_shortcut.clone());
    for (id, binding) in bindings {
        if binding.trim().is_empty() {
            continue;
        }
        if id != "__quick" && !s.database.profiles.iter().any(|p| p.id == id) {
            continue;
        }
        match binding.parse::<Shortcut>() {
            Ok(shortcut) => {
                if let Err(err) =
                    app.global_shortcut()
                        .on_shortcut(shortcut, move |app, _, event| {
                            if event.state != ShortcutState::Pressed {
                                return;
                            }
                            if id == "__quick" {
                                open_quick(app);
                            } else {
                                let app = app.clone();
                                let id = id.clone();
                                let ticket = next_ticket(&app);
                                std::thread::spawn(move || {
                                    if let Err(e) = activate_inner(&app, &id, true, ticket) {
                                        error(&app, e);
                                    }
                                });
                            }
                        })
                {
                    errors.push(format!("{binding}: {err}"));
                }
            }
            Err(err) => errors.push(format!("{binding}: {err}")),
        }
    }
    app.state::<Engine>().status.lock().unwrap().shortcut_errors = errors;
}
fn start_file_watch(app: &tauri::AppHandle) {
    let handle = app.clone();
    let watcher =
        notify::recommended_watcher(move |event: Result<notify::Event, notify::Error>| {
            if let Ok(event) = event {
                if event.paths.iter().any(|p| {
                    p.file_name()
                        .is_some_and(|n| n == "config.txt" || n == "active.txt")
                }) {
                    // Store lock serializes observations after our own commits finish.
                    publish(&handle);
                }
            }
        });
    if let Ok(mut w) = watcher {
        let dir = app
            .state::<Engine>()
            .store
            .lock()
            .unwrap()
            .db
            .integration
            .config_dir
            .clone();
        if w.watch(Path::new(&dir), notify::RecursiveMode::Recursive)
            .is_ok()
        {
            *app.state::<Engine>().watcher.lock().unwrap() = Some(w);
        }
    }
}

#[tauri::command]
fn get_state(app: tauri::AppHandle) -> Snapshot {
    snapshot(&app)
}
#[tauri::command]
fn close_main(app: tauri::AppHandle) -> Result<(), String> {
    let tray = app
        .state::<Engine>()
        .store
        .lock()
        .unwrap()
        .db
        .settings
        .close_to_tray;
    if tray {
        if let Some(w) = app.get_webview_window("main") {
            w.destroy().map_err(|e| e.to_string())?;
        }
    } else {
        app.exit(0);
    }
    Ok(())
}
#[tauri::command]
async fn activate(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let ticket = next_ticket(&app);
    tauri::async_runtime::spawn_blocking(move || activate_inner(&app, &id, true, ticket))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn toggle_eq(app: tauri::AppHandle) -> Result<(), String> {
    let s = snapshot(&app);
    let id = if s.database.active_profile == "stock" {
        s.database.last_enabled
    } else {
        "stock".into()
    };
    activate(app, id).await
}
#[tauri::command]
fn save_profile(
    app: tauri::AppHandle,
    profile: Profile,
    expected: Option<Profile>,
) -> Result<(), String> {
    validate_profile(&profile)?;
    {
        let e = app.state::<Engine>();
        let mut s = e.store.lock().unwrap();
        let mut next = s.db.clone();
        let index = next
            .profiles
            .iter()
            .position(|p| p.id == profile.id)
            .ok_or("Profile not found")?;
        if profile.id == "stock" {
            return Err("Stock cannot be edited".into());
        }
        if expected
            .as_ref()
            .is_some_and(|old| old != &next.profiles[index] && profile != next.profiles[index])
        {
            return Err("This profile changed elsewhere. Reopen it to load the latest values before editing.".into());
        }
        let apply = profile.id == next.active_profile;
        next.profiles[index] = profile;
        regenerate(&mut next)?;
        s.save(next, apply)?;
    }
    publish(&app);
    Ok(())
}
#[tauri::command]
fn profile_action(
    app: tauri::AppHandle,
    action: String,
    id: Option<String>,
) -> Result<String, String> {
    let result;
    {
        let e = app.state::<Engine>();
        let mut s = e.store.lock().unwrap();
        let mut next = s.db.clone();
        let index = id
            .as_ref()
            .and_then(|id| next.profiles.iter().position(|p| &p.id == id));
        result = match action.as_str() {
            "create" | "duplicate" => {
                let mut p = if action == "duplicate" {
                    next.profiles
                        .get(index.ok_or("Select a profile")?)
                        .ok_or("Missing profile")?
                        .clone()
                } else {
                    Profile {
                        schema_version: 1,
                        id: String::new(),
                        name: "New profile".into(),
                        icon: "headphones".into(),
                        headphone_id: next.selected_headphone.clone(),
                        preamp: 0.0,
                        filters: vec![],
                        graphic_eq: vec![],
                        provenance: "Created in Ananda Control".into(),
                    }
                };
                if action == "duplicate" {
                    p.name = format!("{} copy", p.name.chars().take(70).collect::<String>());
                    p.provenance = format!("Copy of {}", p.name);
                }
                p.id = uuid::Uuid::new_v4().to_string();
                let id = p.id.clone();
                next.profiles.insert(next.profiles.len() - 1, p);
                id
            }
            "delete" => {
                let index = index.ok_or("Missing profile")?;
                let p = &next.profiles[index];
                if p.id == "stock" || p.id == next.active_profile || p.id == next.last_enabled {
                    return Err("Switch to another enabled profile before deleting this profile. Stock cannot be deleted.".into());
                }
                let id = p.id.clone();
                next.settings.shortcuts.remove(&id);
                next.profiles.remove(index);
                id
            }
            "up" | "down" => {
                let index = index.ok_or("Missing profile")?;
                let offset = if action == "up" { -1 } else { 1 };
                let other = index as isize + offset;
                if other >= 0 && (other as usize) < next.profiles.len() {
                    next.profiles.swap(index, other as usize);
                }
                id.unwrap()
            }
            _ => return Err("Unknown profile action".into()),
        };
        s.save(next, false)?;
    }
    register_shortcuts(&app);
    publish(&app);
    Ok(result)
}
#[tauri::command]
fn headphone_action(
    app: tauri::AppHandle,
    action: String,
    id: String,
    name: String,
) -> Result<(), String> {
    {
        let e = app.state::<Engine>();
        let mut s = e.store.lock().unwrap();
        let mut next = s.db.clone();
        match action.as_str() {
            "create" => {
                if name.trim().is_empty() || name.chars().count() > 80 {
                    return Err("Enter a headphone name (1–80 characters)".into());
                }
                next.headphones.push(Headphone {
                    id: uuid::Uuid::new_v4().to_string(),
                    name,
                });
            }
            "rename" => {
                if name.trim().is_empty() || name.chars().count() > 80 {
                    return Err("Enter a headphone name (1–80 characters)".into());
                }
                next.headphones
                    .iter_mut()
                    .find(|h| h.id == id)
                    .ok_or("Missing headphone")?
                    .name = name;
            }
            "select" => {
                if !next.headphones.iter().any(|h| h.id == id) {
                    return Err("Missing headphone".into());
                }
                next.selected_headphone = id;
            }
            "delete" => {
                if next.selected_headphone == id
                    || next.profiles.iter().any(|p| p.headphone_id == id)
                {
                    return Err("Move this headphone's profiles and select another headphone before deleting it".into());
                }
                next.headphones.retain(|h| h.id != id);
            }
            _ => return Err("Unknown headphone action".into()),
        }
        s.save(next, false)?;
    }
    publish(&app);
    Ok(())
}
#[tauri::command]
fn update_settings(app: tauri::AppHandle, settings: Settings) -> Result<(), String> {
    let old = snapshot(&app).database.settings;
    if settings.start_with_windows != old.start_with_windows {
        platform::set_autostart(settings.start_with_windows)?;
    }
    let saved = {
        let e = app.state::<Engine>();
        let mut s = e.store.lock().unwrap();
        let mut next = s.db.clone();
        next.settings = settings;
        s.save(next, false)
    };
    if saved.is_err() {
        let _ = platform::set_autostart(old.start_with_windows);
    }
    saved?;
    register_shortcuts(&app);
    publish(&app);
    Ok(())
}
#[tauri::command]
fn import_profile(app: tauri::AppHandle, path: String) -> Result<String, String> {
    let path = PathBuf::from(path);
    if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 1_048_576 {
        return Err("Preset exceeds 1 MB".into());
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Imported preset");
    let mut p = if path
        .extension()
        .is_some_and(|s| s.eq_ignore_ascii_case("json"))
    {
        serde_json::from_str::<Profile>(&text)
            .map_err(|e| format!("Invalid Ananda profile JSON: {e}"))?
    } else {
        apo::parse(&text, name, &path.display().to_string())?
    };
    p.id = uuid::Uuid::new_v4().to_string();
    p.provenance = format!("Imported from {}", path.display());
    validate_profile(&p)?;
    let id = p.id.clone();
    {
        let e = app.state::<Engine>();
        let mut s = e.store.lock().unwrap();
        let mut next = s.db.clone();
        p.headphone_id = next.selected_headphone.clone();
        next.profiles.insert(next.profiles.len() - 1, p);
        s.save(next, false)?;
    }
    publish(&app);
    Ok(id)
}
#[tauri::command]
fn export_profile(app: tauri::AppHandle, id: String, path: String) -> Result<(), String> {
    let s = snapshot(&app);
    let p = s
        .database
        .profiles
        .iter()
        .find(|p| p.id == id)
        .ok_or("Missing profile")?;
    let path = PathBuf::from(path);
    if path.starts_with(&s.database.integration.config_dir) {
        return Err("Export outside the live APO configuration folder".into());
    }
    if path
        .extension()
        .is_some_and(|s| s.eq_ignore_ascii_case("json"))
    {
        storage::json_write(&path, p)
    } else if path
        .extension()
        .is_some_and(|s| s.eq_ignore_ascii_case("txt"))
    {
        storage::atomic_write(&path, apo::export(p)?.as_bytes())
    } else {
        Err("Choose a .json or .txt file".into())
    }
}

fn integrate(app: &tauri::AppHandle, device_id: String, repair: bool) -> Result<(), String> {
    let e = app.state::<Engine>();
    let mut s = e.store.lock().unwrap();
    let config = platform::config_dir();
    if !config.join("config.txt").exists() {
        return Err("Install Equalizer APO and configure the output device first".into());
    }
    let device = platform::devices()
        .into_iter()
        .find(|d| d.id == device_id)
        .ok_or("Output endpoint no longer exists")?;
    if !device.apo_registered {
        return Err("Equalizer APO is not registered on this endpoint. Use Device Selector from Diagnostics, then retry.".into());
    }
    if !repair && s.db.integration.installed {
        return Err("Integration is already installed".into());
    }
    let original = fs::read(&config.join("config.txt")).map_err(|e| e.to_string())?;
    let backup = s.dir.join("backups").join(uuid::Uuid::new_v4().to_string());
    fs::create_dir_all(&backup).map_err(|e| e.to_string())?;
    storage::atomic_write(&backup.join("config.txt"), &original)?;
    storage::json_write(&backup.join("database.json"), &s.db)?;
    for file in ["peace.txt", "peace.ini", "ananda-control/active.txt"] {
        if config.join(file).exists() {
            let dest = backup.join(file.replace('/', "_"));
            fs::copy(config.join(file), dest).map_err(|e| e.to_string())?;
        }
    }
    let root="# Ananda Control owns this configuration. Restore through Diagnostics.\nInclude: ananda-control/active.txt\n".to_string();
    let mut next = s.db.clone();
    next.integration.installed = true;
    next.integration.config_dir = config.display().to_string();
    next.integration.device_id = device_id;
    next.integration.expected_root = root.clone();
    next.integration.audio_verified = false;
    if !repair || next.integration.backup_dir.is_none() {
        next.integration.backup_dir = Some(backup.display().to_string());
    }
    regenerate(&mut next)?;
    let active = config.join("ananda-control/active.txt");
    let previous_active = fs::read(&active).ok();
    // Validate access before stopping Peace or changing startup behavior.
    storage::atomic_write(
        &config.join("ananda-control/access-check.tmp"),
        b"access check",
    )
    .map_err(|e| format!("Setup needs write access to the APO config folder: {e}"))?;
    let _ = fs::remove_file(config.join("ananda-control/access-check.tmp"));
    if let Err(err) = platform::peace_handover(&config, &backup) {
        let _ = platform::peace_restore(&config, &backup);
        return Err(format!(
            "Peace handover failed; startup restoration attempted: {err}"
        ));
    }
    let result = (|| {
        storage::atomic_write(&active, next.integration.expected_active.as_bytes())?;
        storage::atomic_write(&config.join("config.txt"), root.as_bytes())?;
        s.save(next, false)
    })();
    if let Err(err) = result {
        let rollback = storage::atomic_write(&config.join("config.txt"), &original);
        if let Some(bytes) = previous_active {
            let _ = storage::atomic_write(&active, &bytes);
        }
        let _ = platform::peace_restore(&config, &backup);
        return Err(format!(
            "Setup failed: {err}. Config rollback: {}. Backup: {}",
            if rollback.is_ok() {
                "complete"
            } else {
                "needs manual restoration"
            },
            backup.display()
        ));
    }
    drop(s);
    if snapshot(app).database.settings.start_with_windows {
        if let Err(err) = platform::set_autostart(true) {
            e.status.lock().unwrap().error = Some(format!(
                "EQ integration succeeded, but Windows startup could not be enabled: {err}"
            ));
        }
    }
    start_file_watch(app);
    publish(app);
    Ok(())
}
#[tauri::command]
async fn setup_integration(
    app: tauri::AppHandle,
    device_id: String,
    repair: bool,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || integrate(&app, device_id, repair))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
async fn restore_integration(app: tauri::AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move ||{
    {let e=app.state::<Engine>();let mut s=e.store.lock().unwrap();if !s.ownership_ok(){return Err("Configuration changed externally. Back up and repair it before restoring the original configuration.".into());}
        let backup=PathBuf::from(s.db.integration.backup_dir.as_ref().ok_or("No backup available")?);let config=PathBuf::from(&s.db.integration.config_dir);
        let original=fs::read(backup.join("config.txt")).map_err(|e|e.to_string())?;
        let mut next=s.db.clone();next.integration.installed=false;next.integration.audio_verified=false;next.settings.start_with_windows=false;
        storage::atomic_write(&config.join("config.txt"),&original)?;
        if let Err(err)=s.save(next,false){let _=storage::atomic_write(&config.join("config.txt"),s.db.integration.expected_root.as_bytes());return Err(err);}
        platform::peace_restore(&config,&backup)?;
    }let _=platform::set_autostart(false);publish(&app);Ok(())
}).await.map_err(|e|e.to_string())?
}
#[tauri::command]
fn diagnostic_action(app: tauri::AppHandle, action: String) -> Result<(), String> {
    match action.as_str() {
        "grant-access" => platform::request_access()?,
        "refresh" => {
            *app.state::<Engine>().devices.lock().unwrap() = platform::devices();
        }
        "open-data" => platform::open_folder(&app.state::<Engine>().store.lock().unwrap().dir)?,
        "device-selector" => platform::open_device_selector()?,
        "verified" | "unverified" => {
            let e = app.state::<Engine>();
            let mut s = e.store.lock().unwrap();
            if action == "verified" && !s.ownership_ok() {
                return Err(
                    "Complete integration and resolve configuration conflicts first".into(),
                );
            }
            let mut next = s.db.clone();
            next.integration.audio_verified = action == "verified";
            s.save(next, false)?;
        }
        "clear-error" => app.state::<Engine>().status.lock().unwrap().error = None,
        _ => return Err("Unknown diagnostic action".into()),
    }
    publish(&app);
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if !args.iter().any(|a| a == "--background") {
                open_main(app, "profiles");
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_state,
            close_main,
            activate,
            toggle_eq,
            save_profile,
            profile_action,
            headphone_action,
            update_settings,
            import_profile,
            export_profile,
            setup_integration,
            restore_integration,
            diagnostic_action,
            agent::agent_tools,
            agent::agent_call
        ])
        .setup(|app| {
            let mut store = storage::Store::open(app.path().app_data_dir()?)?;
            if !store.db.integration.installed {
                store.db.integration.config_dir = platform::config_dir().display().to_string();
            }
            let status = RuntimeStatus {
                error: store.startup_error.clone(),
                ..Default::default()
            };
            app.manage(Engine {
                activation_ticket: std::sync::atomic::AtomicU64::new(0),
                store: Mutex::new(store),
                status: Mutex::new(status),
                devices: Mutex::new(platform::devices()),
                watcher: Mutex::new(None),
            });
            TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip("Ananda Control")
                .on_menu_event(|app, event| {
                    let id = event.id.as_ref();
                    match id {
                        "open" => open_main(app, "profiles"),
                        "settings" => open_main(app, "settings"),
                        "exit" => app.exit(0),
                        "toggle" => {
                            let s = snapshot(app);
                            let id = if s.database.active_profile == "stock" {
                                s.database.last_enabled
                            } else {
                                "stock".into()
                            };
                            let app = app.clone();
                            let ticket = next_ticket(&app);
                            std::thread::spawn(move || {
                                if let Err(e) = activate_inner(&app, &id, true, ticket) {
                                    error(&app, e);
                                }
                            });
                        }
                        _ => {
                            if let Some(id) = id.strip_prefix("profile:") {
                                let id = id.to_owned();
                                let app = app.clone();
                                let ticket = next_ticket(&app);
                                std::thread::spawn(move || {
                                    if let Err(e) = activate_inner(&app, &id, true, ticket) {
                                        error(&app, e);
                                    }
                                });
                            }
                        }
                    }
                })
                .build(app)?;
            register_shortcuts(app.handle());
            start_file_watch(app.handle());
            if let Err(err) = agent::start(app.handle()) {
                app.state::<Engine>().status.lock().unwrap().error =
                    Some(format!("Agent connection: {err}"));
            }
            let handle = app.handle().clone();
            platform::watch_devices(move || {
                *handle.state::<Engine>().devices.lock().unwrap() = platform::devices();
                publish(&handle);
            });
            let s = snapshot(app.handle());
            if s.database.integration.installed
                && s.database.settings.restore_last_profile
                && s.ownership_ok
            {
                let app = app.handle().clone();
                let ticket = next_ticket(&app);
                std::thread::spawn(move || {
                    if let Err(e) = activate_inner(&app, &s.database.active_profile, false, ticket)
                    {
                        error(&app, e);
                    }
                });
            }
            let background = std::env::args().any(|a| a == "--background");
            if !s.database.integration.installed
                || !background
                || !s.database.settings.start_in_tray
            {
                open_main(app.handle(), "profiles");
            }
            publish(app.handle());
            Ok(())
        })
        .on_window_event(|w, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } if w.label() == "main" => {
                api.prevent_close();
                let _ = w.emit("prepare-close", ());
            }
            tauri::WindowEvent::Focused(false) if w.label() == "quick" => {
                let _ = w.destroy();
            }
            _ => {}
        })
        .build(tauri::generate_context!())
        .expect("Unable to initialize Ananda Control")
        .run(|_, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
