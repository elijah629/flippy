# Changelog

## 0.4.0

### BREAKING CHANGES

- Changes dependencies and their feature flags, potentially breaking consuming
  crates. (Why would you consume a binary? I don't know...)

### Fixed

- Use compile constants for coloring the `FLIPPY` logo.
- Git diffing now supplies mapping information to the operation handler
- Git diffing no longer attempts to remove a `/` prefix in operation paths since
  it does not exist.

### Added

- `json` feature for `tracing_subscriber`
- `json` logging via the `--json` flag
- `release-hyper` overkill performance profile
- Progress bar for `upload` operations
- Improved md5 difference algorithm which caches results for an entire
  directory. This is actually a common occurrence (reading every file in a
  directory one by one instead of all at once) since most directories stay the
  same per commit. May change this if it becomes inefficient, however it is fine
  for now (me).

### Removed

- `owo-colors`: I need to figure out a better way to colorize version info
  without compromising binary size and ease of use.
- `:kekw:` from package description
- `logo.txt`, please see src/art.rs for the latest art.
- Very large comments of legacy code. (~1k LOC)

### Changed

- Updated package versions for:
  - `unicode-width` → `0.2.1`
  - `flipper-rpc` → `0.9.3`
- Updated `flake.lock`
- Logo, the `Y` now joins into the `P` preceding it.
- First iteration of moving all logging statements with variables into the
  `level!(variable, "...")` form instead of `level!("...{variable}...")` for
  better JSON logging support.
