use crate::config::{configure_file, read_oxide_config, read_property, write_oxide_config};
use crate::download::{convert_to_items, download_url, get_jar_url, get_versions};
use crate::mods::{get_curseforge_key, install_curseforge, install_modrinth, search_modrinth};
use crate::preset::{auto_save_preset, list_presets, load_preset, save_preset};
use crate::utils::{backup_world, expand_path, get_platform, list_entries};
use cliclack::{confirm, input, intro, log, multiselect, outro, select, spinner};
use ferinth::structures::project::ProjectType;
//use serde_json::error;
use std::fs::{self, create_dir_all};
use std::path::PathBuf;
use tokio::process::Command;
use std::process::Stdio;

pub struct OxideMC {
    pub dir: PathBuf,
    pub platform: String,
    pub version: String,
}

impl OxideMC {
    pub async fn setup() -> Result<Self, ()> {
        let _ = intro("Setting up your Minecraft Server");

        let name: String = input("What do you want to name your server?")
            .default_input("minecraft-server")
            .required(true)
            .interact()
            .unwrap();

        let input_dir: String = input("Where do you want to save your server?")
            .default_input("./minecraft_servers")
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

        let base_dir = expand_path(input_dir.as_str()).unwrap();
        let dir = base_dir.join(name.as_str());
        create_dir_all(&dir).expect("Failed to create server directory");

        let platform = select("Which software do you want to use?")
            .item("Vanilla", "Vanilla", "")
            .item("Paper", "Paper (Recommended)", "Plugins + better performance")
            .item("Fabric", "Fabric", "Mods support")
            .interact()
            .unwrap()
            .to_string();

        let version = select("Which version do you want to use?")
            .items(&convert_to_items(&get_versions(&platform).await.unwrap()))
            .interact()
            .unwrap();

        let jar_url = get_jar_url(&platform, &version).await.unwrap();
        let _ = download_url(&jar_url, &dir, "server.jar").await;

        let oxide = OxideMC {
            dir: dir.clone(),
            platform,
            version,
        };

        if oxide.start().await.is_err() {
            log::error("Failed to start server. This is likely because Java is not installed or not in your PATH. Please install Java and try again.").unwrap();
        }

        if confirm("Do you accept EULA?").interact().unwrap() {
            let _ = configure_file(&dir, "eula.txt", "eula", "true");
        } else {
            log::warning("You must accept the EULA to run the server.").unwrap();
        }

        auto_save_preset(&dir, &oxide.platform, &oxide.version);

        let _ = outro("You're all set!");

        Ok(oxide)
    }

    pub async fn new(dir: PathBuf, platform: String, version: String) -> Self {
        let jar_url = get_jar_url(&platform, &version).await.unwrap();
        let _ = download_url(&jar_url, &dir, "server.jar").await;

        let _ = configure_file(&dir, "eula.txt", "eula", "true");

        OxideMC {
            dir,
            platform,
            version,
        }
    }

