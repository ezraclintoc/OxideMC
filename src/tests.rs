use super::*;
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

// ── version_matches ──────────────────────────────────────────────

#[test]
fn version_matches_exact() {
    assert!(version_matches("1.21.4", "1.21.4"));
}

#[test]
fn version_matches_exact_mismatch() {
    assert!(!version_matches("1.21.4", "1.21.3"));
}

#[test]
fn version_matches_wildcard_patch() {
    assert!(version_matches("1.21.*", "1.21.0"));
    assert!(version_matches("1.21.*", "1.21.4"));
    assert!(version_matches("1.21.*", "1.21.99"));
}

#[test]
fn version_matches_wildcard_minor() {
    assert!(version_matches("1.*", "1.21.4"));
    assert!(version_matches("1.*", "1.0"));
    assert!(version_matches("1.*", "1.20.1"));
}

#[test]
fn version_matches_wildcard_no_match() {
    assert!(!version_matches("1.21.*", "1.20.4"));
    assert!(!version_matches("1.21.*", "2.21.0"));
    assert!(!version_matches("1.*", "2.0.0"));
}

#[test]
fn version_matches_wildcard_short_version() {
    assert!(!version_matches("1.21.*", "1"));
}

#[test]
fn version_matches_no_wildcard_no_exact() {
    assert!(!version_matches("1.21.4", "1.21.5"));
}

// ── default_server_properties ────────────────────────────────────

#[test]
fn defaults_contain_expected_keys() {
    let defaults = default_server_properties();
    assert_eq!(defaults.get("difficulty"), Some(&"easy"));
    assert_eq!(defaults.get("gamemode"), Some(&"survival"));
    assert_eq!(defaults.get("max-players"), Some(&"20"));
    assert_eq!(defaults.get("server-port"), Some(&"25565"));
    assert_eq!(defaults.get("online-mode"), Some(&"true"));
    assert_eq!(defaults.get("pvp"), Some(&"true"));
    assert_eq!(defaults.get("motd"), Some(&"A Minecraft Server"));
}

// ── convert_to_items ─────────────────────────────────────────────

#[test]
fn convert_to_items_basic() {
    let input = vec!["a".to_string(), "b".to_string()];
    let items = convert_to_items(&input);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0], ("a".to_string(), "a".to_string(), String::new()));
    assert_eq!(items[1], ("b".to_string(), "b".to_string(), String::new()));
}

#[test]
fn convert_to_items_empty() {
    let input: Vec<String> = vec![];
    let items = convert_to_items(&input);
    assert!(items.is_empty());
}

// ── configure_file / read_property ───────────────────────────────

fn write_test_properties(dir: &std::path::Path) {
    let content = "# Minecraft server properties\ndifficulty=easy\ngamemode=survival\nmax-players=20\nmotd=A Minecraft Server\n";
    fs::write(dir.join("server.properties"), content).unwrap();
}

#[test]
fn configure_file_updates_value() {
    let tmp = TempDir::new().unwrap();
    write_test_properties(tmp.path());

    configure_file(
        &tmp.path().to_path_buf(),
        "server.properties",
        "difficulty",
        "hard",
    )
    .unwrap();

    let val =
        read_property(&tmp.path().to_path_buf(), "server.properties", "difficulty").unwrap();
    assert_eq!(val, "hard");
}

#[test]
fn configure_file_leaves_other_values() {
    let tmp = TempDir::new().unwrap();
    write_test_properties(tmp.path());

    configure_file(
        &tmp.path().to_path_buf(),
        "server.properties",
        "difficulty",
        "hard",
    )
    .unwrap();

    let motd = read_property(&tmp.path().to_path_buf(), "server.properties", "motd").unwrap();
    assert_eq!(motd, "A Minecraft Server");
}

#[test]
fn read_property_not_found() {
    let tmp = TempDir::new().unwrap();
    write_test_properties(tmp.path());

    let result = read_property(
        &tmp.path().to_path_buf(),
        "server.properties",
        "nonexistent",
    );
    assert!(result.is_err());
}

// ── list_entries ─────────────────────────────────────────────────

#[test]
fn list_entries_empty_dir() {
    let tmp = TempDir::new().unwrap();
    let entries = list_entries(&tmp.path().to_path_buf()).unwrap();
    assert!(entries.is_empty());
}

