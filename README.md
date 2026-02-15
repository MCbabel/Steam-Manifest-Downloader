# Steam Manifest Downloader

A desktop application for downloading Steam game depots using manifest data. Built with [Tauri v2](https://v2.tauri.app/) (Rust backend + HTML/CSS/JS frontend).

Upload `.lua` files containing Steam depot/manifest information, select the depots you need, and the app handles downloading manifest files and running [DepotDownloaderMod](https://github.com/SteamRE/DepotDownloader) to fetch game content.

![Screenshot](screenshots/main.png)

---

## Features

- **Lua file parsing** — Drag-and-drop or upload `.lua` files with Steam depot/manifest data
- **Automatic manifest resolution** — Fetches manifest files from GitHub repositories and [ManifestHub](https://manifesthub.filegear-sg.me/) API
- **Multi-depot downloads** — Download multiple depots in a single session
- **Depot key support** — Automatically extracts and uses depot decryption keys from lua data
- **Game info lookup** — Fetches game name and artwork from the Steam Store API
- **Download progress tracking** — Real-time progress events streamed from DepotDownloaderMod
- **Configurable download directory** — Choose where game files are saved
- **Embedded tools** — DepotDownloaderMod is bundled inside the application binary
- **Custom title bar** — Frameless window with integrated minimize/maximize/close controls
- **Cross-platform ready** — Windows primary, with Linux support scaffolded

---

## System Requirements

| Requirement | Details |
|---|---|
| **OS** | Windows 10 or later (64-bit) |
| **Runtime** | [.NET 8.0 Runtime](https://dotnet.microsoft.com/en-us/download/dotnet/8.0) (required by DepotDownloaderMod) |
| **Disk space** | ~50 MB for the app + space for downloaded game files |

---

## Installation

1. Go to the [Releases](../../releases) page
2. Download the latest `.exe` installer (NSIS) for Windows
3. Run the installer — it installs per-user, no admin required
4. Launch **Steam Manifest Downloader** from the Start Menu

> **Note:** Make sure you have the [.NET 8.0 Runtime](https://dotnet.microsoft.com/en-us/download/dotnet/8.0) installed. The app will warn you if it's missing.

---

## Usage

1. **Upload a Lua file** — Click the upload area or drag-and-drop a `.lua` file containing depot/manifest data
2. **Review depots** — The app parses the file and displays a list of depots with their manifest IDs and depot keys
3. **Select depots** — Check the depots you want to download (all are selected by default)
4. **Configure options** — Optionally set a GitHub token for higher API rate limits, or change the download directory
5. **Start download** — Click Download to begin. The app will:
   - Fetch manifest files from GitHub or ManifestHub
   - Generate a `steam.keys` file with depot decryption keys
   - Run DepotDownloaderMod for each depot
   - Show real-time progress
6. **Access files** — Downloaded game files are saved to the configured directory (default: `Documents/SteamDownloads/<AppID>`)

---

## Building from Source

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) (for Tauri CLI, if not using cargo-only workflow)
- [.NET 8.0 SDK](https://dotnet.microsoft.com/en-us/download/dotnet/8.0) (only if you need to rebuild DepotDownloaderMod)
- Platform-specific dependencies as listed in the [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/)

### Build

```bash
# Install Tauri CLI (if not already installed)
cargo install tauri-cli --version "^2"

# Development build
cargo tauri dev

# Production build (creates installer in src-tauri/target/release/bundle/)
cargo tauri build
```

### Project Structure

```
├── public/              # Frontend (HTML/CSS/JS)
├── src-tauri/
│   ├── src/
│   │   ├── main.rs              # Tauri app entry point
│   │   ├── commands/            # Tauri command handlers
│   │   └── services/            # Business logic (parsing, downloading, etc.)
│   ├── Cargo.toml               # Rust dependencies
│   └── tauri.conf.json          # Tauri configuration
├── DepotDownloaderMod/  # Bundled .NET tool (embedded via include_bytes!)
└── assets/              # App icons
```

---

## License

This project is licensed under the [MIT License](LICENSE).

---

## Credits

- **[DepotDownloaderMod](https://github.com/SteamRE/DepotDownloader)** — The underlying tool for downloading Steam depot content
- **[ManifestHub](https://manifesthub.filegear-sg.me/)** — API for fetching Steam manifest files
- **[Steam Store API](https://store.steampowered.com/api/)** — Game metadata and artwork
- **[Tauri](https://v2.tauri.app/)** — Framework for building the desktop application
