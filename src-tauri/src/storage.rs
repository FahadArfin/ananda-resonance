use crate::model::{validate_database, Database};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Invalid destination")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temp = parent.join(format!(".ananda-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| e.to_string())?;
        f.write_all(bytes)
            .and_then(|_| f.sync_all())
            .map_err(|e| e.to_string())?;
        drop(f);
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Storage::FileSystem::{
                MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
            };
            let src: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
            let dst: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            if unsafe {
                MoveFileExW(
                    src.as_ptr(),
                    dst.as_ptr(),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
            } == 0
            {
                return Err(std::io::Error::last_os_error().to_string());
            }
        }
        #[cfg(not(windows))]
        fs::rename(&temp, path).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

pub fn json_write<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    atomic_write(
        path,
        &serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?,
    )
}

#[derive(Serialize, Deserialize)]
struct Journal {
    next: Database,
    old_active: String,
}

pub struct Store {
    pub dir: PathBuf,
    pub db: Database,
    pub startup_error: Option<String>,
}
impl Store {
    pub fn open(dir: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join("database.json");
        let mut startup_error = None;
        let db = if path.exists() {
            match fs::read(&path)
                .map_err(|e| e.to_string())
                .and_then(|v| serde_json::from_slice::<Database>(&v).map_err(|e| e.to_string()))
                .and_then(|db| {
                    validate_database(&db)?;
                    Ok(db)
                }) {
                Ok(db) => db,
                Err(e) => {
                    let backup =
                        dir.join(format!("database-corrupt-{}.json", uuid::Uuid::new_v4()));
                    fs::copy(&path, &backup).map_err(|e| e.to_string())?;
                    startup_error=Some(format!("Saved data could not be loaded: {e}. Preserved at {}. Integration has not been changed.",backup.display()));
                    crate::model::defaults()
                }
            }
        } else {
            crate::model::defaults()
        };
        let mut store = Self {
            dir,
            db,
            startup_error,
        };
        let journal = store.dir.join("transaction.json");
        if journal.exists() {
            match fs::read(&journal).ok().and_then(|v|serde_json::from_slice::<Journal>(&v).ok()) {
                Some(j) if validate_database(&j.next).is_ok() => {
                    let actual=fs::read_to_string(Path::new(&j.next.integration.config_dir).join("ananda-control/active.txt")).unwrap_or_default();
                    if actual == j.next.integration.expected_active {
                        json_write(&store.dir.join("database.json"),&j.next)?; store.db=j.next;
                        fs::remove_file(&journal).map_err(|e|e.to_string())?;
                    } else if actual == j.old_active { fs::remove_file(&journal).map_err(|e|e.to_string())?; }
                    else { store.startup_error=Some("An interrupted update conflicts with the current APO file. Review diagnostics before repairing.".into()); }
                }
                _=> store.startup_error=Some("An unreadable transaction journal was preserved. Review configuration before repair.".into()),
            }
        }
        if !path.exists() {
            json_write(&path, &store.db)?;
        }
        Ok(store)
    }
    pub fn ownership_ok(&self) -> bool {
        let i = &self.db.integration;
        i.installed
            && fs::read_to_string(Path::new(&i.config_dir).join("config.txt"))
                .ok()
                .as_deref()
                == Some(&i.expected_root)
            && fs::read_to_string(Path::new(&i.config_dir).join("ananda-control/active.txt"))
                .ok()
                .as_deref()
                == Some(&i.expected_active)
    }
    pub fn save(&mut self, next: Database, apply: bool) -> Result<(), String> {
        validate_database(&next)?;
        let db_path = self.dir.join("database.json");
        if apply && next.integration.installed {
            if !self.ownership_ok() {
                return Err("APO configuration changed outside Ananda Control. Open Diagnostics to review and repair.".into());
            }
            let journal_path = self.dir.join("transaction.json");
            let active = Path::new(&next.integration.config_dir).join("ananda-control/active.txt");
            json_write(
                &journal_path,
                &Journal {
                    next: next.clone(),
                    old_active: self.db.integration.expected_active.clone(),
                },
            )?;
            if let Err(e) = atomic_write(&active, next.integration.expected_active.as_bytes()) {
                let _ = fs::remove_file(&journal_path);
                return Err(format!("Could not apply EQ: {e}"));
            }
            if let Err(e) = json_write(&db_path, &next) {
                if atomic_write(&active, self.db.integration.expected_active.as_bytes()).is_ok() {
                    let _ = fs::remove_file(&journal_path);
                }
                return Err(format!("Could not save selection: {e}. Recovery journal retained if rollback was unsuccessful."));
            }
            self.db = next;
            let _ = fs::remove_file(&journal_path);
        } else {
            json_write(&db_path, &next)?;
            self.db = next;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn integrated() -> (tempfile::TempDir, Store) {
        let t = tempfile::tempdir().unwrap();
        let mut s = Store::open(t.path().join("data")).unwrap();
        s.db.integration.installed = true;
        s.db.integration.config_dir = t.path().join("config").display().to_string();
        s.db.integration.expected_root = "Include: ananda-control/active.txt\n".into();
        s.db.integration.expected_active = "old".into();
        atomic_write(
            &Path::new(&s.db.integration.config_dir).join("config.txt"),
            s.db.integration.expected_root.as_bytes(),
        )
        .unwrap();
        atomic_write(
            &Path::new(&s.db.integration.config_dir).join("ananda-control/active.txt"),
            b"old",
        )
        .unwrap();
        json_write(&s.dir.join("database.json"), &s.db).unwrap();
        (t, s)
    }
    #[test]
    fn rapid_switches_end_at_last_committed() {
        let (_t, mut s) = integrated();
        for n in 0..50 {
            let mut next = s.db.clone();
            next.integration.expected_active = n.to_string();
            s.save(next, true).unwrap();
        }
        assert_eq!(
            fs::read_to_string(
                Path::new(&s.db.integration.config_dir).join("ananda-control/active.txt")
            )
            .unwrap(),
            "49"
        );
    }
    #[test]
    fn external_changes_block_writes() {
        let (_t, mut s) = integrated();
        atomic_write(
            &Path::new(&s.db.integration.config_dir).join("config.txt"),
            b"external",
        )
        .unwrap();
        assert!(s.save(s.db.clone(), true).is_err());
        assert_eq!(s.db.integration.expected_active, "old");
    }
    #[test]
    fn interrupted_commit_recovers() {
        let (_t, s) = integrated();
        let mut next = s.db.clone();
        next.active_profile = "movies".into();
        next.integration.expected_active = "new".into();
        json_write(
            &s.dir.join("transaction.json"),
            &Journal {
                next,
                old_active: "old".into(),
            },
        )
        .unwrap();
        atomic_write(
            &Path::new(&s.db.integration.config_dir).join("ananda-control/active.txt"),
            b"new",
        )
        .unwrap();
        let reopened = Store::open(s.dir).unwrap();
        assert_eq!(reopened.db.active_profile, "movies");
    }
    #[test]
    fn corrupt_database_preserved() {
        let t = tempfile::tempdir().unwrap();
        atomic_write(&t.path().join("database.json"), b"invalid").unwrap();
        let s = Store::open(t.path().to_owned()).unwrap();
        assert!(s.startup_error.is_some());
        assert!(!s.db.integration.installed);
        assert!(fs::read_dir(t.path()).unwrap().count() >= 2);
    }
    #[test]
    fn missing_device_does_not_prevent_config_commit() {
        let (_t, mut s) = integrated();
        let mut next = s.db.clone();
        next.integration.expected_active = "offline selection".into();
        s.save(next, true).unwrap();
        assert_eq!(s.db.integration.expected_active, "offline selection");
    }
    #[cfg(windows)]
    #[test]
    fn locked_destination_preserves_previous() {
        use std::os::windows::fs::OpenOptionsExt;
        let (_t, mut s) = integrated();
        let path = Path::new(&s.db.integration.config_dir).join("ananda-control/active.txt");
        let _lock = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&path)
            .unwrap();
        let mut next = s.db.clone();
        next.integration.expected_active = "new".into();
        assert!(s.save(next, true).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "old");
        assert_eq!(s.db.integration.expected_active, "old");
    }
}
