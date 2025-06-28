# ❌ `qFlipper`, ✅ `flippy`

> Admit it, `qFlipper` sucks.

![Crates.io Version](https://img.shields.io/crates/v/flippy)
![Crates.io License](https://img.shields.io/crates/l/flippy)
![docs.rs](https://img.shields.io/docsrs/flippy)
![Crates.io MSRV](https://img.shields.io/crates/msrv/flippy)
![Crates.io License](https://img.shields.io/crates/l/flippy)
![Crates.io Downloads (recent)](https://img.shields.io/crates/dr/flippy)
![GitHub Repo stars](https://img.shields.io/github/stars/elijah629/flippy)

## What!?

`qFlipper` sucks! What could you mean… It is **the one and only** Flipper
control software produced by the one and only _Flipper Devices Inc_! How could
it be bad!!!?!?!

### Well…

- Proprietary and _barely_ open source as the codebase (pardon my language)
  FUCKING SUCKS.
- Overcomplicated codebase.
- The CLI is bad, barely documented, and not worth automating.
- It’s not `Rust` (okay, that was a joke, but honestly—who writes a new
  application in `C++`, `C`, and `Qt` nowadays?).
- **Slow**: they rolled their own `Protobuf` RPC interface, and they don’t even
  implement it correctly!!!! Pitiful.
- Last updated **1 year ago** just to fix Windows builds…
- The last **real code commit** was **over 2 years ago**!

## Why flippy?

To fix all of the above, and make the Flipper Zero more accessible to everyone.

- _READABLE_ open source, **100% Rust**.
- Ergonomic CLI with first class automation support.
- Built on top of my robust `flipper-rpc` library.
- Regularly maintained and tested on Linux (first class citizen here in the
  penguin empire).

## Features

- **Rust reimplementation** of the official Flipper RPC API
- **Automatic DB management**: keeps track of which files and repos you’ve
  pulled
- **Custom firmware channels**: any channel following the `directory.json` spec
  is supported
- **Interactive setup**: `flippy new` bootstraps a fresh project for you
- **Repo mapping** (`flippy map`): include or exclude paths in remote archives
- **Store management** (`flippy store fetch/clean`): bulk pull or wipe
  everything in one command.
- **Firmware control** (`flippy firmware set/update`): pin to or upgrade to any
  firmware you choose

## 🛠️ Installation

```bash
# Requires Rust ≥1.87.0
cargo install flippy
```

## 🚀 Quickstart

> ![NOTE] You must own a flipper (duh...) and have it plugged in **before**
> running commands that will modify it.

1. **Initialize** a new project in the current directory:

   ```bash
   flippy new my-flipper
   cd my-flipper
   ```

2. *_Add_ a new repository

   ```bash
   flippy repo add https://github.com/UberGuidoZ/Flipper flipper
   ```

3. **Map** entries from a repo to a DB on the flipper

   ```bash
   flippy map subghz flipper "Sub-GHz/**/*.sub"
   ```

4. **Fetch** all configured repos into your local store:

   ```bash
   flippy store fetch
   ```

5. **Upload** all fetched repos onto the flipper.

   ```bash
   flippy upload
   ```

6. **Set** a custom firmware channel:

   ```bash
   flippy firmware set unleashed@development
   ```

7. **Update** your Flipper device:

   ```bash
   flippy firmware update
   ```

## 📖 CLI Reference

```text
    _________  __        _________  ________  ________  __  __
   / _______/ / /       /___  ___/ /   ₀   / /   ₀   / / / / /
  / /______  / /          / /     / ______/ / ______/ / /_/ /
 / _______/ / /_____  ___/ /___  / /       / / ______ \__, /
/_/        /_______/ /________/ /_/       /_/ /___________/ vX.Y.Z

Automates upgrades and pulls remote databases, files, and firmware for the
Flipper Zero

Usage: flippy [OPTIONS] <COMMAND>

Commands:
  new       Interactive setup for a new flip
  upload    Upload local changes to remote storage
  map       Manages mappings in flip.toml files
  repo      Add or remove repositories
  firmware  Manages firmware settings
  store     Manages store files and updates repositories
  help      Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...  Verbosity level (-v, -vv, -vvv)
  -j, --json        Enables machine-readable JSON output
  -h, --help        Print help
  -V, --version     Print version
```

_(full details via `flippy <subcommand> --help`)_

## 📚 Documentation & Support

- **Docs**: [https://docs.rs/flippy](https://docs.rs/flippy)
- **Source**: [https://github.com/elijah629/flippy](https://github.com/elijah629/flippy)
- **License**: MIT

## 🤝 Contributing

Happy to accept issues and PRs!

1. Fork the repo
2. Create a feature branch (`git checkout -b feat/awesome`)
3. Commit your changes (`git commit -m "Add awesome feature"`)
4. Push (`git push origin feat/awesome`) and open a PR