#[test]
fn list_entries_with_files() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("b.jar"), "").unwrap();
    fs::write(tmp.path().join("a.jar"), "").unwrap();

    let entries = list_entries(&tmp.path().to_path_buf()).unwrap();
    assert_eq!(entries, vec!["a.jar", "b.jar"]);
}

#[test]
fn list_entries_nonexistent_dir() {
    let path = PathBuf::from("/tmp/oxidemc_test_nonexistent_dir_12345");
    let entries = list_entries(&path).unwrap();
    assert!(entries.is_empty());
}

// ── Preset serialization / deserialization ───────────────────────

#[test]
fn preset_round_trip() {
    let preset = Preset {
        info: PresetInfo {
            name: "Test".to_string(),
            description: "A test preset".to_string(),
            author: "OxideMC".to_string(),
            compatible_versions: vec!["1.21.*".to_string()],
            compatible_platforms: vec!["Paper".to_string(), "Fabric".to_string()],
        },
        settings: {
            let mut s = HashMap::new();
            let mut props = HashMap::new();
            props.insert("difficulty".to_string(), "hard".to_string());
            props.insert("max-players".to_string(), "50".to_string());
            s.insert("server.properties".to_string(), props);
            s
        },
        mods: PresetMods {
            plugins: vec![ModEntry {
                name: "EssentialsX".to_string(),
                modrinth_id: Some("abc123".to_string()),
                url: None,
                version: Some("2.20.0".to_string()),
            }],
            ..Default::default()
        },
    };

    let json = serde_json::to_string_pretty(&preset).unwrap();
    let deserialized: Preset = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.info.name, "Test");
    assert_eq!(deserialized.info.compatible_versions, vec!["1.21.*"]);
    assert_eq!(
        deserialized.settings["server.properties"]["difficulty"],
        "hard"
    );
    assert_eq!(deserialized.mods.plugins.len(), 1);
    assert_eq!(deserialized.mods.plugins[0].name, "EssentialsX");
    assert!(deserialized.mods.mods.is_empty());
}

#[test]
fn preset_deserialize_minimal() {
    let json = r#"{
        "info": {
            "name": "Minimal",
            "compatible_versions": ["1.21.4"],
            "compatible_platforms": ["Vanilla"]
        }
    }"#;
    let preset: Preset = serde_json::from_str(json).unwrap();
    assert_eq!(preset.info.name, "Minimal");
    assert!(preset.settings.is_empty());
    assert!(preset.mods.mods.is_empty());
    assert!(preset.mods.plugins.is_empty());
}

#[test]
fn preset_skip_serializing_none_fields() {
    let entry = ModEntry {
        name: "test".to_string(),
        modrinth_id: None,
        url: None,
        version: None,
    };
    let json = serde_json::to_string(&entry).unwrap();
    assert!(!json.contains("modrinth_id"));
    assert!(!json.contains("url"));
    assert!(!json.contains("version"));
}

// ── build_preset / auto_save_preset ──────────────────────────────

#[test]
fn build_preset_only_changed_settings() {
    let tmp = TempDir::new().unwrap();
    let content = "difficulty=hard\ngamemode=survival\nmax-players=20\nmotd=A Minecraft Server\npvp=true\nview-distance=10\nsimulation-distance=10\nspawn-protection=16\nlevel-type=minecraft\\:normal\nlevel-seed=\nmax-world-size=29999984\nonline-mode=true\nserver-port=25565\n";
    fs::write(tmp.path().join("server.properties"), content).unwrap();

    let preset = build_preset(&tmp.path().to_path_buf(), "Paper", "1.21.4");

    let props = &preset.settings["server.properties"];
    assert_eq!(props.get("difficulty"), Some(&"hard".to_string()));
    assert!(!props.contains_key("gamemode"));
    assert!(!props.contains_key("max-players"));
}

#[test]
fn build_preset_no_changed_settings() {
    let tmp = TempDir::new().unwrap();
    let content = "difficulty=easy\ngamemode=survival\nmax-players=20\nmotd=A Minecraft Server\npvp=true\nview-distance=10\nsimulation-distance=10\nspawn-protection=16\nlevel-type=minecraft\\:normal\nlevel-seed=\nmax-world-size=29999984\nonline-mode=true\nserver-port=25565\n";
    fs::write(tmp.path().join("server.properties"), content).unwrap();

    let preset = build_preset(&tmp.path().to_path_buf(), "Vanilla", "1.21.4");

    assert!(preset.settings.is_empty());
}

