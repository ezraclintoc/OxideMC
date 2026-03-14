use crate::config::{read_oxide_config, read_property};
use cliclack::spinner;
use std::error::Error;
use std::fs::{self, create_dir_all};
use std::path::PathBuf;
use std::process::Command;

pub fn expand_path(path: &str) -> Result<PathBuf, String> {
    let p: PathBuf;
    if path.starts_with('~') {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        p = PathBuf::from(home).join(path.trim_start_matches('~').trim_start_matches('/'));
    } else {
        p = PathBuf::from(path);
    }
    if p.exists() {
        if p.is_dir() {
            Ok(p.canonicalize().unwrap_or(p))
        } else {
            Err("The path exists but is not a directory".to_string())
        }
    } else {
        Err(format!(
            "The path '{}' does not exist. Please enter a valid directory path.",
            p.display()
        ))
    }
}

/// Lists all entries (files and directories) in a directory — used for remove menus.
pub fn list_entries(dir: &PathBuf) -> Result<Vec<String>, Box<dyn Error>> {
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut entries: Vec<String> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    entries.sort();
    Ok(entries)
}

pub fn get_platform(dir: &PathBuf) -> Result<String, String> {
    let dir =
        expand_path(dir.to_str().unwrap()).map_err(|e| format!("Failed to expand path: {}", e))?;
    let jar_path = dir.join("server.jar");
    if !jar_path.exists() {
        return Err("No server.jar found in the specified directory. Please make sure to provide a valid server directory.".to_string());
    }

    if dir.join("plugins").exists() {
        return Ok("Paper".to_string());
    } else if dir.join("mods").exists() {
        if dir.join(".fabric").exists() {
            return Ok("Fabric".to_string());
        } else {
            return Ok("Forge".to_string());
        }
    } else {
        return Ok("Vanilla".to_string());
    }
}

pub fn backup_world(dir: &PathBuf) -> Result<(), Box<dyn Error>> {
    let level_name = read_property(dir, "server.properties", "level-name")
        .unwrap_or_else(|_| "world".to_string());
    let world_dir = dir.join(&level_name);
    if !world_dir.exists() {
        return Err(format!("World directory '{}' not found", level_name).into());
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let backups_dir = read_oxide_config(dir, "backup_dir")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dir.join("backups"));
    create_dir_all(&backups_dir)?;

    let backup_name = format!("{}_{}.tar.gz", level_name, timestamp);
    let backup_path = backups_dir.join(&backup_name);

    let sp = spinner();
    sp.start(format!(
        "Backing up '{}' to {}...",
        level_name,
        backup_path.display()
    ));

    let status = Command::new("tar")
        .arg("-czf")
        .arg(&backup_path)
        .arg("-C")
        .arg(dir)
        .arg(&level_name)
        .status()?;

    if status.success() {
        sp.stop(format!("Backup saved to {}", backup_path.display()));
        Ok(())
    } else {
        sp.stop("Backup failed!".to_string());
        Err("tar command failed".into())
    }
}
