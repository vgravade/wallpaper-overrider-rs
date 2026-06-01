# Wallpaper Overrider

Wallpaper Overrider is a small Windows utility that forces the desktop wallpaper through the Windows desktop wallpaper policy registry keys.

The app starts as a graphical picker by default. It first tries to write the current user's policy values without elevation. If that fails, it relaunches a small elevated broker with UAC and writes the same values under the current user's SID in `HKEY_USERS`.

## Behavior

- Writes `Wallpaper` and `WallpaperStyle` under `Software\Microsoft\Windows\CurrentVersion\Policies\System`.
- Uses `HKEY_CURRENT_USER` in normal GUI mode when possible.
- Uses `HKEY_USERS\<SID>` in broker mode, which requires administrator privileges.
- Refreshes the current desktop session after a successful GUI apply.
- Does not refresh another user's logon session from broker mode.
- Supports JPEG, PNG, and BMP input for preview and selection.
- JPEG or BMP files are recommended for the final policy value because Microsoft's wallpaper policy documentation specifies those formats.

## Wallpaper Styles

| Code | CLI name | Windows style |
| --- | --- | --- |
| `0` | `CENTER` | Center |
| `1` | `TILE` | Tile |
| `2` | `STRETCH` | Stretch |
| `3` | `FIT` | Fit |
| `4` | `FILL` | Fill |
| `5` | `SPAN` | Span |

## Usage

Run the GUI:

```powershell
cargo run --release
```

Run broker mode directly:

```powershell
wallpaper-overrider.exe --target-sid S-1-5-21-... --wallpaper C:\Path\wallpaper.jpg --style FILL
```

`--style` accepts either the numeric code or the CLI name.

## Development

```powershell
cargo fmt --all -- --check
cargo build --locked
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo audit
```

Release builds are packaged by GitHub Actions when a `v*` tag is pushed.

## License

This project is licensed under GPL-3.0-only. See [LICENSE](LICENSE).
