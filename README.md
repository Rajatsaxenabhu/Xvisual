# x-visual

A real-time terminal audio visualizer for Linux. Captures system audio via PipeWire and renders it live in the terminal.

## Modes

| Key | Mode |
|-----|------|
| `1` | Classic Orb — pulsing circle with beat ripples and radial spikes |
| `2` | Car Dashboard — tachometer arc driven by audio level |
| `3` | EQ Bars — symmetric frequency bars mirrored up and down |

## Requirements

- Linux (Ubuntu/Debian-based) with PipeWire
- A terminal with Unicode and true-color support (e.g. kitty, alacritty, wezterm)

## Quick Start (pre-built binary)

1. Download both `launch.sh` and the `xvisual-build/` folder from the releases page and place them in the same directory.

2. Make the launcher executable and run it:

```sh
chmod +x launch.sh
./launch.sh
```

`launch.sh` will automatically install all required dependencies (PipeWire, ALSA headers, Rust toolchain) and then launch the visualizer.

Press `1`, `2`, or `3` to select a mode. `Ctrl+C` to quit.

## Build from Source

```sh
cd xvisual
cargo build --release
./target/release/xvisual
```

## Dependencies

- [crossterm](https://github.com/crossterm-rs/crossterm) — terminal rendering
- [pipewire-rs](https://gitlab.freedesktop.org/pipewire/pipewire-rs) — audio capture
- [ringbuf](https://github.com/agerasev/ringbuf) — lock-free audio ring buffer
- [ctrlc](https://github.com/Detegr/rust-ctrlc) — signal handling
