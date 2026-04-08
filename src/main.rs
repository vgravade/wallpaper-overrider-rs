// In release builds, suppress the console window for the GUI mode.
// Broker-mode callers should use the process exit code to detect failure.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use std::path::PathBuf;

mod app;
mod elevation;
mod i18n;
mod registry;
mod wallpaper_style;

use i18n::Language;
use wallpaper_style::WallpaperStyle;

/// Override Windows desktop wallpaper settings via registry policies.
///
/// Without arguments the graphical interface is shown (for the current user).
/// Passing --target-sid enables **broker mode**: the wallpaper is written for
/// another user's hive (HKEY_USERS\<SID>) and administrator privileges are
/// required — the process will request UAC elevation automatically if needed.
#[derive(Parser)]
#[command(author, version)]
struct Cli {
    /// Target user SID — writes to HKEY_USERS\<SID> instead of HKCU.
    /// Requires administrator privileges (UAC prompt will appear if needed).
    #[arg(long)]
    target_sid: Option<String>,

    /// Absolute path to the wallpaper image (required in broker mode).
    #[arg(long)]
    wallpaper: Option<PathBuf>,

    /// Display style: CENTER, TILE, STRETCH, FIT, FILL, SPAN  — or 0..5
    /// (required in broker mode).
    #[arg(long)]
    style: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // ── Broker / headless mode ────────────────────────────────────────────
    if let Some(sid) = cli.target_sid {
        let wallpaper_path = cli
            .wallpaper
            .ok_or_else(|| anyhow::anyhow!("--wallpaper is required in broker mode"))?;
        let style_str = cli
            .style
            .ok_or_else(|| anyhow::anyhow!("--style is required in broker mode"))?;
        let style: WallpaperStyle = style_str.parse()?;

        // Broker mode writes to HKEY_USERS which requires admin privileges.
        // If we are not already elevated, re-launch ourselves with UAC and exit.
        if !elevation::is_elevated() {
            elevation::relaunch_elevated()?;
            return Ok(());
        }

        anyhow::ensure!(
            wallpaper_path.is_file(),
            "Wallpaper file not found: {}",
            wallpaper_path.display()
        );

        registry::set_wallpaper_for_sid(&sid, &wallpaper_path, style)?;
        // Session refresh is intentionally omitted: we cannot call
        // SystemParametersInfo for another user's logon session.
        return Ok(());
    }

    // ── GUI mode ──────────────────────────────────────────────────────────
    let lang = Language::detect();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(lang.app_title())
            .with_inner_size([460.0, 530.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        lang.app_title(),
        options,
        Box::new(move |_cc| Ok(Box::new(app::WallpaperApp::new(lang)))),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(())
}
