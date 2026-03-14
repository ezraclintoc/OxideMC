pub mod config;
pub mod download;
pub mod mods;
pub mod preset;
pub mod server;
pub mod utils;

// Re-export everything so tests (and other modules) can use `use super::*`
pub use config::*;
pub use download::*;
pub use mods::*;
pub use preset::*;
pub use server::*;
pub use utils::*;

use cliclack::{input, log, select};
use std::env;
use std::path::PathBuf;

#[tokio::main]
async fn main() {
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
        let _oxide = OxideMC::setup().await;
    } else if action == "configure" {
        let dir: PathBuf = input("Enter the path to your server directory:")
            .interact()
            .unwrap();
        match OxideMC::open(&dir) {
            Ok(oxide) => {
                log::info(format!(
                    "Server found: {} {}",
                    oxide.platform, oxide.version
                ))
                .unwrap();
                let _ = oxide.configure().await;
            }
            Err(e) => {
                eprintln!("Failed to open server: {}", e);
                std::process::exit(1);
            }
        }
    }
}

#[cfg(test)]
mod tests;
