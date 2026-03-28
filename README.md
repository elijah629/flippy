# flippy

`flippy` is a small CLI for keeping a Flipper Zero setup in a project directory.
It is aimed at people who want a text-based `flip.toml`, repeatable local or remote
sources, and something that fits cleanly into shell scripts, Git, and Nix-based workflows.

This repository is still early. The current CLI can:

- create a project directory with `flip.toml` and `store/`
- set a firmware source in `flip.toml`
- add repository sources to `flip.toml`
- validate local sources and write a fetch plan to `store/fetch-plan.txt`

It does not try to replace the normal Flipper desktop workflow.

## Why Use It Alongside qFlipper

qFlipper is still the standard desktop tool for day-to-day device management. It gives
you a GUI, firmware installation, file browsing, logs, and a direct connection to the
device.

`flippy` is useful in a different place:

- you want the setup described in files that can live in Git
- you want to point at local build artifacts as well as remote URLs
- you want a CLI that can be driven from scripts, CI jobs, or Nix shells
- you want to keep notes about which firmware and content sources belong to a given SD layout

A practical model is:

- use qFlipper for direct device interaction
- use `flippy` to manage the project-side definition of what should exist

## Example

Create a project:

```bash
flippy new ./sd
cd ./sd
```

Set firmware from a local build:

```bash
flippy map firmware set path://../fw.tgz
```

Add a repository from GitHub:

```bash
flippy map repo add https://github.com/Lucaslhm/Flipper-IRDB irdb
```

Add a repository using an SCP-style Git remote:

```bash
flippy map repo add git@github.com:flipperdevices/flipperzero-firmware.git firmware
```

Create the store directory and validate the current configuration:

```bash
flippy store create
flippy store fetch --optimize
```

`store fetch` currently validates local paths and writes a fetch plan. It does not yet
clone repositories or download firmware artifacts.

## `flip.toml`

Example configuration:

```toml
name = "sd"
firmware = "path://../fw.tgz"

[repositories.irdb]
source = "https://github.com/Lucaslhm/Flipper-IRDB"

[repositories.firmware]
source = "git@github.com:flipperdevices/flipperzero-firmware.git"
```

Firmware values can be:

- `ofw`
- `unleashed`
- `momentum`
- `rogue-free`
- `http(s)` URLs
- `file://` URLs
- local paths such as `fw.tgz`, `/full/path/fw.tgz`, or `path://../fw.tgz`

Repository values can be:

- `http(s)` URLs
- `ssh://` URLs
- `git://` URLs
- SCP-style Git remotes such as `git@github.com:owner/repo.git`
- local paths such as `path://../repo`

## Development

Use the pinned toolchain from the flake:

```bash
nix develop
cargo fmt -- --check
cargo test
cargo clippy -- -D warnings
```
