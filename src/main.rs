use cliclack::{input, intro, outro, progress_bar, select};
use futures_util::StreamExt;
use reqwest::{Client, blocking::get};
use serde_json::Value;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf; // This brings the .next() method into scope

fn main() {
    intro("Setting up your Minecraft Server");

    let setup_difficulty = select("How do you want to set up your server?")
        .item(
            "easy",
            "Easy (Recommended)",
            "Minimal configuration, just the basics!",
        )
        .item("advanced", "Advanced", "More configuration options!")
        .interact()
        .unwrap();

    let server_name: String = input("What do you want to name your server?")
        .default_input("A Minecraft Server")
        .validate(|input: &String| {
            if input.trim().is_empty() {
                Err("Server name cannot be empty".to_string())
            } else {
                Ok(())
            }
        })
        .interact()
        .unwrap();

    if setup_difficulty != "easy" {
        let server_dir: String = input("Where do you want to save your server?")
            .default_input("./minecraft_server")
            .validate(|input: &String| {
                if input.trim().is_empty() {
                    Err("Server directory cannot be empty".to_string())
                } else {
                    Ok(())
                }
            })
            .interact()
            .unwrap();

        let server_port: u16 = input("Which port do you want to use?")
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

    let url = get_download_url(&platform, &version)
        .trim_matches('"')
        .to_string();

    download(&url).unwrap();

    outro("You're all set!");
}

#[tokio::main]
async fn download(url: &str) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let res = client.get(url).send().await?;

    let total_size = res.content_length().ok_or("Failed to get content length")?;

    let path = "/home/ezra/Downloads/Paper.jar";
    let mut file = File::create(path)?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    let pb = progress_bar(total_size);
    pb.start("Downloading...");

    while let Some(item) = stream.next().await {
        // We handle the result explicitly to avoid the [u8] size error
        let chunk = match item {
            Ok(c) => c,
            Err(e) => return Err(format!("Download error: {}", e).into()),
        };

        file.write_all(&chunk)?;

        downloaded += chunk.len() as u64;
        pb.set_message(format!("Downloaded: {}/{} bytes", downloaded, total_size));
        pb.inc(chunk.len() as u64);
    }

    pb.stop("Download complete!");
    Ok(())
}

fn get_download_url(platform: &str, version: &String) -> String {
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
