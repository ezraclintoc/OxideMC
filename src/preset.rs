use crate::config::{configure_file, default_server_properties, read_property};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, create_dir_all};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Preset {
    pub info: PresetInfo,
    #[serde(default)]
    pub settings: HashMap<String, HashMap<String, String>>,
    #[serde(default)]
    pub mods: PresetMods,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PresetInfo {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    pub compatible_versions: Vec<String>,
    pub compatible_platforms: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PresetMods {
    #[serde(default)]
    pub resource_packs: Vec<ModEntry>,
    #[serde(default)]
    pub datapacks: Vec<ModEntry>,
    #[serde(default)]
    pub mods: Vec<ModEntry>,
    #[serde(default)]
    pub plugins: Vec<ModEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ModEntry {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modrinth_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Check if a server version matches a pattern like "1.21.*" or an exact version.
pub fn version_matches(pattern: &str, version: &str) -> bool {
    if pattern == version {
        return true;
    }
    if pattern.contains('*') {
        let prefix = pattern.trim_end_matches('*').trim_end_matches('.');
        let version_parts: Vec<&str> = version.split('.').collect();
        let prefix_parts: Vec<&str> = prefix.split('.').collect();
        if version_parts.len() < prefix_parts.len() {
            return false;
        }
        prefix_parts
            .iter()
            .zip(version_parts.iter())
            .all(|(p, v)| p == v)
    } else {
        false
    }
}

/// Scan installed content directories and build the PresetMods struct.
pub fn scan_mods(dir: &PathBuf) -> PresetMods {
    let scan_dir = |subdir: &str| -> Vec<ModEntry> {
        let path = dir.join(subdir);
        if !path.exists() {
            return vec![];
        }
        fs::read_dir(&path)
            .unwrap_or_else(|_| fs::read_dir(".").unwrap())
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().into_string().ok()?;
                Some(ModEntry {
                    name,
                    modrinth_id: None,
                    url: None,
                    version: None,
                })
            })
            .collect()
    };

    let level_name = read_property(dir, "server.properties", "level-name")
        .unwrap_or_else(|_| "world".to_string());
    let datapacks_dir = format!("{}/datapacks", level_name);

    PresetMods {
        resource_packs: scan_dir("resourcepacks"),
        datapacks: scan_dir(&datapacks_dir),
        mods: scan_dir("mods"),
        plugins: scan_dir("plugins"),
    }
}

/// Build a Preset from the current server state, only including settings that differ from defaults.
pub fn build_preset(dir: &PathBuf, platform: &str, version: &str) -> Preset {
    let defaults = default_server_properties();
    let mut changed: HashMap<String, String> = HashMap::new();

    for key in defaults.keys() {
        if let Ok(val) = read_property(dir, "server.properties", key) {
            if let Some(default_val) = defaults.get(key) {
                if val != *default_val {
                    changed.insert(key.to_string(), val);
                }
            }
        }
    }

    let mut settings = HashMap::new();
    if !changed.is_empty() {
        settings.insert("server.properties".to_string(), changed);
    }

    let name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("server")
        .to_string();

    Preset {
        info: PresetInfo {
            name,
            description: String::new(),
            author: String::new(),
            compatible_versions: vec![version.to_string()],
            compatible_platforms: vec![platform.to_string()],
        },
        settings,
        mods: scan_mods(dir),
    }
}

/// Auto-save the current server state to preset.json in the server directory.
pub fn auto_save_preset(dir: &PathBuf, platform: &str, version: &str) {
    let preset = build_preset(dir, platform, version);
    let path = dir.join("preset.json");
    if let Ok(json) = serde_json::to_string_pretty(&preset) {
        let _ = fs::write(&path, json);
    }
}

/// Save (export) the current preset.json to a user-chosen path.
pub fn save_preset(dir: &PathBuf, dest: &PathBuf) -> Result<PathBuf, Box<dyn Error>> {
    let src = dir.join("preset.json");
    if !src.exists() {
        return Err("No preset.json found — configure the server first".into());
    }
    create_dir_all(dest.parent().unwrap_or(dest))?;
    fs::copy(&src, dest)?;
    Ok(dest.clone())
}

/// Load a preset file, check version/platform compatibility, and apply settings.
pub fn load_preset(
    dir: &PathBuf,
    preset_path: &PathBuf,
    current_platform: &str,
    current_version: &str,
) -> Result<(), Box<dyn Error>> {
    let text = fs::read_to_string(preset_path)?;
    let preset: Preset = serde_json::from_str(&text)?;

    // Check platform compatibility
    if !preset.info.compatible_platforms.is_empty()
        && !preset
            .info
            .compatible_platforms
            .iter()
            .any(|p| p.eq_ignore_ascii_case(current_platform))
    {
        return Err(format!(
            "Preset is for {:?}, but this server runs {}",
            preset.info.compatible_platforms, current_platform
        )
        .into());
    }

    // Check version compatibility (supports wildcards like "1.21.*")
    if !preset.info.compatible_versions.is_empty()
        && !preset
            .info
            .compatible_versions
            .iter()
            .any(|v| version_matches(v, current_version))
    {
        return Err(format!(
            "Preset is for versions {:?}, but this server is {}",
            preset.info.compatible_versions, current_version
        )
        .into());
    }

    // Apply settings
    for (file, props) in &preset.settings {
        for (key, val) in props {
            configure_file(dir, file, key, val).unwrap();
        }
    }

    Ok(())
}

/// List saved preset files from a directory.
pub fn list_presets(presets_dir: &PathBuf) -> Result<Vec<String>, Box<dyn Error>> {
    if !presets_dir.exists() {
        return Ok(vec![]);
    }
    let mut names: Vec<String> = fs::read_dir(presets_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
        .filter_map(|p| p.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string()))
        .collect();
    names.sort();
    Ok(names)
}
