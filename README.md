# 🚀 OxideMC

<div align="center">

<!-- TODO: Add project logo (e.g., an icon related to Minecraft or Rust) -->
<img src="logo.png" align="center" width="150" style="margin: 20px; border-radius: 50%;">

[![GitHub stars](https://img.shields.io/github/stars/ezraclintoc/OxideMC?style=for-the-badge)](https://github.com/ezraclintoc/OxideMC/stargazers)
[![GitHub forks](https://img.shields.io/github/forks/ezraclintoc/OxideMC?style=for-the-badge)](https://github.com/ezraclintoc/OxideMC/network)
[![GitHub issues](https://img.shields.io/github/issues/ezraclintoc/OxideMC?style=for-the-badge)](https://github.com/ezraclintoc/OxideMC/issues)
[![GitHub license](https://img.shields.io/github/license/ezraclintoc/OxideMC?style=for-the-badge)](LICENSE)

**A high-performance, Rust-powered Minecraft server setup tool**

<!-- TODO: Add live demo link if applicable (e.g., a hosted server) -->
<!-- TODO: Add documentation link if applicable -->

</div>

## 📖 Overview

OxideMC is a project aimed at helping people create minecraft server faster and quicker. It provides a streamlined setup process for creating Minecraft servers, making it easier than ever to get started with your own server. With its user-friendly interface, you can quickly set up your Minecraft server with just a few clicks. Whether you're a beginner or an experienced server administrator, OxideMC is the perfect tool for anyone looking to set up their own Minecraft server.

## ✨ Features

- 🪄 **Interactive Setup Wizard** - Easy and advanced modes for beginners and pros
- 🎮 **Multi-Platform Support** - Download Vanilla, Paper, or Fabric servers
- 📥 **Automatic Downloads** - Fetch the latest JAR files directly with progress bars
- ⚙️ **Customizable** - Set server name, directory, and port
- 📋 **Version Selection** - Choose from all available Minecraft versions
- 🦀 **Blazing Fast** - Built with Rust for maximum performance

<!-- ## 🖥️ Screenshots -->

<!-- TODO: Add actual screenshots if this project has a visual component (e.g., a game client or server console) -->

## 🛠️ Tech Stack

![Rust](https://img.shields.io/badge/Rust-black?style=for-the-badge&logo=rust&logoColor=white) ![Cargo](https://img.shields.io/badge/Cargo-black?style=for-the-badge&logo=rust&logoColor=white)

## 🚀 Quick Start

Follow these steps to get OxideMC up and running on your local machine.

### Option 1: Install Script (Recommended)
> ⚠️ Not yet implemented - Coming soon!

```bash
curl -sL https://oxidemc.dev/install | bash
```

### Option 2: Download Binary
Download the latest release for your platform:

| Platform | Download |
|----------|----------|
| Linux x86_64 | [oxidemc-linux-x86_64](https://github.com/ezraclintoc/OxideMC/releases/latest) |
| macOS ARM64 | [oxidemc-macos-arm64](https://github.com/ezraclintoc/OxideMC/releases/latest) |
| Windows x86_64 | [oxidemc-windows-x86_64.exe](https://github.com/ezraclintoc/OxideMC/releases/latest) |

### Option 3: Build from Source
```bash
git clone https://github.com/ezraclintoc/OxideMC.git
cd OxideMC
cargo build --release
./target/release/oxidemc
```

## 📋 TODO

### Server Types
- [x] Add Paper server support
- [ ] Add Vanilla server support
- [x] Add Fabric server support
- [ ] Add Forge server support

### Core Features
- [x] Interactive setup wizard (easy/advanced modes)
- [x] Version selection from available MC versions
- [x] Automatic JAR downloads with progress
- [x] Path expansion (~, ./, ../)
- [x] Server start after download with EULA
- [ ] Add more server configuration options (RAM, JVM flags)
- [ ] Server port configuration
- [ ] Server name configuration

### Mod Management
- [ ] Create start script (cross-platform)
- [ ] Create mod installer script
  - [ ] Modrinth support
  - [ ] CurseForge support
  - [ ] Add mods by URL

### Distribution
- [x] Create GitHub Actions for automated builds
- [ ] Host install script on domain
- [ ] Add more platforms for binary releases

### Polish
- [ ] Add configuration file support
- [ ] Add update checker
- [ ] Add server properties editor

## 📁 Project Structure

```
OxideMC/
├── src/                # Source code directory (e.g., main.rs, lib.rs, modules)
├── Cargo.toml          # Rust project manifest and dependencies
├── Cargo.lock          # Locked dependencies for reproducible builds
├── .gitignore          # Specifies intentionally untracked files to ignore
└── README.md           # This README file
```

## 🤝 Contributing

We welcome contributions to OxideMC! Please consider opening an issue first to discuss potential changes or enhancements.

## 📄 License

This project is licensed under the [GNU General Public License v3.0](LICENSE).

## 🙏 Acknowledgments

- Built with the power and safety of the **Rust programming language**.
- Managed by **Cargo**, Rust's robust build system and package manager.

## 📞 Support & Contact

- 🐛 Issues: [GitHub Issues](https://github.com/ezraclintoc/OxideMC/issues)

---

<div align="center">

**⭐ Star this repo if you find it helpful!**

Made with ❤️ by [ezraclintoc](https://github.com/ezraclintoc)

</div>