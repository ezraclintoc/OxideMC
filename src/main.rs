use cliclack::{confirm, input, intro, log, outro, progress_bar, select, spinner};
use reqwest::blocking::Client;
use serde_json::Value;
use std::env;
use std::error::Error;
use std::fs::{self, create_dir_all, File};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

struct OxideMC {
    dir: PathBuf,
    platform: String,
    version: String,
}

impl OxideMC {
    pub fn create_with_interact() -> Result<Self, ()> {
        let _ = intro("Setting up your Minecraft Server");

        let name: String = input("What do you want to name your server?")
            .default_input("minecraft-server")
            .required(true)
            .interact()
            .unwrap();

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
        let dir = expand_path(format!("{}{}", input_dir.as_str(), slash).as_str())
            .unwrap()
            .join(name.as_str());

        let platform = select("Which software do you want to use?")
            .item("Vanilla", "Vanilla", "")
            .item(
                "Paper",
                "Paper (Recommended)",
                "Has plugins support and better performance!",
            )
            .item("Fabric", "Fabric", "Has mods support!")
            .interact()
            .unwrap()
            .to_string();

        let version = select("Which version do you want to use?")
            .items(&convert_to_items(&get_versions(&platform).unwrap()))
            .interact()
            .unwrap();

        let jar_url = get_jar_url(&platform, &version).unwrap();
        let _ = download_url(&jar_url, &dir, "server.jar");

        let out = OxideMC {
            dir: dir.clone(),
            platform,
            version,
        };

        let _ = out.run();

        if confirm("Do you accept EULA?").interact().unwrap() {
            let _ = configure_file(&dir, "eula.txt", "eula", "true");
        } else {
            eprintln!("You must accept the EULA to start the server. Exiting...");
            std::process::exit(0);
        }

        let _ = outro("You're all set!");

        Ok(out)
    }

    pub fn create(dir: PathBuf, platform: String, version: String) -> Self {
        let jar_url = get_jar_url(&platform, &version).unwrap();
        let _ = download_url(&jar_url, &dir, "server.jar");

        let _ = configure_file(&dir, "eula.txt", "eula", "true");

        OxideMC {
            dir,
            platform,
            version,
        }
    }

