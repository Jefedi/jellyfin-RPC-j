use std::sync::mpsc::{channel, Receiver};
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    TrayIcon, TrayIconBuilder, TrayIconEvent,
};

pub enum TrayMessage {
    ShowWindow,
    ToggleWindow,
    Quit,
    StartRpc,
    StopRpc,
}

pub struct Tray {
    _icon: TrayIcon,
}

impl Tray {
    pub fn new(repaint: impl Fn() + Send + Sync + 'static) -> (Self, Receiver<TrayMessage>) {
        let (tx, rx) = channel::<TrayMessage>();

        let menu = Menu::new();
        let show = MenuItem::new("Afficher la fenêtre", true, None);
        let start = MenuItem::new("Démarrer le RPC", true, None);
        let stop = MenuItem::new("Arrêter le RPC", true, None);
        let quit = MenuItem::new("Quitter", true, None);
        let sep = PredefinedMenuItem::separator();
        let sep2 = PredefinedMenuItem::separator();

        menu.append(&show).ok();
        menu.append(&sep).ok();
        menu.append(&start).ok();
        menu.append(&stop).ok();
        menu.append(&sep2).ok();
        menu.append(&quit).ok();

        let show_id_c = show.id().clone();
        let start_id_c = start.id().clone();
        let stop_id_c = stop.id().clone();
        let quit_id_c = quit.id().clone();

        let icon = build_tray_icon();
        let tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Jellyfin-RPC")
            .with_icon(icon)
            .build()
            .expect("failed to build tray icon");

        // Spawn dispatcher thread that listens to menu events and tray clicks
        let tx_c = tx.clone();
        std::thread::spawn(move || {
            let menu_rx = MenuEvent::receiver();
            let tray_rx = TrayIconEvent::receiver();
            loop {
                let mut acted = false;
                while let Ok(ev) = menu_rx.try_recv() {
                    let msg = if ev.id == show_id_c {
                        Some(TrayMessage::ShowWindow)
                    } else if ev.id == start_id_c {
                        Some(TrayMessage::StartRpc)
                    } else if ev.id == stop_id_c {
                        Some(TrayMessage::StopRpc)
                    } else if ev.id == quit_id_c {
                        Some(TrayMessage::Quit)
                    } else {
                        None
                    };
                    if let Some(m) = msg {
                        let _ = tx_c.send(m);
                        repaint();
                        acted = true;
                    }
                }
                while let Ok(ev) = tray_rx.try_recv() {
                    if let TrayIconEvent::DoubleClick { .. } = ev {
                        let _ = tx_c.send(TrayMessage::ToggleWindow);
                        repaint();
                        acted = true;
                    }
                }
                if !acted {
                    std::thread::sleep(std::time::Duration::from_millis(150));
                }
            }
        });

        (Self { _icon: tray_icon }, rx)
    }
}

fn build_tray_icon() -> tray_icon::Icon {
    let size: u32 = 32;
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
    tray_icon::Icon::from_rgba(rgba, size, size).expect("invalid tray icon data")
}
