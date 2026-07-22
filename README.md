# Pixgine

**Pixgine** is a modern 2D game engine written in Rust.

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/Kulvrik07/Pixgine)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

## Work in Progress

This project is under active development. Don't expect full functionality or a stable API yet.

## Project Structure

```
Pixgine/
├── engine/     # Core game engine (ECS, rendering, physics, audio)
├── editor/     # Visual editor for level and asset management
├── game/       # Example game as a demo
└── assets/     # Textures, tilesets, scenes, scripts
```

## Features (planned)

- ECS architecture (Entity-Component-System)
- 2D rendering with wgpu
- Physics (collision, movement)
- Audio system
- Scripting (via WASM or Lua)
- Visual editor (in development)

## Technologies

- **Language:** Rust
- **Rendering:** wgpu
- **UI:** egui
- **Build system:** Cargo

## Usage

### Building the engine

```bash
cd engine
cargo build
```

### Building the editor

```bash
cd editor
cargo build
```

### Running the example game

```bash
cd game
cargo run
```

## License

This project is licensed under the MIT License.

## Contributing

Contributions are welcome. Please open an issue before making any large changes.

---

Built with love, in Rust.
