<div align="center">

# 🎮 Steam Manifest Downloader

**A sleek desktop app for downloading Steam game depots using manifest data.**

![Version](https://img.shields.io/badge/version-1.0.0-blue?style=for-the-badge)
![License](https://img.shields.io/badge/license-GPL--2.0-blue?style=for-the-badge)
![Platform](https://img.shields.io/badge/platform-Windows-0078D6?style=for-the-badge&logo=windows)
![Built with](https://img.shields.io/badge/built_with-Rust-dea584?style=for-the-badge&logo=rust)
![Tauri](https://img.shields.io/badge/Tauri_v2-FFC131?style=for-the-badge&logo=tauri&logoColor=white)
![Downloads](https://img.shields.io/github/downloads/MCbabel/Steam-Manifest-Downloader/total?style=for-the-badge&color=brightgreen)

Upload `.lua` files, search across GitHub repos, and let the app handle manifests, depot keys, and downloads — all in one click.

</div>

---

> [!WARNING]
> ## ⚠️ Legal Disclaimer
>
> This project does **NOT** support or encourage piracy in any way.
>
> - **DepotDownloaderMod** must **ONLY** be used with your own legally obtained Steam keys.
> - This tool is intended for **legitimate use cases only** (e.g., downloading your own purchased content, archiving, backup, etc.).
> - The developer takes **no responsibility** for any misuse of this tool.
> - By using this software, you agree to comply with all applicable laws and Steam's Terms of Service.

---

## ✨ Features

| | Feature |
|---|---|
| 📂 | **Drag & drop** `.lua` file upload |
| 🔍 | **Multi-repo search** across 5+ GitHub repositories |
| 📦 | **Automatic manifest download** from ManifestHub |
| 🔑 | **Automatic depot keys** generation |
| ⚡ | **Integrated DepotDownloader** execution |
| 📊 | **Real-time download** progress tracking |
| 🎮 | **Steam Store API** integration — game names + cover art |
| 🌙 | **Dark / Light theme** support |
| ⚙️ | **Configurable** download location & GitHub token |
| 📝 | **Batch script export** (`.bat`) |
| 🔒 | **Fully self-contained** — DepotDownloaderMod embedded |

---

## 🚀 Quick Start

> **How it works — in 5 steps:**

1. 📥 **Download** the installer from [Releases](../../releases)
2. 📂 **Upload** your `.lua` file or search for a game
3. ✅ **Select** the depots and manifests you want
4. 🚀 **Click Download** — everything happens automatically
5. ✨ **Done!** Files are in your configured download folder

---

## 💻 System Requirements

| | Requirement | Details |
|---|---|---|
| 💻 | **Operating System** | Windows 10 / 11 (64-bit) |
| ⚙️ | **Runtime** | [.NET 8.0 Runtime](https://dotnet.microsoft.com/en-us/download/dotnet/8.0) (for DepotDownloader) |
| 🌐 | **Network** | Internet connection |

---

## 📥 Installation

1. Head to the [**Releases**](../../releases) page
2. Download the latest `.exe` installer (NSIS) for Windows
3. Run the installer — installs per-user, **no admin required**
4. Launch **Steam Manifest Downloader** from the Start Menu

> [!NOTE]
> Make sure you have the [.NET 8.0 Runtime](https://dotnet.microsoft.com/en-us/download/dotnet/8.0) installed. The app will warn you if it's missing.

<details>
<summary><b>🔧 Build from Source</b></summary>

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Tauri CLI v2](https://v2.tauri.app/start/prerequisites/)

### Build commands

```bash
# Install Tauri CLI
cargo install tauri-cli --version "^2"

# Development mode
cargo tauri dev

# Production build (creates installer in src-tauri/target/release/bundle/)
cargo tauri build
```

</details>

---

## 🛠️ Tech Stack

<div align="center">

![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)
![Tauri](https://img.shields.io/badge/Tauri_v2-FFC131?style=for-the-badge&logo=tauri&logoColor=white)
![HTML5](https://img.shields.io/badge/HTML5-E34F26?style=for-the-badge&logo=html5&logoColor=white)
![CSS3](https://img.shields.io/badge/CSS3-1572B6?style=for-the-badge&logo=css3&logoColor=white)
![JavaScript](https://img.shields.io/badge/JavaScript-F7DF1E?style=for-the-badge&logo=javascript&logoColor=black)

</div>

| Layer | Technology |
|---|---|
| **Backend** | Rust, reqwest, tokio, serde |
| **Frontend** | HTML / CSS / JS (vanilla) |
| **Framework** | Tauri v2 |
| **Downloader** | DepotDownloaderMod (.NET 8) |

---

<details>
<summary><b>📁 Project Structure</b></summary>

```
DepoDownloaderWebApp/
├── public/                     # Frontend (HTML/CSS/JS)
│   ├── index.html              # Main UI
│   ├── css/style.css           # Styles & themes
│   └── js/app.js               # Application logic
├── src-tauri/
│   ├── src/
│   │   ├── main.rs             # Tauri entry point
│   │   ├── commands/           # Tauri command handlers
│   │   │   ├── download.rs     # Download orchestration
│   │   │   ├── search.rs       # Game search
│   │   │   ├── file_ops.rs     # File operations
│   │   │   ├── settings.rs     # App settings
│   │   │   ├── system.rs       # System utilities
│   │   │   └── window.rs       # Window controls
│   │   └── services/           # Business logic
│   │       ├── github_api.rs   # GitHub API client
│   │       ├── manifest_hub_api.rs
│   │       ├── steam_store_api.rs
│   │       ├── depot_runner.rs # DepotDownloaderMod runner
│   │       ├── lua_parser.rs   # .lua file parser
│   │       └── ...
│   ├── Cargo.toml              # Rust dependencies
│   └── tauri.conf.json         # Tauri configuration
├── DepotDownloaderMod/         # Embedded .NET tool
├── assets/                     # App icons
└── README.md
```

</details>

---

## 📄 License

<div align="center">

![License](https://img.shields.io/badge/license-GPL--2.0-blue?style=for-the-badge)

This project is licensed under the [GPL-2.0 License](LICENSE).

</div>

---

## 🙏 Credits & Acknowledgments

- **[DepotDownloaderMod](https://github.com/SteamAutoCracks/DepotDownloaderMod)** — Steam depot downloading engine
- **[ManifestHub](https://manifesthub1.filegear-sg.me/)** — Manifest file API
- **[Steam Store API](https://store.steampowered.com/api/)** — Game metadata & artwork
- **[Tauri](https://v2.tauri.app/)** — Desktop application framework

---

<div align="center">

Made with ❤️ and 🦀

</div>
