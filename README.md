# Chubby Mod Manager

<p align="center">
  <b>A modern, high-performance Mod Manager for Resident Evil games with integrated co-op host networking, bundled RE4R Multiplayer Enhanced Mod, and REFramework synchronization.</b>
</p>

<p align="center">
  <a href="https://github.com/iiEKiLLeR/chubby-mod-manager-releases/releases/latest/download/Chubby_Mod_Manager_Setup.exe">
    <img src="https://img.shields.io/badge/Download-Latest_Setup.exe-brightgreen?style=for-the-badge&logo=windows" alt="Download Latest Setup" />
  </a>
  <a href="https://github.com/iiEKiLLeR/chubby-mod-manager-releases/releases">
    <img src="https://img.shields.io/badge/Releases-v1.0.15-blue?style=for-the-badge" alt="Releases" />
  </a>
</p>

Written in 100% Rust with GPU-accelerated immediate mode graphics.

---

## Features

- **📂 Fluffy-Style Category Navigator (`CHARACTERS` & `OTHER`)**: Automatically classifies mods by game character (*Leon*, *Ada*, *Ashley*, *Luis*, *Claire*, *Jill*, *Wesker*, etc.) and category (*Co-op & Multiplayer*, *RE Framework*, *Weapons*, *Models*, *Textures*, *Animations*, *Music*, *Misc*) with live active indicators, mod counts, and custom category assignment.
- **🟢 Instant ON / OFF Mod & Category Toggles**: Turn individual mods, entire categories (`All ON` / `All OFF`), or your full library (`Turn All ON` / `Turn All OFF`) on and off with a single click.
- **📐 Resizable Panels & Auto-Save Layout**: Every panel across the app (Sidebar, Categories, Mod List, Mod Details & Preview, Co-op Center, and Options) can be resized via draggable splitters and automatically persists your layout in `Data/config.cfg`.
- **📦 Full `.pak` & Loose-File Compatibility**: Installs `.pak` archives as numbered `re_chunk_000.pak.patch_NNN.pak` patches with the 5-file `KPKA` dummy PAK barrier and automatically enables `LooseFileLoader_Enabled=true` in `re2_fw_config.txt`.
- **🎮 Direct Game Launch**: Launch your Resident Evil game directly through Steam or executable with one click (`▶ PLAY`).
- **✨ Subpixel Anti-Aliased UI**: Retinal, smooth vector UI elements, buttons, badges, and outlines rendered with analytical alpha feathering.
- **🧟 RE4R Multiplayer (Chubby Enhanced)**: Bundled with the Resident Evil 4 Remake Co-op mod:
  - Single OS mouse cursor fix (eliminates in-game double cursor).
  - Smooth 60 Hz enemy and partner animations for connected peers.
  - Auto-hiding immersive co-op HUD during gameplay.
  - Non-blocking inventory and pause menus.
  - Theme sync between the manager, REFramework (`dinput8.dll`), and co-op menus.
- **🎨 5 Built-in Themes**: Umbrella Crimson, Obsidian Cyan, Biohazard Emerald, Amethyst Noir, and Tactical Amber.
- **⚡ Fast Mod Operations**: Non-destructive `.zip`, `.7z`, `.rar`, and `.pak` archive management with atomic file swaps, backup restores, and update checks.
- **🤝 P2P Co-op Mod Syncing**: Built-in HTTP and TCP transfer engine allowing peers to sync matching mod lists with the host.

---

## Screenshots

### Mod Library, Category Navigator & Active Toggles
<p align="center">
  <img src="https://raw.githubusercontent.com/iiEKiLLeR/chubby-mod-manager-releases/main/screenshots/mods.png" alt="Chubby Mod Manager - Mods Library & Categories" width="95%" />
</p>

### Co-op Host & Network Center
<p align="center">
  <img src="https://raw.githubusercontent.com/iiEKiLLeR/chubby-mod-manager-releases/main/screenshots/coop.png" alt="Chubby Mod Manager - Co-op Center" width="95%" />
</p>

### Manager Configuration & Options
<p align="center">
  <img src="https://raw.githubusercontent.com/iiEKiLLeR/chubby-mod-manager-releases/main/screenshots/options.png" alt="Chubby Mod Manager - Options" width="95%" />
</p>

### What's New & Patch Notes Modal
<p align="center">
  <img src="https://raw.githubusercontent.com/iiEKiLLeR/chubby-mod-manager-releases/main/screenshots/patch-notes.png" alt="Chubby Mod Manager - Patch Notes" width="95%" />
</p>

### Themes Showcase

| Umbrella Crimson | Obsidian Cyan |
| :---: | :---: |
| <img src="https://raw.githubusercontent.com/iiEKiLLeR/chubby-mod-manager-releases/main/screenshots/theme-umbrella_crimson.png" width="100%" /> | <img src="https://raw.githubusercontent.com/iiEKiLLeR/chubby-mod-manager-releases/main/screenshots/theme-obsidian_cyan.png" width="100%" /> |

| Biohazard Emerald | Amethyst Noir |
| :---: | :---: |
| <img src="https://raw.githubusercontent.com/iiEKiLLeR/chubby-mod-manager-releases/main/screenshots/theme-biohazard_emerald.png" width="100%" /> | <img src="https://raw.githubusercontent.com/iiEKiLLeR/chubby-mod-manager-releases/main/screenshots/theme-amethyst_noir.png" width="100%" /> |

| Tactical Amber |
| :---: |
| <img src="https://raw.githubusercontent.com/iiEKiLLeR/chubby-mod-manager-releases/main/screenshots/theme-tactical_amber.png" width="50%" /> |

---

## Building from Source

### Prerequisites
- [Rust](https://rustup.rs/) (edition 2024 / latest stable)
- Windows 10 / 11 (64-bit)

### Build Manager Binary
```powershell
cargo build --release --bin chubby_mod_manager
```

### Build Single-File Setup Installer
```powershell
Copy-Item target\release\chubby_mod_manager.exe assets\chubby_mod_manager_payload.exe
cargo build --release --bin Chubby_Mod_Manager_Setup
```

### Run Self-Test Suite
```powershell
cargo run -- --self-test selftest_out.json
```

---

## Download Releases

Prebuilt binaries and standalone Windows installers are available on the [Releases Repository](https://github.com/iiEKiLLeR/chubby-mod-manager-releases/releases).

---

## License

This project is licensed under the [MIT License](LICENSE).
