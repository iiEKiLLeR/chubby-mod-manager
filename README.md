# Chubby Mod Manager

A modern, high-performance Mod Manager for Resident Evil games with integrated co-op host networking, bundled RE4R Multiplayer Enhanced Mod, and REFramework synchronization.

Written in 100% Rust with GPU-accelerated immediate mode graphics.

---

## Features

- **🎮 Direct Game Launch**: Launch your Resident Evil game directly through Steam or executable with one click (`▶ PLAY`).
- **✨ Subpixel Anti-Aliased UI**: Retinal, smooth vector UI elements, buttons, badges, and outlines rendered with analytical alpha feathering.
- **🧟 RE4R Multiplayer (Chubby Enhanced)**: Bundled with the Resident Evil 4 Remake Co-op mod:
  - Single OS mouse cursor fix (eliminates in-game double cursor).
  - Smooth 60 Hz enemy and partner animations for connected peers.
  - Auto-hiding immersive co-op HUD during gameplay.
  - Non-blocking inventory and pause menus.
  - Theme sync between the manager, REFramework (`dinput8.dll`), and co-op menus.
- **🎨 5 Built-in Themes**: Obsidian Cyan, Amethyst Noir, Biohazard Emerald, Umbrella Crimson, and Tactical Amber.
- **⚡ Fast Mod Operations**: Non-destructive zip, 7z, rar, and pak archive management with atomic file swaps, backup restores, and update checks.
- **🤝 P2P Co-op Mod Syncing**: Built-in HTTP and TCP transfer engine allowing peers to sync matching mod lists with the host.

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
# Copy the manager release executable as installer payload
Copy-Item target\release\chubby_mod_manager.exe src\bin\chubby_mod_manager.exe
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
