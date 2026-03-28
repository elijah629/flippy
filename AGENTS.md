# Repository Guidelines

## Project Structure & Module Organization
`flippy` is a small Rust CLI crate. The manifest lives in `Cargo.toml`, the pinned toolchain in `rust-toolchain.toml`, the Nix dev shell in `flake.nix`, and the current executable entrypoint in `src/main.rs`. Build artifacts are written to `target/` and should never be committed. As the CLI grows, move subcommands and validation logic into focused modules under `src/` instead of expanding `main.rs` indefinitely.

## Build, Test, and Development Commands
Use the Nix shell to get the pinned nightly toolchain and contributor utilities:

- `nix develop`: enter the shell with Rust, `clippy`, `rust-analyzer`, and `ripgrep`.
- `cargo run -- new /tmp/store`: run the CLI locally and exercise a subcommand.
- `cargo build`: compile the binary in debug mode.
- `cargo test`: run unit and integration tests.
- `cargo fmt -- --check`: verify Rust formatting.
- `cargo clippy -- -D warnings`: lint and fail on warnings.

Do not rely on a global Rust install; prefer the flake so everyone uses the same latest nightly toolchain.

## Coding Style & Naming Conventions
Follow idiomatic Rust defaults: 4-space indentation, `snake_case` for functions and variables, `PascalCase` for types, and `SCREAMING_SNAKE_CASE` for constants. Keep `clap` command definitions readable by grouping related subcommands and using one argument per line when signatures get long. Run `cargo fmt` before submitting changes.

## Testing Guidelines
There is no dedicated `tests/` directory yet. Add unit tests close to implementation with `#[cfg(test)]` for small parsing or validation logic, and add integration tests under `tests/` once command behavior expands. Name tests after the behavior they verify, for example `map_repo_add_rejects_invalid_url`. Treat `cargo test`, `cargo fmt -- --check`, and `cargo clippy -- -D warnings` as the minimum pre-PR checks.

## Commit & Pull Request Guidelines
The current history uses short, imperative, lowercase subjects such as `setup project; basic cli`. Keep commits concise and focused on one logical change. Pull requests should include a clear summary, note any CLI-facing behavior changes, link related issues, and include example invocations or terminal output when command UX changes.
