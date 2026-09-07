use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Filter {
    pub id: String,
    pub kind: String,
    pub enabled: bool,
    pub frequency: f64,
    pub gain: f64,
    pub q: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Point {
    pub frequency: f64,
    pub gain: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Profile {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub icon: String,
    pub headphone_id: String,
    pub preamp: f64,
    pub filters: Vec<Filter>,
    #[serde(default)]
    pub graphic_eq: Vec<Point>,
    pub provenance: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Headphone {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default = "agent_default")]
    pub agent_control: bool,
    pub start_with_windows: bool,
    pub start_in_tray: bool,
    pub restore_last_profile: bool,
    pub close_to_tray: bool,
    pub notifications: bool,
    pub shortcuts: BTreeMap<String, String>,
    pub quick_shortcut: String,
}
fn agent_default() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            agent_control: true,
            start_with_windows: true,
            start_in_tray: true,
            restore_last_profile: true,
            close_to_tray: true,
            notifications: true,
            shortcuts: [
                ("music", "1"),
                ("movies", "2"),
                ("gaming", "3"),
                ("fps", "4"),
                ("voice", "5"),
                ("stock", "0"),
            ]
            .into_iter()
            .map(|(id, key)| (id.into(), format!("Ctrl+Alt+{key}")))
            .collect(),
            quick_shortcut: "Ctrl+Alt+E".into(),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Integration {
    pub installed: bool,
    pub config_dir: String,
    pub device_id: String,
    pub backup_dir: Option<String>,
    pub expected_root: String,
    pub expected_active: String,
    pub audio_verified: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Database {
    pub schema_version: u32,
    pub profiles: Vec<Profile>,
    pub headphones: Vec<Headphone>,
    pub selected_headphone: String,
    pub active_profile: String,
    pub last_enabled: String,
    pub settings: Settings,
    pub integration: Integration,
}

pub fn validate_profile(p: &Profile) -> Result<(), String> {
    if p.schema_version != 1 {
        return Err("Unsupported profile version".into());
    }
    if p.name.trim().is_empty() || p.name.chars().count() > 80 || p.name.contains(['\r', '\n']) {
        return Err("Profile name must contain 1–80 characters on one line".into());
    }
    if !p.preamp.is_finite() || !(-60.0..=20.0).contains(&p.preamp) {
        return Err("Preamp must be between −60 and +20 dB".into());
    }
    if p.filters.len() > 64 || p.graphic_eq.len() > 2048 {
        return Err("Profile is too large (64 filters or 2048 graphic points maximum)".into());
    }
    let mut ids = std::collections::HashSet::new();
    for f in &p.filters {
        if !ids.insert(&f.id) {
            return Err("Filter IDs must be unique".into());
        }
        if !["PK", "LSC", "HSC", "HPQ", "LPQ"].contains(&f.kind.as_str()) {
            return Err(format!("Unsupported filter type: {}", f.kind));
        }
        if !f.frequency.is_finite() || !(10.0..=22000.0).contains(&f.frequency) {
            return Err("Frequency must be between 10 and 22000 Hz".into());
        }
        if !f.gain.is_finite() || !(-30.0..=30.0).contains(&f.gain) {
            return Err("Gain must be between −30 and +30 dB".into());
        }
        if !f.q.is_finite() || !(0.1..=30.0).contains(&f.q) {
            return Err("Q must be between 0.1 and 30".into());
        }
    }
    let mut last = 0.0;
    for point in &p.graphic_eq {
        if !point.frequency.is_finite()
            || !(10.0..=22000.0).contains(&point.frequency)
            || point.frequency <= last
            || !point.gain.is_finite()
            || !(-60.0..=30.0).contains(&point.gain)
        {
            return Err("GraphicEQ points must have increasing frequencies (10–22000 Hz) and finite gains (−60 to +30 dB)".into());
        }
        last = point.frequency;
    }
    if p.id == "stock" && (p.preamp != 0.0 || !p.filters.is_empty() || !p.graphic_eq.is_empty()) {
        return Err("Stock is always a true bypass".into());
    }
    Ok(())
}

pub fn defaults() -> Database {
    let sources = [
        (
            "music",
            "Balanced Music",
            "music",
            include_str!("../presets/Ananda_Stealth_01_Balanced_Music.txt"),
            "Ananda_Stealth_01_Balanced_Music.txt",
        ),
        (
            "movies",
            "Movies / Cinematic",
            "film",
            include_str!("../presets/Ananda_Stealth_02_Movies_Fun.txt"),
            "Ananda_Stealth_02_Movies_Fun.txt",
        ),
        (
            "fps",
            "Competitive FPS",
            "target",
            include_str!("../presets/Ananda_Stealth_03_Competitive_FPS.txt"),
            "Ananda_Stealth_03_Competitive_FPS.txt",
        ),
        (
            "voice",
            "Voice / Discord",
            "mic",
            include_str!("../presets/Ananda_Stealth_04_Voice_Discord.txt"),
            "Ananda_Stealth_04_Voice_Discord.txt",
        ),
    ];
    let mut profiles: Vec<Profile> = sources
        .iter()
        .map(|(id, name, icon, text, source)| {
            let mut p =
                crate::apo::parse(text, name, source).expect("Bundled preset must validate");
            p.id = (*id).into();
            p.icon = (*icon).into();
            p
        })
        .collect();
    let mut gaming = profiles[0].clone();
    gaming.id = "gaming".into();
    gaming.name = "Gaming".into();
    gaming.icon = "gamepad".into();
    gaming.preamp = -7.0;
    gaming.provenance = "App-created • Music/Movies gain midpoint, −7 dB preamp".into();
    for (f, m) in gaming.filters.iter_mut().zip(&profiles[1].filters) {
        f.gain = (f.gain + m.gain) / 2.0;
    }
    profiles.insert(2, gaming);
    profiles.push(Profile {
        schema_version: 1,
        id: "stock".into(),
        name: "Stock / EQ Off".into(),
        icon: "power".into(),
        headphone_id: "default".into(),
        preamp: 0.0,
        filters: vec![],
        graphic_eq: vec![],
        provenance: "Unprocessed output • 0 dB preamp".into(),
    });
    Database {
        schema_version: 1,
        profiles,
        headphones: vec![Headphone {
            id: "default".into(),
            name: "My headphones".into(),
        }],
        selected_headphone: "default".into(),
        active_profile: "music".into(),
        last_enabled: "music".into(),
        settings: Settings::default(),
        integration: Integration {
            config_dir: "C:\\Program Files\\EqualizerAPO\\config".into(),
            ..Default::default()
        },
    }
}

pub fn validate_database(db: &Database) -> Result<(), String> {
    if db.schema_version != 1 {
        return Err("Unsupported database version; your data has been preserved".into());
    }
    let mut ids = std::collections::HashSet::new();
    for p in &db.profiles {
        validate_profile(p)?;
        if !ids.insert(&p.id) {
            return Err("Duplicate profile ID".into());
        }
        if !db.headphones.iter().any(|h| h.id == p.headphone_id) {
            return Err("Profile refers to a missing headphone".into());
        }
    }
    if !ids.contains(&"stock".to_string())
        || !ids.contains(&db.active_profile)
        || !ids.contains(&db.last_enabled)
        || db.last_enabled == "stock"
    {
        return Err("Missing stock, active, or last-enabled profile".into());
    }
    if !db.headphones.iter().any(|h| h.id == db.selected_headphone) {
        return Err("Missing selected headphone".into());
    }
    Ok(())
}