    pub fn open(dir: &PathBuf) -> Result<Self, String> {
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

    pub async fn configure(&self) {
        let mut page: &str = "main";

        // Auto-save preset.json on every loop iteration (after each setting change)
        loop {
            auto_save_preset(&self.dir, &self.platform, &self.version);
            match page {
                "main" => {
                    let mods_label = match self.platform.as_str() {
                        "Paper" => "Plugins",
                        "Vanilla" => "Mods / Plugins",
                        _ => "Mods",
                    };
                    let mods_hint = match self.platform.as_str() {
                        "Vanilla" => "Vanilla does not support mods or plugins",
                        "Paper" => "Install, update, or remove plugins",
                        _ => "Install, update, or remove mods",
                    };
                    page = select("What do you want to configure?")
                        .item("presets", "Presets", "Save/load configs")
                        .item("game", "Game", "Players, difficulty, gamemode")
                        .item("world", "World", "Seed, type, backup, border")
                        .item("mods", mods_label, mods_hint)
                        .item("advanced", "Advanced", "Port, MOTD, online mode")
                        .item("quit", "Quit", "")
                        .interact()
                        .unwrap();
                }
                "presets" => {
                    let action = select("Presets")
                        .item("save", "Export Preset", "Copy preset.json to a file")
                        .item("load", "Load Preset", "Apply a saved preset")
                        .item("back", "Back", "")
                        .interact()
                        .unwrap();
                    match action {
                        "save" => {
                            // Make sure preset.json is up to date first
                            auto_save_preset(&self.dir, &self.platform, &self.version);
                            let dest: String = input("Save preset to (file path):")
                                .required(true)
                                .interact()
                                .unwrap();
                            let dest_path = PathBuf::from(
                                dest.replace(
                                    '~',
                                    &dirs::home_dir()
                                        .unwrap_or_default()
                                        .to_string_lossy(),
                                ),
                            );
                            match save_preset(&self.dir, &dest_path) {
                                Ok(path) => {
                                    log::success(format!(
                                        "Preset exported to {}",
                                        path.display()
                                    ))
                                    .unwrap()
                                }
                                Err(e) => {
                                    log::error(format!("Failed to export preset: {}", e)).unwrap()
                                }
                            }
                        }
                        "load" => {
                            let presets_dir = self.dir.join("presets");
                            let has_saved = list_presets(&presets_dir)
                                .map(|p| !p.is_empty())
                                .unwrap_or(false);

                            let source = if has_saved {
                                select("Load from:")
                                    .item("saved", "Saved Presets", "From presets/ folder")
                                    .item("file", "File Path", "Load from a file")
                                    .item("back", "Back", "")
                                    .interact()
                                    .unwrap()
                            } else {
                                "file"
                            };

                            let preset_path = match source {
                                "saved" => {
                                    let presets = list_presets(&presets_dir).unwrap();
                                    let items: Vec<(String, String, String)> = presets
                                        .iter()
                                        .map(|p| (p.clone(), p.clone(), String::new()))
                                        .collect();
                                    let chosen = select("Select a preset:")
                                        .items(&items)
                                        .interact()
                                        .unwrap()
                                        .to_string();
                                    Some(presets_dir.join(format!("{}.json", chosen)))
                                }
                                "file" => {
                                    let path_str: String = input("Path to preset file:")
                                        .required(true)
                                        .validate(|s: &String| {
                                            let expanded = s.replace(
                                                '~',
                                                &dirs::home_dir()
                                                    .unwrap_or_default()
                                                    .to_string_lossy(),
                                            );
                                            let p = PathBuf::from(&expanded);
                                            if !p.exists() {
                                                Err("File not found. Please enter a valid path."
                                                    .to_string())
                                            } else if p.is_dir() {
                                                Err("Path is a directory, not a file.".to_string())
                                            } else {
                                                Ok(())
                                            }
                                        })
                                        .interact()
                                        .unwrap();
                                    Some(PathBuf::from(path_str.replace(
                                        '~',
                                        &dirs::home_dir()
                                            .unwrap_or_default()
                                            .to_string_lossy(),
                                    )))
                                }
                                _ => None,
                            };

                            if let Some(path) = preset_path {
                                match load_preset(
                                    &self.dir,
                                    &path,
                                    &self.platform,
                                    &self.version,
                                ) {
                                    Ok(()) => {
                                        auto_save_preset(
                                            &self.dir,
                                            &self.platform,
                                            &self.version,
                                        );
                                        log::success("Preset applied!").unwrap();
                                    }
                                    Err(e) => {
                                        log::error(format!("Failed to load: {}", e)).unwrap()
                                    }
                                }
                            }
                        }
                        _ => {
                            page = "main";
                        }
                    }
                }
                "game" => {
                    let subpage = select("Game Settings")
                        .item("max-players", "Max Players", "Max simultaneous players")
                        .item("difficulty", "Difficulty", "")
                        .item("gamemode", "Gamemode", "Default for new players")
                        .item("pvp", "PVP", "Player vs player combat")
                        .item("back", "Back", "")
                        .interact()
                        .unwrap();
                    match subpage {
                        "max-players" => {
                            let max: String = input("Max players:")
                                .default_input("20")
                                .validate(|input: &String| {
                                    if input.parse::<u32>().map(|n| n >= 1).unwrap_or(false) {
                                        Ok(())
                                    } else {
                                        Err("Please enter a positive integer".to_string())
                                    }
                                })
                                .interact()
                                .unwrap();
                            configure_file(&self.dir, "server.properties", "max-players", &max)
                                .unwrap();
                        }
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
                        _ => {
                            page = "main";
                        }
                    }
                }
                "world" => {
                    let subpage = select("World")
                        .item("seed", "Seed", "World generation seed")
                        .item("worldtype", "Type", "Terrain generation type")
                        .item("backup", "Backup", "Create or configure backups")
                        .item("worldborder", "World Border", "Radius and center")
                        .item("back", "Back", "")
                        .interact()
                        .unwrap();
                    match subpage {
                        "seed" => {
                            let seed: String = input("Seed (leave blank for random):")
                                .default_input("")
                                .required(false)
                                .interact()
                                .unwrap();
                            configure_file(
                                &self.dir,
                                "server.properties",
                                "level-seed",
                                &seed,
                            )
                            .unwrap();
                        }
                        "worldtype" => {
                            let worldtype = select("Select the world type:")
                                .item("minecraft:normal", "Normal", "Default terrain generation")
                                .item("minecraft:flat", "Flat", "Completely flat world")
                                .item(
                                    "minecraft:large_biomes",
                                    "Large Biomes",
                                    "Normal world with larger biomes",
                                )
                                .item(
                                    "minecraft:amplified",
                                    "Amplified",
                                    "Normal world with extreme terrain",
                                )
                                .interact()
                                .unwrap();
                            configure_file(
                                &self.dir,
                                "server.properties",
                                "level-type",
                                worldtype,
                            )
                            .unwrap();
                        }
                        "backup" => {
                            let backup_action = select("Backup")
                                .item("now", "Backup Now", "Timestamped backup now")
                                .item("folder", "Backup Folder", "Set backup directory")
                                .item("back", "Back", "")
                                .interact()
                                .unwrap();
                            match backup_action {
                                "now" => match backup_world(&self.dir) {
                                    Ok(()) => {}
                                    Err(e) => {
                                        log::error(format!("Backup failed: {}", e)).unwrap()
                                    }
                                },
                                "folder" => {
                                    let current = read_oxide_config(&self.dir, "backup_dir")
                                        .unwrap_or_else(|_| {
                                            self.dir
                                                .join("backups")
                                                .to_string_lossy()
                                                .to_string()
                                        });
                                    let folder: String = input("Backup folder path:")
                                        .default_input(&current)
                                        .interact()
                                        .unwrap();
                                    match write_oxide_config(&self.dir, "backup_dir", &folder) {
                                        Ok(()) => log::success(format!(
                                            "Backup folder set to {}",
                                            folder
                                        ))
                                        .unwrap(),
                                        Err(e) => {
                                            log::error(format!("Failed to save: {}", e)).unwrap()
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        "worldborder" => {
                            let border_action = select("World Border")
                                .item("radius", "Radius", "Max world radius in blocks")
                                .item("center", "Center", "Get /worldborder center command")
                                .item("back", "Back", "")
                                .interact()
                                .unwrap();
                            match border_action {
                                "radius" => {
                                    let size: String =
                                        input("Max world radius in blocks (1-29999984):")
                                            .default_input("29999984")
                                            .validate(|input: &String| {
                                                match input.parse::<u32>() {
                                                    Ok(n) if n >= 1 => Ok(()),
                                                    _ => Err(
                                                        "Please enter a positive integer up to 29999984"
                                                            .to_string(),
                                                    ),
                                                }
                                            })
                                            .interact()
                                            .unwrap();
                                    configure_file(
                                        &self.dir,
                                        "server.properties",
                                        "max-world-size",
                                        &size,
                                    )
                                    .unwrap();
                                }
                                "center" => {
                                    let center: String = input("World border center (x,z):")
                                        .default_input("0,0")
                                        .validate(|input: &String| {
                                            let parts: Vec<&str> = input.split(',').collect();
                                            if parts.len() == 2
                                                && parts[0].trim().parse::<f64>().is_ok()
                                                && parts[1].trim().parse::<f64>().is_ok()
                                            {
                                                Ok(())
                                            } else {
                                                Err("Enter coordinates as x,z (e.g. 0,0)"
                                                    .to_string())
                                            }
                                        })
                                        .interact()
                                        .unwrap();
                                    let parts: Vec<&str> = center.split(',').collect();
                                    log::info(format!(
                                        "Run in-game or via RCON: /worldborder center {} {}",
                                        parts[0].trim(),
                                        parts[1].trim()
                                    ))
                                    .unwrap();
                                }
                                _ => {}
                            }
                        }
                        _ => {
                            page = "main";
                        }
                    }
                }
                "mods" => {
                    let action = select("Mods & Content")
                        .item("install", "Install", "Modrinth or CurseForge")
                        .item("remove", "Remove", "Uninstall content")
                        .item("update", "Update", "Check for updates (coming soon)")
                        .item("back", "Back", "")
                        .interact()
                        .unwrap();
                    match action {
                        "install" => {
                            // Build content type options based on platform
                            let ct_options: &[(&str, &str, &str)] =
                                match self.platform.as_str() {
                                    "Paper" => &[
                                        ("plugin", "Plugin", "Bukkit/Paper plugins"),
                                        ("mod", "Mod", "Mods via bridge"),
                                        ("datapack", "Datapack", "Data packs"),
                                        ("resourcepack", "Resource Pack", "Resource packs"),
                                    ],
                                    "Fabric" | "Forge" => &[
                                        ("mod", "Mod", "Fabric/Forge mods"),
                                        ("datapack", "Datapack", "Data packs"),
                                        ("resourcepack", "Resource Pack", "Resource packs"),
                                    ],
                                    _ => &[
                                        ("datapack", "Datapack", "Data packs"),
                                        ("resourcepack", "Resource Pack", "Resource packs"),
                                    ],
                                };

                            let mut ct_select = select("What type of content?");
                            for (id, label, hint) in ct_options {
                                ct_select = ct_select.item(*id, *label, *hint);
                            }
                            ct_select = ct_select.item("back", "Back", "");
                            let content_type = ct_select.interact().unwrap();
                            if content_type == "back" {
                                continue;
                            }

                            let install_dir = match content_type {
                                "plugin" => self.dir.join("plugins"),
                                "mod" => self.dir.join("mods"),
                                "datapack" => {
                                    let level_name = read_property(
                                        &self.dir,
                                        "server.properties",
                                        "level-name",
                                    )
                                    .unwrap_or_else(|_| "world".to_string());
                                    self.dir.join(level_name).join("datapacks")
                                }
                                _ => self.dir.join("resourcepacks"),
                            };

                            let source = select("Install from:")
                                .item("modrinth", "Modrinth", "Search Modrinth library")
                                .item("curseforge", "CurseForge", "Install by project ID")
                                .item("back", "Back", "")
                                .interact()
                                .unwrap();

                            match source {
                                "modrinth" => {
                                    let project_type = match content_type {
                                        "plugin" => ProjectType::Plugin,
                                        "datapack" => ProjectType::Datapack,
                                        "resourcepack" => ProjectType::ResourcePack,
                                        _ => ProjectType::Mod,
                                    };
                                    let loader: Option<&str> =
                                        match (content_type, self.platform.as_str()) {
                                            ("mod", "Fabric") => Some("fabric"),
                                            ("mod", "Forge") => Some("forge"),
                                            ("plugin", "Paper") => Some("paper"),
                                            _ => None,
                                        };

                                    let query: String = input("Search Modrinth:")
                                        .required(true)
                                        .interact()
                                        .unwrap();

                                    match search_modrinth(
                                        &query,
                                        project_type,
                                        loader,
                                        Some(&self.version),
                                    )
                                    .await
                                    {
                                        Ok(results) if !results.is_empty() => {
                                            let items: Vec<(String, String, String)> = results
                                                .iter()
                                                .map(|h| {
                                                    let hint = if h.description.len() > 45 {
                                                        format!("{}…", &h.description[..45])
                                                    } else {
                                                        h.description.clone()
                                                    };
                                                    (h.project_id.clone(), h.title.clone(), hint)
                                                })
                                                .collect();
                                            let chosen_ids = multiselect(
                                                "Select content to install (space to toggle, enter to confirm):",
                                            )
                                            .items(&items)
                                            .interact()
                                            .unwrap();

                                            for chosen_id in chosen_ids {
                                                match install_modrinth(
                                                    &chosen_id,
                                                    loader,
                                                    &self.version,
                                                    &install_dir,
                                                )
                                                .await
                                                {
                                                    Ok(name) => log::success(format!(
                                                        "Installed {}",
                                                        name
                                                    ))
                                                    .unwrap(),
                                                    Err(e) => log::error(format!(
                                                        "Failed to install {}: {}",
                                                        chosen_id, e
                                                    ))
                                                    .unwrap(),
                                                }
                                            }
                                        }
                                        Ok(_) => log::warning("No results found.").unwrap(),
                                        Err(e) => {
                                            log::error(format!("Search failed: {}", e)).unwrap()
                                        }
                                    }
                                }
                                "curseforge" => {
                                    let api_key = get_curseforge_key(&self.dir);
                                    let id_str: String = input("Enter CurseForge project ID:")
                                        .validate(|s: &String| {
                                            if s.parse::<i32>().is_ok() {
                                                Ok(())
                                            } else {
                                                Err("Please enter a numeric project ID"
                                                    .to_string())
                                            }
                                        })
                                        .interact()
                                        .unwrap();
                                    let mod_id: i32 = id_str.parse().unwrap();
                                    match install_curseforge(
                                        mod_id,
                                        &self.version,
                                        &install_dir,
                                        &api_key,
                                    )
                                    .await
                                    {
                                        Ok(name) => {
                                            log::success(format!("Installed {}", name)).unwrap()
                                        }
                                        Err(e) => {
                                            log::error(format!("Install failed: {}", e)).unwrap()
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        "remove" => {
                            let dir_options: &[(&str, &str, &str)] =
                                match self.platform.as_str() {
                                    "Paper" => &[
                                        ("plugins", "Plugins", ""),
                                        ("mods", "Mods", ""),
                                        ("datapacks", "Datapacks", ""),
                                        ("resourcepacks", "Resource Packs", ""),
                                    ],
                                    "Fabric" | "Forge" => &[
                                        ("mods", "Mods", ""),
                                        ("datapacks", "Datapacks", ""),
                                        ("resourcepacks", "Resource Packs", ""),
                                    ],
                                    _ => &[
                                        ("datapacks", "Datapacks", ""),
                                        ("resourcepacks", "Resource Packs", ""),
                                    ],
                                };

                            let mut dir_select = select("Remove from:");
                            for (id, label, hint) in dir_options {
                                dir_select = dir_select.item(*id, *label, *hint);
                            }
                            dir_select = dir_select.item("back", "Back", "");
                            let dir_choice = dir_select.interact().unwrap();
                            if dir_choice == "back" {
                                continue;
                            }

                            let remove_dir = if dir_choice == "datapacks" {
                                let level_name = read_property(
                                    &self.dir,
                                    "server.properties",
                                    "level-name",
                                )
                                .unwrap_or_else(|_| "world".to_string());
                                self.dir.join(level_name).join("datapacks")
                            } else {
                                self.dir.join(dir_choice)
                            };

                            let entries = list_entries(&remove_dir).unwrap_or_default();
                            if entries.is_empty() {
                                log::warning("Nothing found to remove.").unwrap();
                            } else {
                                let items: Vec<(String, String, String)> = entries
                                    .iter()
                                    .map(|f| (f.clone(), f.clone(), String::new()))
                                    .collect();
                                let chosen = select("Select to remove:")
                                    .items(&items)
                                    .interact()
                                    .unwrap()
                                    .to_string();
                                if confirm(format!("Remove {}?", chosen)).interact().unwrap() {
                                    let target = remove_dir.join(&chosen);
                                    let result = if target.is_dir() {
                                        fs::remove_dir_all(&target).map_err(|e| e.to_string())
                                    } else {
                                        fs::remove_file(&target).map_err(|e| e.to_string())
                                    };
                                    match result {
                                        Ok(()) => {
                                            log::success(format!("Removed {}", chosen)).unwrap()
                                        }
                                        Err(e) => {
                                            log::error(format!("Failed: {}", e)).unwrap()
                                        }
                                    }
                                }
                            }
                        }
                        "update" => {
                            log::warning("Update checking is not yet implemented.").unwrap();
                        }
                        _ => {
                            page = "main";
                        }
                    }
                }
                "advanced" => {
                    let setting = select("Advanced Settings")
                        .item("port", "Server Port", "Default: 25565")
                        .item("motd", "MOTD", "Server list message")
                        .item("online-mode", "Online Mode", "Require Mojang auth")
                        .item("view-distance", "View Distance", "Chunks sent to clients")
                        .item(
                            "simulation-distance",
                            "Simulation Distance",
                            "Entity update range",
                        )
                        .item("spawn-protection", "Spawn Protection", "Protected spawn radius")
                        .item("back", "Back", "")
                        .interact()
                        .unwrap();
                    match setting {
                        "port" => {
                            let port: String = input("Server port:")
                                .default_input("25565")
                                .validate(|input: &String| {
                                    if input.parse::<u16>().is_ok() {
                                        Ok(())
                                    } else {
                                        Err("Please enter a valid port (1-65535)".to_string())
                                    }
                                })
                                .interact()
                                .unwrap();
                            configure_file(
                                &self.dir,
                                "server.properties",
                                "server-port",
                                &port,
                            )
                            .unwrap();
                        }
                        "motd" => {
                            let motd: String = input("MOTD (supports \u{00a7}color codes):")
                                .default_input("A Minecraft Server")
                                .interact()
                                .unwrap();
                            configure_file(&self.dir, "server.properties", "motd", &motd).unwrap();
                        }
                        "online-mode" => {
                            let online = select("Require valid Mojang accounts?")
                                .item("true", "Yes (Recommended)", "")
                                .item("false", "No (Cracked)", "Allows non-premium accounts")
                                .interact()
                                .unwrap();
                            configure_file(
                                &self.dir,
                                "server.properties",
                                "online-mode",
                                online,
                            )
                            .unwrap();
                        }
                        "view-distance" => {
                            let dist: String = input("View distance (3-32 chunks):")
                                .default_input("10")
                                .validate(|input: &String| match input.parse::<u32>() {
                                    Ok(n) if (3..=32).contains(&n) => Ok(()),
                                    _ => {
                                        Err("Please enter a number between 3 and 32".to_string())
                                    }
                                })
                                .interact()
                                .unwrap();
                            configure_file(
                                &self.dir,
                                "server.properties",
                                "view-distance",
                                &dist,
                            )
                            .unwrap();
                        }
                        "simulation-distance" => {
                            let dist: String = input("Simulation distance (3-32 chunks):")
                                .default_input("10")
                                .validate(|input: &String| match input.parse::<u32>() {
                                    Ok(n) if (3..=32).contains(&n) => Ok(()),
                                    _ => {
                                        Err("Please enter a number between 3 and 32".to_string())
                                    }
                                })
                                .interact()
                                .unwrap();
                            configure_file(
                                &self.dir,
                                "server.properties",
                                "simulation-distance",
                                &dist,
                            )
                            .unwrap();
                        }
                        "spawn-protection" => {
                            let radius: String =
                                input("Spawn protection radius in blocks (0 to disable):")
                                    .default_input("16")
                                    .validate(|input: &String| {
                                        if input.parse::<u32>().is_ok() {
                                            Ok(())
                                        } else {
                                            Err("Please enter a non-negative integer".to_string())
                                        }
                                    })
                                    .interact()
                                    .unwrap();
                            configure_file(
                                &self.dir,
                                "server.properties",
                                "spawn-protection",
                                &radius,
                            )
                            .unwrap();
                        }
                        _ => {
                            page = "main";
                        }
                    }
                }
                "quit" => {
                    break;
                }
                _ => {
                    page = "main";
                }
            }
        }
    }

    pub async fn start(&self) -> Result<(), ()> {
        let spinner = spinner();
        spinner.start("Setting up server...");
        let mut cmd: tokio::process::Child = match Command::new("java")
            .arg("-jar")
            .arg("server.jar")
            .arg("nogui")
            .current_dir(&self.dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(e) => {
                spinner.stop("Failed to start server");
                eprintln!(
                    "Could not launch Java: {}. Is Java installed and in your PATH?",
                    e
                );
                return Err(());
            }
        };

        cmd.wait().await.unwrap();
        spinner.stop("");
        Ok(())
    }
}
