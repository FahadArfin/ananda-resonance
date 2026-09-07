use serde::{Deserialize, Serialize};
use std::os::windows::process::CommandExt;
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};
use winreg::{enums::*, RegKey};

pub const RENDER_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\Audio\Render";
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
        .create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run")
        .map_err(|e| e.to_string())?;
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        key.set_value(
            "Ananda Control",
            &format!("\"{}\" --background", exe.display()),
        )
        .map_err(|e| e.to_string())
    } else {
        match key.delete_value("Ananda Control") {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub connected: bool,
    pub apo_registered: bool,
    pub enhancements_disabled: bool,
}

pub fn config_dir() -> PathBuf {
    RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SOFTWARE\EqualizerAPO")
        .ok()
        .and_then(|k| k.get_value::<String, _>("ConfigPath").ok())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Program Files\EqualizerAPO\config"))
}
pub fn devices() -> Vec<Device> {
    let Ok(root) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(RENDER_KEY) else {
        return vec![];
    };
    root.enum_keys()
        .flatten()
        .filter_map(|id| {
            let k = root.open_subkey(&id).ok()?;
            let props = k.open_subkey("Properties").ok()?;
            let name = props
                .get_value::<String, _>("{a45c254e-df1c-4efd-8020-67d146a850e0},14")
                .or_else(|_| {
                    props.get_value::<String, _>("{b3f8fa53-0004-438e-9003-51a46e139bfc},6")
                })
                .unwrap_or_else(|_| id.clone());
            let fx = k.open_subkey("FxProperties").ok();
            let apo_registered = fx
                .as_ref()
                .map(|f| {
                    f.enum_values().flatten().any(|(_, v)| {
                        let s = String::from_utf16_lossy(
                            &v.bytes
                                .chunks_exact(2)
                                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                                .collect::<Vec<_>>(),
                        );
                        regex::Regex::new(r"\{[0-9A-Fa-f-]{36}\}")
                            .unwrap()
                            .find_iter(&s)
                            .any(|m| {
                                RegKey::predef(HKEY_LOCAL_MACHINE)
                                    .open_subkey(format!(
                                        r"SOFTWARE\Classes\CLSID\{}\InprocServer32",
                                        m.as_str()
                                    ))
                                    .ok()
                                    .and_then(|k| k.get_value::<String, _>("").ok())
                                    .map(|v| v.to_lowercase().contains("equalizerapo"))
                                    .unwrap_or(false)
                            })
                    })
                })
                .unwrap_or(false);
            let enhancements_disabled = fx
                .as_ref()
                .and_then(|f| {
                    f.get_value::<u32, _>("{1da5d803-d492-4edd-8c23-e0c0ffee7f0e},5")
                        .ok()
                })
                .unwrap_or(0)
                != 0;
            Some(Device {
                id,
                name,
                connected: k.get_value::<u32, _>("DeviceState").unwrap_or(0) == 1,
                apo_registered,
                enhancements_disabled,
            })
        })
        .collect()
}

pub fn powershell(script: &str, config: &Path, backup: &Path) -> Result<Output, String> {
    let out = Command::new("powershell.exe")
        .creation_flags(0x08000000)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ])
        .env("ANANDA_APO_CONFIG", config)
        .env("ANANDA_BACKUP", backup)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        let message = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if message.is_empty() {
            format!("Windows setup command exited with {}", out.status)
        } else {
            message
        });
    }
    Ok(out)
}
pub fn peace_handover(config: &Path, backup: &Path) -> Result<(), String> {
    powershell(include_str!("peace-handover.ps1"), config, backup).map(|_| ())
}
pub fn peace_restore(config: &Path, backup: &Path) -> Result<(), String> {
    powershell(include_str!("peace-restore.ps1"), config, backup).map(|_| ())
}
pub fn open_folder(path: &Path) -> Result<(), String> {
    Command::new("explorer.exe")
        .arg(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}
pub fn open_device_selector() -> Result<(), String> {
    let config = config_dir();
    let install = config.parent().ok_or("Missing APO installation")?;
    let target = if install.join("DeviceSelector.exe").exists() {
        install.join("DeviceSelector.exe")
    } else {
        install.join("Configurator.exe")
    };
    Command::new(target)
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn watch_devices(on_change: impl Fn() + Send + 'static) {
    std::thread::spawn(move || {
        let Ok(key) =
            RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(RENDER_KEY, KEY_NOTIFY)
        else {
            return;
        };
        loop {
            let result = unsafe {
                windows_sys::Win32::System::Registry::RegNotifyChangeKeyValue(
                    key.raw_handle() as _,
                    1,
                    windows_sys::Win32::System::Registry::REG_NOTIFY_CHANGE_NAME
                        | windows_sys::Win32::System::Registry::REG_NOTIFY_CHANGE_LAST_SET,
                    std::ptr::null_mut(),
                    0,
                )
            };
            if result != 0 {
                break;
            }
            on_change();
        }
    });
}

/// Elevated mode only grants the invoking account access to the fixed APO output files.
/// It never runs a webview, imports profiles, or processes user-provided script text.
pub fn access_helper(sid: &str) -> Result<(), String> {
    use std::os::windows::fs::MetadataExt;
    if !regex::Regex::new(r"^S-1-5-21-\d+-\d+-\d+-\d+$")
        .unwrap()
        .is_match(sid)
    {
        return Err("Invalid account SID".into());
    }
    let config = config_dir();
    if !config.is_absolute() || !config.join("config.txt").is_file() {
        return Err("Equalizer APO configuration was not found".into());
    }
    // Refuse redirected targets at every level, including the owned directory.
    for p in config.ancestors().chain(
        [config.join("ananda-control"), config.join("config.txt")]
            .iter()
            .map(|p| p.as_path()),
    ) {
        if let Ok(meta) = std::fs::symlink_metadata(p) {
            if meta.file_attributes() & 0x400 != 0 {
                return Err("Setup helper refuses reparse-point targets".into());
            }
        }
    }
    let owned = config.join("ananda-control");
    std::fs::create_dir_all(&owned).map_err(|e| e.to_string())?;
    let system = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\Windows"))
        .join("System32/icacls.exe");
    // Add-file on parent is needed to stage atomic config.txt replacements.
    for (path, rights) in [
        (config.clone(), "(WD)"),
        (config.join("config.txt"), "(M)"),
        (owned, "(OI)(CI)(M)"),
    ] {
        let out = Command::new(&system)
            .creation_flags(0x08000000)
            .arg(path)
            .arg("/grant")
            .arg(format!("*{sid}:{rights}"))
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            return Err(String::from_utf8_lossy(&out.stderr).into());
        }
    }
    Ok(())
}

pub fn request_access() -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    let out = Command::new("whoami.exe")
        .creation_flags(0x08000000)
        .args(["/user", "/fo", "csv", "/nh"])
        .output()
        .map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    let re = regex::Regex::new(r"S-1-5-21-\d+-\d+-\d+-\d+").unwrap();
    let sid = re
        .find(&text)
        .ok_or("Could not identify the current Windows account")?
        .as_str();
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let wide = |s: &std::ffi::OsStr| s.encode_wide().chain(Some(0)).collect::<Vec<u16>>();
    let file = wide(exe.as_os_str());
    let verb = wide(std::ffi::OsStr::new("runas"));
    let args = wide(std::ffi::OsStr::new(&format!(
        "--grant-config-access {sid}"
    )));
    let result = unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            args.as_ptr(),
            std::ptr::null(),
            0,
        )
    };
    if result as isize <= 32 {
        return Err("Setup helper could not start, or elevation was cancelled".into());
    }
    Ok(())
}
