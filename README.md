# Cut Craft

A personal educational project to create an open-source alternative to Vectric Aspire CAD software.

## Features

- Cross-platform 3D modeling and CNC toolpath generation
- Works the way Vectric Aspire 12 does, built clean-room from a behaviour spec (`re/SPEC.md`);
  no Vectric code or assets are in this repository
- Built with Rust for performance and safety
- AMD graphics support enabled

## Technical Stack

- Rust (workspace, edition 2024)
- egui / eframe on wgpu (Vulkan on AMD) for the UI
- serde + schemars: command parameters, saving, generated forms

## Project Structure

```
crates/core/              document model, invertible changes, the Command contract
crates/commands/<name>/   one crate per command: params, run(), tests
crates/formats/<name>/    one crate per file importer or exporter
crates/post/              post-processors: toolpaths to machine G-code
crates/app/               the desktop UI (cut-craft binary)
scripts/                  new-command scaffolding, the desktop launcher
re/                       reverse-engineering notes and scripts (re/SPEC.md)
docs/                     the tool inventory, the handoff, design docs
```

What every tool is built from and what is still to build: `docs/tool-inventory.md`.
Architecture and backlog: `docs/superpowers/specs/2026-10-23-command-architecture-design.md`.

## Checks

Every commit runs `.githooks/pre-commit`: `cargo fmt --check`, `cargo clippy` with warnings
as errors (pedantic), and `cargo nextest`. Run it by hand the same way. After cloning,
enable it with `git config core.hooksPath .githooks`.

## Building

```bash
cargo build
```

## Running

```bash
cargo run
```

To start it from the desktop's application menu, build it for release and install a launcher
(re-run after pulling changes; `--remove` takes the launcher out again):

```bash
scripts/install-launcher
```

## AMD Graphics Support

This project is designed to work with AMD graphics cards through:
- Vulkan backend support in wgpu
- Cross-platform graphics API compatibility
- Proper device selection for different GPU types

## License

This project is for educational purposes only.