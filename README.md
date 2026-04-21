<div align="center">

# 🎮 Steam Manifest Downloader

**A sleek desktop app for downloading Steam game depots using manifest data.**

![Version](https://img.shields.io/badge/version-1.3.0-blue)
![License](https://img.shields.io/badge/license-GPL--2.0-blue)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux-0078D6?logo=windows)
![Built with](https://img.shields.io/badge/built_with-Rust-dea584?logo=rust)
![Tauri](https://img.shields.io/badge/Tauri_v2-FFC131?logo=tauri&logoColor=white)
![Downloads](https://img.shields.io/github/downloads/MCbabel/Steam-Manifest-Downloader/total?color=brightgreen)
[![Lines of Code](https://img.shields.io/endpoint?url=https%3A%2F%2Ftokei.kojix2.net%2Fbadge%2Fgithub%2FMCbabel%2FSteam-Manifest-Downloader%2Flines)](https://tokei.kojix2.net/github/MCbabel/Steam-Manifest-Downloader)

Upload `.lua` files, search across GitHub repos, and let the app handle manifests, depot keys, and downloads — all in one click.

</div>

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
| 🔍 | **Multi-repo search** via Internet Archive |
| 📦 | **Automatic manifest download** from Internet Archive |
| 🔑 | **Automatic depot keys** generation |
| ⚡ | **Integrated DepotDownloader** execution |
| 📊 | **Real-time download** progress tracking |
| 🎮 | **Steam Store API** integration — game names + cover art |
| 🌙 | **Dark / Light theme** support |
| ⚙️ | **Configurable** download location & GitHub token |
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
| 💻 | **Operating System** | Windows 10 / 11 (64-bit) or a modern Linux distro (glibc ≥ 2.35) |
| ⚙️ | **Runtime (Windows)** | [.NET 9.0 Runtime](https://dotnet.microsoft.com/en-us/download/dotnet/9.0) — required by DepotDownloader |
| 📦 | **Runtime (Linux)** | `webkit2gtk-4.1`, `libayatana-appindicator3`, `librsvg2` (install commands below) |
| 🌐 | **Network** | Internet connection |

---

## 📥 Installation

### 🪟 Windows

1. Head to the [**Releases**](../../releases) page and download the latest `.exe` installer (NSIS)
2. Run the installer — installs per-user, **no admin required**
3. Launch **Steam Manifest Downloader** from the Start Menu

> [!NOTE]
> Make sure you have the [.NET 9.0 Runtime](https://dotnet.microsoft.com/en-us/download/dotnet/9.0) installed. The app will warn you on first launch if it's missing.

### 🐧 Linux

Download the latest `.AppImage` from [**Releases**](../../releases).

Tauri apps on Linux don't bundle their own browser engine — they render the UI through the system **WebKitGTK**. Install the runtime for your distro:

<details>
<summary><b>Ubuntu / Debian</b> (22.04+ / 12+)</summary>

```bash
sudo apt install libwebkit2gtk-4.1-0 libayatana-appindicator3-1 librsvg2-2
```
</details>

<details>
<summary><b>Arch / CachyOS / Manjaro</b></summary>

```bash
sudo pacman -S webkit2gtk-4.1 libayatana-appindicator librsvg
```
</details>

<details>
<summary><b>Fedora</b></summary>

```bash
sudo dnf install webkit2gtk4.1 libappindicator-gtk3 librsvg2
```
</details>

<details>
<summary><b>openSUSE</b> (Tumbleweed / Leap 15.6+)</summary>

```bash
sudo zypper install libwebkit2gtk-4_1-0 libayatana-appindicator3-1 librsvg-2-2
```
</details>

Then make the AppImage executable and launch it:

```bash
chmod +x Steam\ Manifest\ Downloader_*_amd64.AppImage
./Steam\ Manifest\ Downloader_*_amd64.AppImage
```

> [!NOTE]
> On **NixOS**, portable binaries can't find system libs through the normal loader paths. Launch via `steam-run ./Steam\ Manifest\ Downloader_*_amd64.AppImage`, or wrap the binary in a Nix derivation that lists `webkitgtk_4_1`, `libayatana-appindicator`, `librsvg` and `gtk3` as build inputs.

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

![Rust](https://img.shields.io/badge/Rust-000000?logo=rust&logoColor=white)
![Tauri](https://img.shields.io/badge/Tauri_v2-FFC131?logo=tauri&logoColor=white)
![HTML5](https://img.shields.io/badge/HTML5-E34F26?logo=html5&logoColor=white)
![CSS3](https://img.shields.io/badge/CSS3-1572B6?logo=css3&logoColor=white)
![JavaScript](https://img.shields.io/badge/JavaScript-F7DF1E?logo=javascript&logoColor=black)

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

![License](https://img.shields.io/badge/license-GPL--2.0-blue)

This project is licensed under the [GPL-2.0 License](LICENSE).

</div>

---

## 🙏 Credits & Acknowledgments

- **[DepotDownloaderMod](https://github.com/SteamAutoCracks/DepotDownloaderMod)** — Steam depot downloading engine
- **[Steam Store API](https://store.steampowered.com/api/)** — Game metadata & artwork
- **[Tauri](https://v2.tauri.app/)** — Desktop application framework

---

<div align="center">

Made with ❤️ and 🦀

</div>
