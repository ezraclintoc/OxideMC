use cliclack::progress_bar;
use serde_json::Value;
use std::error::Error;
use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::PathBuf;

pub async fn get_versions(platform: &str) -> Result<Vec<String>, String> {
    if platform == "Vanilla" {
        let json_text = reqwest::get(
        "https://raw.githubusercontent.com/liebki/MinecraftServerForkDownloads/refs/heads/main/release_vanilla_downloads.json"
        )
        .await
        .map_err(|e| format!("Failed to fetch vanilla downloads: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

        let json: Value =
            serde_json::from_str(&json_text).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        let mut versions: Vec<String> = json
            .get("server_available")
            .and_then(|v| v.as_object())
            .ok_or_else(|| "Missing or invalid 'server_available' key".to_string())?
            .keys()
            .cloned()
            .collect();

        if versions.is_empty() {
            return Err("No Vanilla versions found".to_string());
        }

        versions.sort_by(|a, b| {
            let parse =
                |v: &str| -> Vec<u32> { v.split('.').filter_map(|n| n.parse().ok()).collect() };
            parse(b).cmp(&parse(a))
        });
        Ok(versions)
    } else if platform == "Paper" {
        let json_text = reqwest::get("https://api.papermc.io/v2/projects/paper")
            .await
            .map_err(|e| format!("Failed to fetch Paper versions: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;
        let json: Value = serde_json::from_str(&json_text)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;
        let mut versions: Vec<String> = json
            .get("versions")
            .and_then(|v| v.as_array())
            .ok_or_else(|| "Missing 'versions' key".to_string())?
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .filter(|v| !v.contains("rc") && !v.contains("pre"))
            .collect();
        versions.reverse();
        Ok(versions)
    } else if platform == "Fabric" {
        let json_text = reqwest::get("https://meta.fabricmc.net/v2/versions")
            .await
            .map_err(|e| format!("Failed to fetch Fabric versions: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;
        let json: Value = serde_json::from_str(&json_text)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;
        let versions: Vec<String> = json
            .get("game")
            .and_then(|g| g.as_array())
            .ok_or_else(|| "Missing 'game' key".to_string())?
            .iter()
            .filter_map(|v| {
                if v.get("stable").and_then(|s| s.as_bool()).unwrap_or(false) {
                    v.get("version")
                        .and_then(|ver| ver.as_str())
                        .map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect();
        Ok(versions)
    } else {
        Err("Unknown platform".to_string())
    }
}

pub async fn get_jar_url(platform: &str, version: &str) -> Result<String, String> {
    if platform == "Vanilla" {
        let json_text = reqwest::get(
            "https://raw.githubusercontent.com/liebki/MinecraftServerForkDownloads/refs/heads/main/release_vanilla_downloads.json"
        )
        .await
        .map_err(|e| format!("Failed to fetch vanilla downloads: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

        let json: Value =
            serde_json::from_str(&json_text).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        json.get("server_available")
            .and_then(|v| v.get(version))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| format!("No download URL found for Vanilla {}", version))
    } else if platform == "Paper" {
        let json_text = reqwest::get(&format!(
            "https://fill.papermc.io/v3/projects/paper/versions/{}/builds/latest",
            version
        ))
        .await
        .map_err(|e| format!("Failed to fetch Paper build info: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

        let json: Value = serde_json::from_str(&json_text)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;

        json.get("downloads")
            .and_then(|d| d.get("server:default"))
            .and_then(|s| s.get("url"))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| format!("No download URL found for Paper {}", version))
    } else if platform == "Fabric" {
        let json_text = reqwest::get(&format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}",
            version
        ))
        .await
        .map_err(|e| format!("Failed to fetch Fabric loader info: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

        let json: Value = serde_json::from_str(&json_text)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;

        let fabric_version = json
            .as_array()
            .and_then(|a| a.first())
            .and_then(|v| v.get("loader"))
            .and_then(|l| l.get("version"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| "Failed to find Fabric loader version".to_string())?;

        Ok(format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}/{}/1.1.0/server/jar",
            version, fabric_version
        ))
    } else if platform == "Forge" {
        todo!("Forge not implemented");
    } else {
        Err("Unknown Platform".to_string())
    }
}

pub async fn download_url(url: &str, dir: &PathBuf, filename: &str) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let mut res = client.get(url).send().await?;

    if !res.status().is_success() {
        return Err(format!("Server returned error: {}", res.status()).into());
    }

    let total_size = res.content_length().unwrap_or(0);

    create_dir_all(&dir)?;

    let file_path = dir.join(filename);
    let mut file = File::create(&file_path)?;

    let pb = progress_bar(total_size.max(1));
    pb.start(format!("Downloading {}", filename));

    let mut downloaded: u64 = 0;

    while let Some(chunk) = res.chunk().await? {
        file.write_all(&chunk)?;
        downloaded += chunk.len() as u64;
        if total_size > 0 {
            pb.set_message(format!(
                "{:.2} MB / {:.2} MB",
                downloaded as f64 / 1_048_576.0,
                total_size as f64 / 1_048_576.0
            ));
            pb.inc(chunk.len() as u64);
        } else {
            pb.set_message(format!(
                "{:.2} MB downloaded",
                downloaded as f64 / 1_048_576.0
            ));
        }
    }

    pb.stop(format!("Finished downloading to {:?}", file_path.display()));
    Ok(())
}

pub fn convert_to_items(input: &[String]) -> Vec<(String, String, String)> {
    input
        .iter()
        .map(|v| (v.clone(), v.clone(), String::new()))
        .collect()
}