#[test]
fn auto_save_creates_preset_json() {
    let tmp = TempDir::new().unwrap();
    let content = "difficulty=hard\ngamemode=survival\n";
    fs::write(tmp.path().join("server.properties"), content).unwrap();

    auto_save_preset(&tmp.path().to_path_buf(), "Paper", "1.21.4");

    let preset_path = tmp.path().join("preset.json");
    assert!(preset_path.exists());

    let text = fs::read_to_string(&preset_path).unwrap();
    let preset: Preset = serde_json::from_str(&text).unwrap();
    assert_eq!(preset.info.compatible_platforms, vec!["Paper"]);
    assert_eq!(preset.info.compatible_versions, vec!["1.21.4"]);
}

// ── save_preset (export) ─────────────────────────────────────────

#[test]
fn save_preset_copies_file() {
    let tmp = TempDir::new().unwrap();
    fs::write(
        tmp.path().join("preset.json"),
        r#"{"info":{"name":"t","compatible_versions":[],"compatible_platforms":[]}}"#,
    )
    .unwrap();

    let dest = tmp.path().join("exports").join("my_preset.json");
    let result = save_preset(&tmp.path().to_path_buf(), &dest);
    assert!(result.is_ok());
    assert!(dest.exists());
}

#[test]
fn save_preset_no_source_errors() {
    let tmp = TempDir::new().unwrap();
    let dest = tmp.path().join("out.json");
    let result = save_preset(&tmp.path().to_path_buf(), &dest);
    assert!(result.is_err());
}

// ── load_preset ──────────────────────────────────────────────────

#[test]
fn load_preset_applies_settings() {
    let tmp = TempDir::new().unwrap();
    write_test_properties(tmp.path());

    let preset_json = r#"{
        "info": {
            "name": "Hard Mode",
            "compatible_versions": ["1.21.*"],
            "compatible_platforms": ["Paper"]
        },
        "settings": {
            "server.properties": {
                "difficulty": "hard",
                "max-players": "50"
            }
        }
    }"#;
    let preset_path = tmp.path().join("test_preset.json");
    fs::write(&preset_path, preset_json).unwrap();

    load_preset(
        &tmp.path().to_path_buf(),
        &preset_path,
        "Paper",
        "1.21.4",
    )
    .unwrap();

    assert_eq!(
        read_property(
            &tmp.path().to_path_buf(),
            "server.properties",
            "difficulty"
        )
        .unwrap(),
        "hard"
    );
    assert_eq!(
        read_property(
            &tmp.path().to_path_buf(),
            "server.properties",
            "max-players"
        )
        .unwrap(),
        "50"
    );
}

#[test]
fn load_preset_rejects_wrong_platform() {
    let tmp = TempDir::new().unwrap();
    write_test_properties(tmp.path());

    let preset_json = r#"{
        "info": {
            "name": "Fabric Only",
            "compatible_versions": ["1.21.*"],
            "compatible_platforms": ["Fabric"]
        }
    }"#;
    let preset_path = tmp.path().join("fabric_preset.json");
    fs::write(&preset_path, preset_json).unwrap();

    let result = load_preset(
        &tmp.path().to_path_buf(),
        &preset_path,
        "Paper",
        "1.21.4",
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Paper"));
}

#[test]
fn load_preset_rejects_wrong_version() {
    let tmp = TempDir::new().unwrap();
    write_test_properties(tmp.path());

    let preset_json = r#"{
        "info": {
            "name": "Old Only",
            "compatible_versions": ["1.20.*"],
            "compatible_platforms": ["Paper"]
        }
    }"#;
    let preset_path = tmp.path().join("old_preset.json");
    fs::write(&preset_path, preset_json).unwrap();

    let result = load_preset(
        &tmp.path().to_path_buf(),
        &preset_path,
        "Paper",
        "1.21.4",
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("1.21.4"));
}

#[test]
fn load_preset_accepts_wildcard_version() {
    let tmp = TempDir::new().unwrap();
    write_test_properties(tmp.path());

    let preset_json = r#"{
        "info": {
            "name": "Wildcard",
            "compatible_versions": ["1.21.*"],
            "compatible_platforms": ["Paper"]
        },
        "settings": {
            "server.properties": {
                "difficulty": "hard"
            }
        }
    }"#;
    let preset_path = tmp.path().join("wc_preset.json");
    fs::write(&preset_path, preset_json).unwrap();

    let result = load_preset(
        &tmp.path().to_path_buf(),
        &preset_path,
        "Paper",
        "1.21.1",
    );
    assert!(result.is_ok());
}

