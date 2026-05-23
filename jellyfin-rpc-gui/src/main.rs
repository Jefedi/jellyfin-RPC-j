#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod rpc;
mod settings;
mod tray;
#[cfg(windows)]
mod autostart;

use app::App;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .init();

    let settings = settings::Settings::load();
    let start_hidden = settings.start_minimized;

    let icon = build_icon();

    let viewport = egui::ViewportBuilder::default()
        .with_title("Jellyfin-RPC")
        .with_inner_size([900.0, 680.0])
        .with_min_inner_size([720.0, 540.0])
        .with_icon(icon)
        .with_visible(!start_hidden);

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Jellyfin-RPC",
        native_options,
        Box::new(|cc| Ok(Box::new(App::new(cc, settings)))),
    )
}

fn build_icon() -> egui::IconData {
    let size: u32 = 64;
    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let cx = x as f32 - size as f32 / 2.0;
            let cy = y as f32 - size as f32 / 2.0;
            let dist = (cx * cx + cy * cy).sqrt();
            let radius = size as f32 / 2.0 - 1.0;
            if dist <= radius {
                let t = (x as f32 + y as f32) / (size as f32 * 2.0);
                let r = (90.0 + 120.0 * t) as u8;
                let g = (40.0 + 60.0 * (1.0 - t)) as u8;
                let b = (170.0 + 60.0 * t) as u8;
                rgba.extend_from_slice(&[r, g, b, 255]);
            } else {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            }
        }
    }
    egui::IconData {
        rgba,
        width: size,
        height: size,
    }
}
