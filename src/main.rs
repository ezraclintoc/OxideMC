use cliclack::{input, intro, outro, progress_bar, confirm, select};
use futures_util::StreamExt;
use reqwest::{Client, blocking::get};
use serde_json::{Value};
use std::error::Error;
use std::fs::{File, create_dir_all};
use std::io::Write;
use std::path::PathBuf; // This brings the .next() method into scope

fn main() {
    // Intro
    // Setup Difficulty
    // Server Name
    // Server Directory (if advanced)
    // Server Port (if advanced)
    // Platform
    // Version
    // Start after download
    // Download
    // Eula
    // Outro

    intro("Setting up your Minecraft Server");

    let setup_difficulty: String = select("How do you want to set up your server?")
        .item(
            "easy",
            "Easy (Recommended)",
            "Minimal configuration, just the basics!",
        )
        .item("advanced", "Advanced", "More configuration options!")
        .interact().unwrap().to_string();

    let server_name: String = input("What do you want to name your server?")
        .default_input("A Minecraft Server")
        .validate(|input: &String| {
            if input.trim().is_empty() {
                Err("Server name cannot be empty".to_string())
            } else {
                Ok(())
            }
        })
        .interact().unwrap();

    let mut server_dir: String = "~/minecraft_server".to_string();
    let mut server_port: u16 = 25565;

    if setup_difficulty != "easy" {
        server_dir = input("Where do you want to save your server?")
            .default_input("~/minecraft_server")
            .validate(|input: &String| {
                if input.trim().is_empty() {
                    Err("Server directory cannot be empty".to_string())
                } else {
                    Ok(())
                }
            })
            .interact()
            .unwrap();

        server_port = input("Which port do you want to use?")
            .default_input("25565")
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
        .item("Paper", "Paper (Recommended)", "Has plugins support and better performance!")
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
    let _ = download(&jar_url, &server_dir, &"server.jar".to_string());

    if start_after_download {
        todo!("Start server after download not implemented yet");
    }


    outro("You're all set!");
}


#[tokio::main]
async fn download(url: &str, dir: &str, filename: &str) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let res = client.get(url).send().await?;

    // 1. Check response status before proceeding
    if !res.status().is_success() {
        return Err(format!("Server returned error: {}", res.status()).into());
    }

    let total_size = res.content_length().ok_or("Failed to get content length")?;

    // 2. Robust Path Handling
    let expanded_dir = shellexpand::full(dir)?.into_owned();
    let directory_path = PathBuf::from(expanded_dir);
    
    // Create the directory if it's missing
    create_dir_all(&directory_path)?;
    
    let file_path = directory_path.join(filename);

    // 3. File Setup
    let mut file = File::create(&file_path)?;
    let mut stream = res.bytes_stream();
    let mut downloaded: u64 = 0;

    // 4. Progress Bar Setup
    let pb = progress_bar(total_size);
    pb.start(format!("Downloading {}", filename));

    while let Some(item) = stream.next().await {
        // Concisely handle the stream result
        let chunk = item?; 
        
        file.write_all(&chunk)?;

        downloaded += chunk.len() as u64;
        pb.set_message(format!("{:.2} MB / {:.2} MB", 
            downloaded as f64 / 1_048_576.0, 
            total_size as f64 / 1_048_576.0
        ));
        pb.inc(chunk.len() as u64);
    }

    pb.stop(format!("Finished downloading to {:?}", file_path.display()));
    Ok(())
}

fn get_jar_url(platform: &str, version: &String) -> String {
    if platform == "Vanilla" {
        todo!("Vanilla not impllemented yet")
    } else if platform == "Paper" {
        let json: Value = serde_json::from_str(
            &get(&format!(
                "https://fill.papermc.io/v3/projects/paper/versions/{}/builds/latest",
                version
            ))
            .unwrap()
            .text()
            .unwrap(),
        )
        .unwrap();
        return json
            .get("downloads")
            .unwrap()
            .get("server:default")
            .unwrap()
            .get("url")
            .unwrap()
            .to_string()
            .trim_matches('"')
            .to_string();
    } else if platform == "Fabric" {
        todo!("Fabric not impllemented yet")
    } else {
        panic!("Unknown platform");
    }
}

fn convert_to_items(input: &[String]) -> Vec<(String, String, String)> {
    input
        .iter()
        .map(|v| (v.clone(), v.clone(), String::new()))
        .collect() // This puts the items into a Vec on the heap
}

fn get_versions(platform: &str) -> Result<Vec<String>, String> {
    if platform == "Vanilla" {
        todo!("Vanilla not impllemented yet")
    } else if platform == "Paper" {
        let json = get("https://api.papermc.io/v2/projects/paper")
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
        todo!("Fabric not impllemented yet")
    } else {
        Err("Unknown platform".to_string())
    }
}
