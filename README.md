# Pixgine

**Pixgine** ist ein moderner 2D-Game-Engine in Rust entwickelt.

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/Kulvrik07/Pixgine)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## 🚧 Work in Progress

Dieses Projekt ist in Aktiventwicklung. Erwartet keine vollständige Funktionalität oder stabile API.

## 📦 Projektstruktur

```
Pixgine/
├── engine/     # Kern-Game-Engine (ECS, Rendering, Physics, Audio)
├── editor/     # Visual Editor für Level- und Asset-Management
├── game/       # Beispiel-Game als Demo
└── assets/     # Texturen, Tilesets, Szenen, Skripte
```

## 🚀 Features (geplant)

- **ECS-Architektur** (Entity-Component-System)
- **2D-Rendering** mit wgpu
- **Physik** (Kollision, Bewegung)
- **Audio-System**
- **Scripting** (via WASM oder Lua)
- **Visual Editor** (in Entwicklung)

## 🛠️ Technologien

- **Sprache:** Rust
- **Rendering:** wgpu
- **UI:** egui
- **Build-System:** Cargo

## 📚 Nutzung

### Engine bauen

```bash
cd engine
cargo build
```

### Editor bauen

```bash
cd editor
cargo build
```

### Beispiel-Game starten

```bash
cd game
cargo run
```

## 📝 Lizenz

Dieses Projekt ist unter der MIT-Lizenz lizenziert.

## 🤝 Beiträge

Beiträge sind willkommen! Bitte Issues öffnen, bevor du große Änderungen vornimmst.

---

*Entwickelt mit ❤️ in Rust*