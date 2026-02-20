<div align="center">

# 🎮 Steam Manifest Downloader

**A sleek desktop app for downloading Steam game depots using manifest data.**

![Version](https://img.shields.io/badge/version-1.1.0-blue?style=for-the-badge)
![License](https://img.shields.io/badge/license-GPL--2.0-blue?style=for-the-badge)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-0078D6?style=for-the-badge&logo=windows)
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
<summary><b>🔧 Building from Source</b></summary>

### Prerequisites

- **Rust** (latest stable) + **Cargo** — [Install via rustup](https://rustup.rs/)
- **Tauri CLI** — `cargo install tauri-cli`
- **.NET SDK 9.0** — Only needed if building DepotDownloaderMod from source ([Download](https://dotnet.microsoft.com/en-us/download/dotnet/9.0))
- **Linux additional:** `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf` (for AppImage)

---

### Step 1: Building DepotDownloaderMod (optional)

The project embeds DepotDownloaderMod binaries at compile time. **Pre-built versions are already included** in the repo:

- `DepotDownloaderMod-Windows/` — Windows build (framework-dependent, requires .NET runtime)
- `DepotDownloaderMod-linux-full/` — Linux build (self-contained, no runtime needed)

If you want to build DepotDownloaderMod yourself:

**Source:** [github.com/SteamAutoCracks/DepotDownloaderMod](https://github.com/SteamAutoCracks/DepotDownloaderMod)

#### Windows (framework-dependent)

```bash
git clone https://github.com/SteamAutoCracks/DepotDownloaderMod.git
cd DepotDownloaderMod
dotnet publish -c Release -o ./publish-windows
```

Copy **all** files from `publish-windows/` to `DepotDownloaderMod-Windows/` in this project:

- `DepotDownloaderMod.exe`
- `DepotDownloaderMod.dll`
- `DepotDownloaderMod.deps.json`
- `DepotDownloaderMod.runtimeconfig.json`
- `SteamKit2.dll`
- `protobuf-net.Core.dll`
- `protobuf-net.dll`
- `QRCoder.dll`
- `System.IO.Hashing.dll`
- `ZstdSharp.dll`

#### Linux (self-contained, NO trimming)

```bash
git clone https://github.com/SteamAutoCracks/DepotDownloaderMod.git
cd DepotDownloaderMod
dotnet publish -c Release -r linux-x64 --self-contained true \
    -p:PublishSingleFile=true -o ./publish-linux
```

> [!CAUTION]
> **Do NOT use `-p:PublishTrimmed=true`** — .NET trimming removes reflection metadata needed by SteamKit2/protobuf-net, causing "A task was canceled" errors at runtime.

Copy `publish-linux/DepotDownloaderMod` to `DepotDownloaderMod-linux-full/DepotDownloaderMod` in this project.

---

### Step 2: Building the Tauri App

#### Windows

```bash
cargo tauri build
```

Output:
- **NSIS installer:** `src-tauri/target/release/bundle/nsis/`
- **Portable executable:** `src-tauri/target/release/steam-manifest-downloader.exe`

#### Linux (Arch/CachyOS/etc.)

```bash
NO_STRIP=true APPIMAGE_EXTRACT_AND_RUN=1 cargo tauri build
```

Output: `src-tauri/target/release/bundle/appimage/Steam Manifest Downloader_1.0.0_amd64.AppImage`

> [!NOTE]
> `NO_STRIP=true` prevents stripping symbols from the embedded .NET binary. `APPIMAGE_EXTRACT_AND_RUN=1` is needed on some distros for the AppImage bundler.

---

### Project Structure (for reference)

The `include_bytes!` macro in `src-tauri/src/services/embedded_tools.rs` embeds the DDM binaries at compile time:

- **Windows build** reads from `DepotDownloaderMod-Windows/`
- **Linux build** reads from `DepotDownloaderMod-linux-full/`

> [!IMPORTANT]
> The DDM binary files **must be in place before** running `cargo tauri build`. The Rust compiler reads them via `include_bytes!` at compile time — if the files are missing, the build will fail.

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
