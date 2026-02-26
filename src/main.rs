use cliclack::{confirm, input, intro, outro, progress_bar, spinner, select};
use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde_json::Value;
use std::error::Error;
use std::fs::{self, create_dir_all, File};
use std::io::{self, BufRead, BufReader, Read, Write, BufWriter};
use std::path::{PathBuf};
use std::process::{Command, Stdio};

fn main() {
    let _ = intro("Setting up your Minecraft Server");

    let setup_complexity: String = select("How do you want to set up your server?")
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
        .default_input("minecraft-server")
        .required(true)
        .interact()
        .unwrap();

    let server_dir: PathBuf;
    let server_port: u16;

    if setup_complexity != "easy" {
        let input_dir: String = input("Where do you want to save your server?")
            .default_input("~/minecraft_server")
            .required(true)
            .validate(|path: &String| {
                let epath = expand_path(path);
                if epath.is_ok() {
                    Ok(())
                } else {
                    Err(epath.err().unwrap())
                }
            })
            .interact()
            .unwrap();

        let slash = if input_dir.ends_with('/') { "" } else { "/" };
        server_dir = expand_path(format!("{}{}", input_dir.as_str(), slash).as_str())
            .unwrap()
            .join(server_name.as_str());

        println!("Server directory set to: {}", server_dir.display());

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
    } else {
        server_dir = expand_path(".").unwrap().join(server_name.as_str());
        println!("Server directory set to: {}", server_dir.display());
        server_port = 25565;
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

    let jar_url = get_jar_url(&platform, &version);
    let _ = download_url(&jar_url, &server_dir, "server.jar");

    run_server(&server_dir);

    if confirm("Do you accept EULA?").interact().unwrap() {
        configure_server(format!("{}/eula.txt", server_dir.to_str().unwrap()).as_str(), "eula", "true");
    } else {
        println!("You must accept the EULA to start the server. Exiting...");
        std::process::exit(0);
    }

    configure_server(format!("{}/server.properties", server_dir.to_str().unwrap()).as_str(), "server-port", server_port.to_string().as_str());

    let _ = outro("You're all set!");

    println!("Server Name: {}", server_name);
    println!("Server Directory: {}", server_dir.display());
    println!("Server Port: {}", server_port);
}

fn run_server(dir: &PathBuf) -> std::io::Result<()> {
    let spinner = spinner();
    spinner.start("Setting up server...");
    let mut cmd = Command::new("java")
        .arg("-jar")
        .arg("server.jar")
        .arg("nogui")
        .current_dir(dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;

    cmd.wait()?;
    spinner.stop("");
    Ok(())
}

fn configure_server(filename: &str, name: &str, value: &str) {
    let file = File::open(filename).unwrap();
    let reader = BufReader::new(file);

    for (index, line) in reader.lines().enumerate() {   
        let line = line.unwrap();     
        // Trim whitespace to handle "  name=" cases
        let trimmed = line.trim_start();
        
        if trimmed.starts_with(format!("{}=", name).as_str()) {
            update_line_at_index(PathBuf::from(filename), index, format!("{}={}", name, value).as_str()).unwrap();
            break;
        }
    }
}

fn update_line_at_index(
    path: PathBuf,          // Now takes ownership of a PathBuf
    target_index: usize, 
    new_content: &str
) -> std::io::Result<()> {
    // 1. Create the temp path
    // .with_extension returns a new PathBuf
    let temp_path = path.with_extension("tmp");

    let input = File::open(&path)?;
    let buffered_input = BufReader::new(input);

    let output = File::create(&temp_path)?;
    let mut buffered_output = BufWriter::new(output);

    for (current_index, line) in buffered_input.lines().enumerate() {
        let line = line?;
        
        if current_index == target_index {
            writeln!(buffered_output, "{}", new_content)?;
        } else {
            writeln!(buffered_output, "{}", line)?;
        }
    }

    buffered_output.flush()?;

    // 2. Atomic swap
    // We pass the paths by reference here
    fs::rename(&temp_path, &path)?;

    Ok(())
}

fn download_url(url: &str, dir: &PathBuf, filename: &str) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let mut res = client.get(url).send()?;

    if !res.status().is_success() {
        return Err(format!("Server returned error: {}", res.status()).into());
    }

    let total_size = res.content_length().ok_or("Failed to get content length")?;

    create_dir_all(&dir)?;

    let file_path = dir.join(filename);
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

fn expand_path(path: &str) -> Result<PathBuf, String> {
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
        Err("The path does not exist. Please enter a valid directory path.".to_string())
    }
}
