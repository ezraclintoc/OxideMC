use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::PathBuf;

pub fn configure_file(dir: &PathBuf, filename: &str, name: &str, value: &str) -> Result<(), ()> {
    let path = dir.join(filename);

    if !path.exists() {
        return Err(());
    }

    let input = File::open(&path).unwrap();
    let buffered_input = BufReader::new(input);

    let temp_path = path.with_extension("tmp");

    let output = File::create(&temp_path).unwrap();
    let mut buffered_output = BufWriter::new(output);

    let mut found = false;
    for line in buffered_input.lines() {
        let line = line.unwrap();
        let trimmed = line.trim_start();

        if trimmed.starts_with(format!("{}=", name).as_str()) {
            writeln!(buffered_output, "{}={}", name, value).unwrap();
            found = true;
        } else {
            writeln!(buffered_output, "{}", line).unwrap();
        }
    }

    // If the key wasn't found in the file, append it
    if !found {
        writeln!(buffered_output, "{}={}", name, value).unwrap();
    }

    buffered_output.flush().unwrap();
    fs::rename(&temp_path, &path).unwrap();

    Ok(())
}

pub fn read_property(dir: &PathBuf, filename: &str, name: &str) -> Result<String, String> {
    let path = dir.join(filename);
    let file = File::open(&path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        let trimmed = line.trim_start();
        if trimmed.starts_with(&format!("{}=", name)) {
            return Ok(trimmed[name.len() + 1..].to_string());
        }
    }
    Err(format!("Property '{}' not found", name))
}

pub fn read_oxide_config(dir: &PathBuf, key: &str) -> Result<String, Box<dyn Error>> {
    let path = dir.join("oxidemc.json");
    let text = fs::read_to_string(&path)?;
    let json: Value = serde_json::from_str(&text)?;
    json.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Key '{}' not found", key).into())
}

pub fn write_oxide_config(dir: &PathBuf, key: &str, value: &str) -> Result<(), Box<dyn Error>> {
    let path = dir.join("oxidemc.json");
    let mut map: serde_json::Map<String, Value> = if path.exists() {
        serde_json::from_str(&fs::read_to_string(&path)?)?
    } else {
        serde_json::Map::new()
    };
    map.insert(key.to_string(), Value::String(value.to_string()));
    fs::write(&path, serde_json::to_string_pretty(&Value::Object(map))?)?;
    Ok(())
}

/// Default Minecraft server.properties values — only settings that differ are saved in presets.
pub fn default_server_properties() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        ("difficulty", "easy"),
        ("gamemode", "survival"),
        ("pvp", "true"),
        ("max-players", "20"),
        ("view-distance", "10"),
        ("simulation-distance", "10"),
        ("spawn-protection", "16"),
        ("level-type", "minecraft\\:normal"),
        ("level-seed", ""),
        ("max-world-size", "29999984"),
        ("online-mode", "true"),
        ("server-port", "25565"),
        ("motd", "A Minecraft Server"),
    ])
}