    pub fn create_from_existing(dir: &PathBuf) -> Result<Self, String> {
        let dir = expand_path(dir.to_str().unwrap()).unwrap();
        let jar_path = dir.join("server.jar");
        if !jar_path.exists() {
            let jar = fs::read_dir(&dir)
                .map_err(|e| e.to_string())?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .find(|p| p.extension().and_then(|e| e.to_str()) == Some("jar"));

            match jar {
                Some(path) => fs::rename(&path, &dir.join("server.jar")).map_err(|e| {
                    format!("Failed to rename {} to server.jar: {}", path.display(), e)
                })?,
                None => return Err("No .jar file found in directory".to_string()),
            }
        }

        let versions_dir = dir.join("versions");
        let mut versions: Vec<String> = fs::read_dir(&versions_dir)
            .unwrap()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.is_dir() {
                    Some(
                        path.to_str()
                            .unwrap()
                            .split("/")
                            .last()
                            .unwrap()
                            .to_string(),
                    )
                } else {
                    None
                }
            })
            .collect();

        versions.sort_by(|a, b| {
            let parse =
                |v: &str| -> Vec<u32> { v.split('.').filter_map(|n| n.parse().ok()).collect() };
            parse(b).cmp(&parse(a))
        });

        Ok(OxideMC {
            dir: dir.clone(),
            platform: get_platform(&dir).unwrap(),
            version: versions.first().unwrap().to_string(),
        })
    }

    pub fn configure(&self) {
        let mut page: &str = "main";

        loop {
            match page {
                "main" => {
                    page = select("What do you want to configure?")
                        .item("game", "Game Settings", "")
                        .item(
                            "mod",
                            format!(
                                "{}Datapacks/Resource Packs",
                                match self.platform.as_str() {
                                    "Vanilla" => "",
                                    "Paper" => "Plugin/",
                                    "Fabric" => "Mod/",
                                    "Forge" => "Mod/",
                                    _ => "",
                                }
                            ),
                            "",
                        )
                        .item("advanced", "advanced", "")
                        .interact()
                        .unwrap();
                }
                "game" => {
                    let subpage = select("Which setting do you want to change?")
                        .item("difficulty", "Difficulty", "")
                        .item("gamemode", "Gamemode", "")
                        .item("pvp", "PVP", "")
                        .item("back", "Back", "")
                        .interact()
                        .unwrap();
                    match subpage {
                        "difficulty" => {
                            let difficulty = select("Select the difficulty level:")
                                .item("peaceful", "Peaceful", "")
                                .item("easy", "Easy", "")
                                .item("normal", "Normal", "")
                                .item("hard", "Hard", "")
                                .interact()
                                .unwrap();
                            configure_file(
                                &self.dir,
                                "server.properties",
                                "difficulty",
                                difficulty,
                            )
                            .unwrap();
                        }
                        "gamemode" => {
                            let gamemode = select("Select the default gamemode:")
                                .item("survival", "Survival", "")
                                .item("creative", "Creative", "")
                                .item("adventure", "Adventure", "")
                                .item("spectator", "Spectator", "")
                                .interact()
                                .unwrap();
                            configure_file(&self.dir, "server.properties", "gamemode", gamemode)
                                .unwrap();
                        }
                        "pvp" => {
                            let pvp = select("Enable PVP?")
                                .item("true", "Yes", "")
                                .item("false", "No", "")
                                .interact()
                                .unwrap();
                            configure_file(&self.dir, "server.properties", "pvp", pvp).unwrap();
                        }
                        "back" => {
                            page = "main";
                        }
                        _ => {}
                    }
                }
                _ => {
                    println!("This page is not implemented yet.");
                    page = "main";
                }
            }
        }
    }

    pub fn run(&self) -> Result<(), ()> {
        let spinner = spinner();
        spinner.start("Setting up server...");
        let mut cmd = Command::new("java")
            .arg("-jar")
            .arg("server.jar")
            .arg("nogui")
            .current_dir(&self.dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();

        cmd.wait().unwrap();
        spinner.stop("");
        Ok(())
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let action = if args.len() > 1 {
        match args[1].to_lowercase().as_str() {
            "install" | "--install" | "i" | "-i" => "install".to_string(),
            "configure" | "--configure" | "c" | "-c" => "configure".to_string(),
            _ => todo!("WHAT?"),
        }
    } else {
        select("Do you want to install a new server or configure an existing one?")
            .item("install", "Install a New Server", "")
            .item("configure", "Configure An Exisiting Server", "")
            .interact()
            .unwrap()
            .to_string()
    };

    if action == "install" {
        let _oxide = OxideMC::create_with_interact();
    } else if action == "configure" {
        let oxide = OxideMC::create_from_existing(
            &input("Enter the path to your server directory:")
                .interact()
                .unwrap(),
        )
        .unwrap();
        log::info(format!(
            "Server found: {} {}",
            oxide.platform, oxide.version
        ));
        let _ = oxide.configure();
    }
}

fn get_versions(platform: &str) -> Result<Vec<String>, String> {
    if platform == "Vanilla" {
        let json_text = reqwest::blocking::get(
        "https://raw.githubusercontent.com/liebki/MinecraftServerForkDownloads/refs/heads/main/release_vanilla_downloads.json"
        )
        .map_err(|e| format!("Failed to fetch vanilla downloads: {}", e))?
        .text()
        .map_err(|e| format!("Failed to read response: {}", e))?;

        let json: serde_json::Value =
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

fn configure_file(dir: &PathBuf, filename: &str, name: &str, value: &str) -> Result<(), ()> {
    let path = PathBuf::from(dir.to_str().unwrap().to_string() + "/" + filename);
    let input = File::open(&path).unwrap();
    let buffered_input = BufReader::new(input);

    let temp_path = path.with_extension("tmp");

    let output = File::create(&temp_path).unwrap();
    let mut buffered_output = BufWriter::new(output);

    for line in buffered_input.lines() {
        let line = line.unwrap();
        // Trim whitespace to handle "  name=" cases
        let trimmed = line.trim_start();

        if trimmed.starts_with(format!("{}=", name).as_str()) {
            writeln!(buffered_output, "{}={}", name, value).unwrap();
        } else {
            writeln!(buffered_output, "{}", line).unwrap();
        }
    }
    buffered_output.flush().unwrap();

    fs::rename(&temp_path, &path).unwrap();

    Ok(())
}

fn get_jar_url(platform: &str, version: &str) -> Result<String, String> {
    if platform == "Vanilla" {
        let json = reqwest::blocking::get(
            "https://raw.githubusercontent.com/liebki/MinecraftServerForkDownloads/refs/heads/main/release_vanilla_downloads.json"
        )
        .map_err(|e| format!("Failed to fetch vanilla downloads: {}", e))?
        .text()
        .map_err(|e| format!("Failed to read response: {}", e))?;

        let json: serde_json::Value =
            serde_json::from_str(&json).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        json.get("server_available")
            .and_then(|v| v.get(version))
            .and_then(|u| u.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| format!("No download URL found for Vanilla {}", version))
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
        Ok(json
            .get("downloads")
            .unwrap()
            .get("server:default")
            .unwrap()
            .get("url")
            .unwrap()
            .to_string()
            .trim_matches('"')
            .to_string())
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

        Ok(fabric_url)
    } else if platform == "Forge" {
        todo!("Forge not implemented");
    } else {
        Err("Unkonwn Platform".to_string())
    }
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

fn convert_to_items(input: &[String]) -> Vec<(String, String, String)> {
    input
        .iter()
        .map(|v| (v.clone(), v.clone(), String::new()))
        .collect()
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

fn get_platform(dir: &PathBuf) -> Result<String, String> {
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

    Err("".to_string())
}