// ── scan_mods ────────────────────────────────────────────────────

#[test]
fn scan_mods_empty_server() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("server.properties"), "level-name=world\n").unwrap();

    let mods = scan_mods(&tmp.path().to_path_buf());
    assert!(mods.mods.is_empty());
    assert!(mods.plugins.is_empty());
    assert!(mods.datapacks.is_empty());
    assert!(mods.resource_packs.is_empty());
}

#[test]
fn scan_mods_finds_plugins() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("server.properties"), "level-name=world\n").unwrap();

    let plugins_dir = tmp.path().join("plugins");
    fs::create_dir(&plugins_dir).unwrap();
    fs::write(plugins_dir.join("EssentialsX.jar"), "").unwrap();
    fs::write(plugins_dir.join("Vault.jar"), "").unwrap();

    let mods = scan_mods(&tmp.path().to_path_buf());
    assert_eq!(mods.plugins.len(), 2);
    let names: Vec<&str> = mods.plugins.iter().map(|m| m.name.as_str()).collect();
    assert!(names.contains(&"EssentialsX.jar"));
    assert!(names.contains(&"Vault.jar"));
}

// ── list_presets ─────────────────────────────────────────────────

#[test]
fn list_presets_finds_json_files() {
    let tmp = TempDir::new().unwrap();
    let presets_dir = tmp.path().join("presets");
    fs::create_dir(&presets_dir).unwrap();
    fs::write(presets_dir.join("survival.json"), "{}").unwrap();
    fs::write(presets_dir.join("creative.json"), "{}").unwrap();
    fs::write(presets_dir.join("readme.txt"), "not a preset").unwrap();

    let names = list_presets(&presets_dir).unwrap();
    assert_eq!(names, vec!["creative", "survival"]);
}

#[test]
fn list_presets_empty() {
    let tmp = TempDir::new().unwrap();
    let presets_dir = tmp.path().join("presets");
    fs::create_dir(&presets_dir).unwrap();

    let names = list_presets(&presets_dir).unwrap();
    assert!(names.is_empty());
}

// ── oxide config ─────────────────────────────────────────────────

#[test]
fn write_and_read_oxide_config() {
    let tmp = TempDir::new().unwrap();
    write_oxide_config(&tmp.path().to_path_buf(), "test_key", "test_value").unwrap();

    let val = read_oxide_config(&tmp.path().to_path_buf(), "test_key").unwrap();
    assert_eq!(val, "test_value");
}

#[test]
fn read_oxide_config_missing_key() {
    let tmp = TempDir::new().unwrap();
    write_oxide_config(&tmp.path().to_path_buf(), "other", "val").unwrap();

    let result = read_oxide_config(&tmp.path().to_path_buf(), "missing");
    assert!(result.is_err());
}

// ── get_platform ─────────────────────────────────────────────────

#[test]
fn get_platform_detects_paper() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("server.jar"), "").unwrap();
    fs::create_dir(tmp.path().join("plugins")).unwrap();

    let platform = get_platform(&tmp.path().to_path_buf()).unwrap();
    assert_eq!(platform, "Paper");
}

#[test]
fn get_platform_detects_fabric() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("server.jar"), "").unwrap();
    fs::create_dir(tmp.path().join("mods")).unwrap();
    fs::create_dir(tmp.path().join(".fabric")).unwrap();

    let platform = get_platform(&tmp.path().to_path_buf()).unwrap();
    assert_eq!(platform, "Fabric");
}

#[test]
fn get_platform_detects_forge() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("server.jar"), "").unwrap();
    fs::create_dir(tmp.path().join("mods")).unwrap();

    let platform = get_platform(&tmp.path().to_path_buf()).unwrap();
    assert_eq!(platform, "Forge");
}

#[test]
fn get_platform_detects_vanilla() {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join("server.jar"), "").unwrap();

    let platform = get_platform(&tmp.path().to_path_buf()).unwrap();
    assert_eq!(platform, "Vanilla");
}

#[test]
fn get_platform_no_jar_errors() {
    let tmp = TempDir::new().unwrap();
    let result = get_platform(&tmp.path().to_path_buf());
    assert!(result.is_err());
}
