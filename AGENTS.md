# AGENTS.md

Rules for human/AI agent contributing to `wallpaper-overrider-rs`.
Language: **English** in code, logs, docs, commit messages.

## Project

`wallpaper-overrider-rs` forces Windows desktop wallpaper using registry policy.
Modes:
- **GUI Mode** (default): Win32 UI (current HKCU user).
- **Broker / Headless Mode** (`--target-sid`): writes HKEY_USERS\<SID>. Need admin, triggers UAC.

Specs:
- **Rust edition 2024**: prefer `let-chains`, `bool::then`, `is_some_and`, `let else`.
- `release` profile: `lto = true`, `codegen-units = 1`, `panic = "abort"`, `strip = true`. No `panic = unwind`.
- Deps: `winreg`, `clap`, `anyhow`, `image`, `windows-sys`, `windows`.

## Mandatory Verification

Run at root before submit:
```pwsh
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```
Run `cargo fmt` systematically. Zero warnings/errors allowed. Inline `#[allow(...)]` is strictly forbidden.

### Clippy

Lints (`pedantic`, `nursery`, and strict `restriction` lints) are centrally managed in `Cargo.toml` under `[lints.clippy]`.
- **Zero Inline Allows**: Inline `#[allow(...)]` is strictly forbidden in the entire repository (including tests and build scripts). All lint configurations must be managed centrally in `Cargo.toml`.
- **Clean Configuration**: No redundant or unused `allow` rules are allowed in `Cargo.toml`.
- **Strict Build Script**: `build.rs` is subject to the same zero-warning and zero-allow policy.
- **Priority**: `pedantic` and `nursery` must have `priority = -1` in `Cargo.toml`.

## Reliability & Win32 Resources

No async (no Tokio). Synchronous code, native Win32 loop, `std::thread`.

### RAII / Drop

Manage handles (`HBRUSH`, `HFONT`, etc.) strictly. Destroy/release when done (`DeleteObject`, `ReleaseDC`, etc.).
Prefer Rust `Drop` (RAII) for automatic release (e.g. `OwnedBrush`, `OwnedHandle`, `LocalWideString`).

### Errors

- Boundaries (`main`, threads, GUI): `anyhow::Result`.
- I/O: use `std::io::Error::other(e)` (not `new(ErrorKind::Other, e)`).

### Safe / Unsafe

- `unsafe` only for Win32 API (`windows`, `windows-sys`).
- Wrap `unsafe` in safe functions.
- Init structs: `Struct { field: val, ..Default::default() }` or `std::mem::zeroed()`. No field reassign.

### Print & Panic

`unwrap()`, `expect()`, `panic!()`, `println!()`, `eprintln!()`:
- **GUI / Main**: Forbidden. Use GUI dialogs or return `anyhow` error.
- **Broker / Headless**: stdout/stderr allowed.
- **Tests**: `panic!()` (via assertions like `assert_eq!`) is allowed. `unwrap()` and `expect()` are forbidden (as they would trigger Clippy warnings which cannot be bypassed inline). Use safe pattern matching or assertions like `assert!(opt.is_some())`.

## Idiomatic Rust

- `if cond && let Ok(x) = f()` (no nested if).
- `opt.is_some_and()` / `opt.map_or()` (no `map().unwrap_or()`).
- `std::io::Error::other(e)`.
- `let else` for early return.
- `format!("{val:#?}")`.

## Platform

- Target: Windows.
- Windows code: `#[cfg(windows)]`.
- Other OS (Linux/macOS): stub functions under `#[cfg(not(windows))]` returning defaults.

## Workflow

1. Read rules.
2. Edit code.
3. Run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
4. Zero errors/warnings allowed.

