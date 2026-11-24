# TerraCL

[![CI](https://github.com/arraynothere/terracl/actions/workflows/ci.yml/badge.svg)](https://github.com/arraynothere/terracl/actions/workflows/ci.yml)

Lightweight terminal code editor written in Rust. Line numbers, file tree sidebar, syntax highlighting and multi-encoding support. Editor only, not a compiler.

Runs on Linux, Termux and Windows Command Prompt.

## Install

### Debian, Ubuntu (apt)

```
sudo install -d -m 0755 /usr/share/keyrings
curl -fsSL https://arraynothere.github.io/terracl/terracl.gpg | sudo tee /usr/share/keyrings/terracl.gpg > /dev/null
echo "deb [signed-by=/usr/share/keyrings/terracl.gpg] https://arraynothere.github.io/terracl stable main" | sudo tee /etc/apt/sources.list.d/terracl.list
sudo apt update
sudo apt install terracl
```

Architectures: amd64 and arm64. Built on Ubuntu 22.04, so it runs on Debian 12 and Ubuntu 22.04 or newer.

### Termux

```
pkg install rust git
git clone https://github.com/arraynothere/terracl
cd terracl
cargo install --path .
```

The apt repository above is not meant for Termux. A prebuilt Android aarch64 binary is attached to each release.

### Windows

Download `terracl-<version>-x86_64-pc-windows-msvc.zip` from the releases page, extract it and add the folder containing `terracl.exe` to `PATH`. Windows Terminal is recommended.

With Scoop, once a bucket is published:

```
scoop bucket add arraynothere https://github.com/arraynothere/scoop-bucket
scoop install terracl
```

### From source

```
cargo install --git https://github.com/arraynothere/terracl
```

## Usage

```
terracl [FILE|DIR]
```

## Shortcuts

| Key | Action |
| --- | --- |
| `^S` | Save (asks for a name on a new buffer) |
| `^Q` | Quit |
| `^F` / `F3` | Find / find next |
| `^R` | Find and replace all |
| `^Z` / `^Y` | Undo / redo |
| `^C` / `^X` / `^V` | Copy / cut / paste |
| `^A` | Select all |
| `^G` | Toggle mark mode, then move to select (useful without Shift) |
| `Shift` + arrows | Select |
| `^E` | Choose encoding used when saving |
| `^B` | Show or hide file tree |
| `^W` | Switch focus between tree and editor |

Screens narrower than 70 columns show the file tree full screen and hide it again after a file is opened.

Copy also sends the text to the system clipboard through OSC 52 on terminals that support it. Pasting from the system clipboard works through the terminal paste action (bracketed paste) and is inserted as a single undo step.

## Development

```
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

## Layout

```
src/
  main.rs
  app.rs
  editor/   buffer, syntax highlighting, text width helpers
  ui/       layout, sidebar, editor view, status bar, popups
  system/   file I/O and encodings, file tree, terminal setup
```

## License

MIT
