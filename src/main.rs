use cliclack::{confirm, input, intro, outro, progress_bar, select};
use reqwest::blocking::Client;
use serde_json::Value;
use std::error::Error;
use std::fs::{create_dir_all, File};
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

fn expand_path(path: &str) -> PathBuf {
    let p = if path.starts_with('~') {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        PathBuf::from(home).join(path.trim_start_matches('~').trim_start_matches('/'))
    } else {
        PathBuf::from(path)
    };

    if p.is_relative() {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(&p)
            .canonicalize()
            .unwrap_or_else(|_| p)
    } else {
        p
    }
}

fn main() {
    let _ = intro("Setting up your Minecraft Server");

    let setup_difficulty: String = select("How do you want to set up your server?")
        .item(
            "easy",
            "Easy (Recommended)",
            "Minimal configuration, just the basics!",
        )
        .item("advanced", "Advanced", "More configuration options!")
        .interact()
        .unwrap()
        .to_string();

    let server_name: String = input("What do you want to name your server?")
        .default_input("A Minecraft Server")
        .required(true)
        .interact()
        .unwrap();

    let mut server_dir: String = "~/minecraft_server".to_string();
    let mut server_port: u16 = 25565;

    if setup_difficulty != "easy" {
        server_dir = input("Where do you want to save your server?")
            .default_input("~/minecraft_server")
            .required(true)
            .interact()
            .unwrap();

        server_port = input("Which port do you want to use?")
            .default_input("25565")
            .required(true)
            .validate(|input: &String| {
                if input.parse::<u16>().is_ok() {
                    Ok(())
                } else {
                    Err("Please enter a valid port number (0-65535)".to_string())
                }
            })
            .interact()
            .unwrap();
    }

    let platform = select("Which software do you want to use?")
        .item("Vanilla", "Vanilla", "")
        .item(
            "Paper",
            "Paper (Recommended)",
            "Has plugins support and better performance!",
        )
        .item("Fabric", "Fabric", "Has mods support!")
        .interact()
        .unwrap();

    let version = select("Which version do you want to use?")
        .items(&convert_to_items(&get_versions(&platform).unwrap()))
        .interact()
        .unwrap();

    let start_after_download = confirm("Do you want to start the server after downloading?")
        .interact()
        .unwrap();

    let jar_url = get_jar_url(&platform, &version);
    let _ = download(&jar_url, &server_dir, "server.jar");

    if start_after_download {
        if confirm("Do you accept EULA?").interact().unwrap() {
            std::fs::write(expand_path(&server_dir).join("eula.txt"), "eula=true")
                .expect("Failed to write EULA file");
        } else {
            println!("You must accept the EULA to start the server. Exiting...");
            std::process::exit(0);
        }

        std::fs::write(
            expand_path(&server_dir).join("server.properties"),
            format!("server-port={}", server_port),
        )
        .expect("Failed to write server properties file");

        let mut cmd = std::process::Command::new("java");
        cmd.arg("-Xmx1024M")
            .arg("-Xms1024M")
            .arg("-jar")
            .arg("server.jar")
            .arg("nogui")
            .current_dir(expand_path(&server_dir))
            .spawn()
            .expect("Failed to start the server");
    }

    let _ = outro("You're all set!");

    println!("Server Name: {}", server_name);
    println!("Server Directory: {}", server_dir);
    println!("Server Port: {}", server_port);
}

fn download(url: &str, dir: &str, filename: &str) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let mut res = client.get(url).send()?;

    if !res.status().is_success() {
        return Err(format!("Server returned error: {}", res.status()).into());
    }

    let total_size = res.content_length().ok_or("Failed to get content length")?;

    let directory_path = expand_path(dir);
    create_dir_all(&directory_path)?;

    let file_path = directory_path.join(filename);
    let mut file = File::create(&file_path)?;

    let pb = progress_bar(total_size);
    pb.start(format!("Downloading {}", filename));

    let mut downloaded: u64 = 0;
    let mut buffer = vec![0u8; 8192];

    while let Ok(bytes_read) = res.read(&mut buffer) {
        if bytes_read == 0 {
            break;
        }
        file.write_all(&buffer[..bytes_read])?;
        downloaded += bytes_read as u64;
        pb.set_message(format!(
            "{:.2} MB / {:.2} MB",
            downloaded as f64 / 1_048_576.0,
            total_size as f64 / 1_048_576.0
        ));
        pb.inc(bytes_read as u64);
    }

    pb.stop(format!("Finished downloading to {:?}", file_path.display()));
    Ok(())
}

fn get_jar_url(platform: &str, version: &str) -> String {
    if platform == "Vanilla" {
        todo!("Vanilla not impllemented yet")
    } else if platform == "Paper" {
        let json: Value = serde_json::from_str(
            &reqwest::blocking::get(&format!(
                "https://fill.papermc.io/v3/projects/paper/versions/{}/builds/latest",
                version
            ))
            .unwrap()
            .text()
            .unwrap(),
        )
        .unwrap();
        json.get("downloads")
            .unwrap()
            .get("server:default")
            .unwrap()
            .get("url")
            .unwrap()
            .to_string()
            .trim_matches('"')
            .to_string()
    } else if platform == "Fabric" {
        let json: Value = serde_json::from_str(
            &reqwest::blocking::get(&format!(
                "https://meta.fabricmc.net/v2/versions/loader/{}",
                version
            ))
            .unwrap()
            .text()
            .unwrap(),
        )
        .unwrap();
        let fabric_version = json.as_array().unwrap()[0]
            .get("loader")
            .unwrap()
            .get("version")
            .unwrap()
            .to_string()
            .trim_matches('"')
            .to_string();

        let fabric_url: String = format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}/{}/1.1.0/server/jar",
            version, fabric_version
        );

        fabric_url
    } else {
        panic!("Unknown platform");
    }
}

fn convert_to_items(input: &[String]) -> Vec<(String, String, String)> {
    input
        .iter()
        .map(|v| (v.clone(), v.clone(), String::new()))
        .collect()
}

fn get_versions(platform: &str) -> Result<Vec<String>, String> {
    if platform == "Vanilla" {
        todo!("Vanilla not impllemented yet")
    } else if platform == "Paper" {
        let json = reqwest::blocking::get("https://api.papermc.io/v2/projects/paper")
            .unwrap()
            .text()
            .unwrap();
        let json: serde_json::Value = serde_json::from_str(&json).unwrap();
        let mut versions: Vec<String> = json
            .get("versions")
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .filter(|v| !v.contains("rc") && !v.contains("pre"))
            .collect();
        versions.reverse();
        Ok(versions)
    } else if platform == "Fabric" {
        let json = reqwest::blocking::get("https://meta.fabricmc.net/v2/versions")
            .unwrap()
            .text()
            .unwrap();
        let json: serde_json::Value = serde_json::from_str(&json).unwrap();
        let versions: Vec<String> = json
            .get("game")
            .and_then(|g| g.as_array())
            .unwrap()
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
