use crate::config::{
    Blacklist, ConfigBuilder, DiscordBuilder, DisplayField, DisplayOptionsBuilder, ImagesBuilder,
    ImgurBuilder, UsernameField,
};
use crate::rpc::{RpcCommand, RpcEvent, RpcWorker};
use crate::settings::Settings;
use crate::tray::{Tray, TrayMessage};
use eframe::egui;
use jellyfin_rpc::{Button, MediaType};
use std::sync::mpsc::Receiver;

const MAX_LOG_LINES: usize = 200;

pub struct App {
    config: ConfigBuilder,
    settings: Settings,
    rpc: RpcWorker,

    status: String,
    is_running: bool,
    now_playing: Option<String>,
    log: Vec<String>,

    tab: Tab,

    // tray
    _tray: Option<Tray>,
    tray_rx: Option<Receiver<TrayMessage>>,
    close_requested: bool,
    really_quit: bool,

    // form helpers (UI-only mirrors of nested optional fields)
    music_display_text: String,
    music_separator_text: String,
    music_status_display_type: String,

    movies_display_text: String,
    movies_separator_text: String,
    movies_status_display_type: String,

    episodes_display_text: String,
    episodes_separator_text: String,
    episodes_status_display_type: String,

    blacklist_media_types_text: String,
    blacklist_libraries_text: String,

    buttons_text: String,

    config_save_status: String,
    settings_save_status: String,
}

#[derive(PartialEq, Eq, Clone, Copy)]
enum Tab {
    Server,
    Discord,
    Display,
    Images,
    Background,
    Status,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, settings: Settings) -> Self {
        setup_visuals(&cc.egui_ctx);

        let config = ConfigBuilder::load_or_default();

        #[cfg(windows)]
        {
            if let Err(e) = crate::autostart::sync(settings.start_with_windows) {
                log::warn!("Échec de la synchronisation du démarrage automatique: {e}");
            }
        }

        let rpc = RpcWorker::spawn(cc.egui_ctx.clone());

        let repaint_ctx = cc.egui_ctx.clone();
        let (tray, tray_rx) = Tray::new(move || repaint_ctx.request_repaint());

        let mut app = Self {
            music_display_text: display_to_text(
                config.jellyfin.music.as_ref().and_then(|m| m.display.as_ref()),
            ),
            music_separator_text: config
                .jellyfin
                .music
                .as_ref()
                .and_then(|m| m.separator.clone())
                .unwrap_or_else(|| "-".into()),
            music_status_display_type: config
                .jellyfin
                .music
                .as_ref()
                .and_then(|m| m.status_display_type.clone())
                .unwrap_or_default(),
            movies_display_text: display_to_text(
                config.jellyfin.movies.as_ref().and_then(|m| m.display.as_ref()),
            ),
            movies_separator_text: config
                .jellyfin
                .movies
                .as_ref()
                .and_then(|m| m.separator.clone())
                .unwrap_or_else(|| "-".into()),
            movies_status_display_type: config
                .jellyfin
                .movies
                .as_ref()
                .and_then(|m| m.status_display_type.clone())
                .unwrap_or_default(),
            episodes_display_text: display_to_text(
                config.jellyfin.episodes.as_ref().and_then(|e| e.display.as_ref()),
            ),
            episodes_separator_text: config
                .jellyfin
                .episodes
                .as_ref()
                .and_then(|e| e.separator.clone())
                .unwrap_or_else(|| "-".into()),
            episodes_status_display_type: config
                .jellyfin
                .episodes
                .as_ref()
                .and_then(|e| e.status_display_type.clone())
                .unwrap_or_default(),
            blacklist_media_types_text: config
                .jellyfin
                .blacklist
                .as_ref()
                .and_then(|b| b.media_types.as_ref())
                .map(|v| {
                    v.iter()
                        .map(media_type_to_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default(),
            blacklist_libraries_text: config
                .jellyfin
                .blacklist
                .as_ref()
                .and_then(|b| b.libraries.as_ref())
                .map(|v| v.join(", "))
                .unwrap_or_default(),
            buttons_text: config
                .discord
                .as_ref()
                .and_then(|d| d.buttons.as_ref())
                .map(|btns| {
                    btns.iter()
                        .map(|b| format!("{}|{}", b.name, b.url))
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_else(|| "dynamic|dynamic\ndynamic|dynamic".to_string()),
            config,
            settings,
            rpc,
            status: "Inactif".into(),
            is_running: false,
            now_playing: None,
            log: Vec::new(),
            tab: Tab::Server,
            _tray: Some(tray),
            tray_rx: Some(tray_rx),
            close_requested: false,
            really_quit: false,
            config_save_status: String::new(),
            settings_save_status: String::new(),
        };

        if app.settings.autostart_rpc && !app.config.jellyfin.url.trim().is_empty() {
            app.push_log("Démarrage automatique du RPC".into());
            app.commit_form_to_config();
            app.rpc.send(RpcCommand::Start(app.config.clone()));
            app.is_running = true;
            app.status = "Démarrage…".into();
        }

        app
    }

    fn push_log(&mut self, line: String) {
        let ts = chrono_like_now();
        self.log.push(format!("[{ts}] {line}"));
        if self.log.len() > MAX_LOG_LINES {
            let cut = self.log.len() - MAX_LOG_LINES;
            self.log.drain(..cut);
        }
    }

    fn commit_form_to_config(&mut self) {
        // Music
        let music = DisplayOptionsBuilder {
            display: parse_display_text(&self.music_display_text),
            separator: empty_to_none(&self.music_separator_text),
            status_display_type: empty_to_none(&self.music_status_display_type),
        };
        self.config.jellyfin.music = if music == DisplayOptionsBuilder::default() {
            None
        } else {
            Some(music)
        };

        // Movies
        let movies = DisplayOptionsBuilder {
            display: parse_display_text(&self.movies_display_text),
            separator: empty_to_none(&self.movies_separator_text),
            status_display_type: empty_to_none(&self.movies_status_display_type),
        };
        self.config.jellyfin.movies = if movies == DisplayOptionsBuilder::default() {
            None
        } else {
            Some(movies)
        };

        // Episodes
        let episodes = DisplayOptionsBuilder {
            display: parse_display_text(&self.episodes_display_text),
            separator: empty_to_none(&self.episodes_separator_text),
            status_display_type: empty_to_none(&self.episodes_status_display_type),
        };
        self.config.jellyfin.episodes = if episodes == DisplayOptionsBuilder::default() {
            None
        } else {
            Some(episodes)
        };

        // Blacklist
        let mt: Vec<MediaType> = self
            .blacklist_media_types_text
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .filter_map(|s| str_to_media_type(&s))
            .collect();
        let libs: Vec<String> = self
            .blacklist_libraries_text
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if mt.is_empty() && libs.is_empty() {
            self.config.jellyfin.blacklist = None;
        } else {
            self.config.jellyfin.blacklist = Some(Blacklist {
                media_types: if mt.is_empty() { None } else { Some(mt) },
                libraries: if libs.is_empty() { None } else { Some(libs) },
            });
        }

        // Discord buttons
        let buttons: Vec<Button> = self
            .buttons_text
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .filter_map(|l| {
                let (name, url) = l.split_once('|')?;
                Some(Button::new(name.trim().to_string(), url.trim().to_string()))
            })
            .collect();
        if let Some(d) = self.config.discord.as_mut() {
            d.buttons = if buttons.is_empty() {
                None
            } else {
                Some(buttons)
            };
        }
    }

    fn handle_tray_messages(&mut self, ctx: &egui::Context) {
        let mut messages: Vec<TrayMessage> = Vec::new();
        if let Some(rx) = self.tray_rx.as_ref() {
            while let Ok(msg) = rx.try_recv() {
                messages.push(msg);
            }
        }
        for msg in messages {
            match msg {
                TrayMessage::ShowWindow => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                TrayMessage::ToggleWindow => {
                    // Best effort: just show. Without a reliable visibility flag we err on showing.
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                TrayMessage::Quit => {
                    self.really_quit = true;
                    self.rpc.send(RpcCommand::Shutdown);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                TrayMessage::StartRpc => {
                    self.commit_form_to_config();
                    self.rpc.send(RpcCommand::Start(self.config.clone()));
                    self.is_running = true;
                    self.status = "Démarrage…".into();
                    self.push_log("Démarrage du RPC (depuis la tray)".into());
                }
                TrayMessage::StopRpc => {
                    self.rpc.send(RpcCommand::Stop);
                    self.is_running = false;
                    self.status = "Arrêté".into();
                    self.push_log("Arrêt du RPC (depuis la tray)".into());
                }
            }
        }
    }

    fn handle_rpc_events(&mut self) {
        while let Ok(evt) = self.rpc.evt_rx.try_recv() {
            match evt {
                RpcEvent::Status(s) => {
                    self.status = s;
                }
                RpcEvent::Log(l) => {
                    self.push_log(l);
                }
                RpcEvent::NowPlaying(np) => {
                    self.push_log(format!("En cours: {np}"));
                    self.now_playing = Some(np);
                }
                RpcEvent::Cleared => {
                    self.now_playing = None;
                    self.push_log("Activité effacée".into());
                }
                RpcEvent::Error(e) => {
                    self.push_log(format!("ERREUR: {e}"));
                    self.is_running = false;
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_tray_messages(ctx);
        self.handle_rpc_events();

        // Intercept close request: if minimize_to_tray, hide instead of close.
        if ctx.input(|i| i.viewport().close_requested()) {
            if self.really_quit {
                // accept close
            } else if self.settings.minimize_to_tray {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                self.close_requested = false;
            } else {
                self.rpc.send(RpcCommand::Shutdown);
            }
        }

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading("🎬 Jellyfin-RPC");
                ui.add_space(12.0);
                status_pill(ui, &self.status, self.is_running);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.is_running {
                        if ui
                            .add(egui::Button::new("⏹  Arrêter").min_size(egui::vec2(110.0, 28.0)))
                            .clicked()
                        {
                            self.rpc.send(RpcCommand::Stop);
                            self.is_running = false;
                            self.status = "Arrêté".into();
                            self.push_log("Arrêt du RPC".into());
                        }
                    } else if ui
                        .add(egui::Button::new("▶  Démarrer").min_size(egui::vec2(110.0, 28.0)))
                        .clicked()
                    {
                        self.commit_form_to_config();
                        self.rpc.send(RpcCommand::Start(self.config.clone()));
                        self.is_running = true;
                        self.status = "Démarrage…".into();
                        self.push_log("Démarrage du RPC".into());
                    }
                });
            });
            ui.add_space(4.0);
        });

        egui::SidePanel::left("nav")
            .resizable(false)
            .exact_width(180.0)
            .show(ctx, |ui| {
                ui.add_space(10.0);
                for (tab, label, icon) in [
                    (Tab::Server, "Serveur", "🖥"),
                    (Tab::Discord, "Discord", "💬"),
                    (Tab::Display, "Affichage", "📺"),
                    (Tab::Images, "Images", "🖼"),
                    (Tab::Background, "Arrière-plan", "⚙"),
                    (Tab::Status, "Journal", "📋"),
                ] {
                    let selected = self.tab == tab;
                    let resp = ui.add_sized(
                        [ui.available_width(), 36.0],
                        egui::SelectableLabel::new(selected, format!("  {icon}  {label}")),
                    );
                    if resp.clicked() {
                        self.tab = tab;
                    }
                    ui.add_space(2.0);
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| match self.tab {
            Tab::Server => self.ui_server(ui),
            Tab::Discord => self.ui_discord(ui),
            Tab::Display => self.ui_display(ui),
            Tab::Images => self.ui_images(ui),
            Tab::Background => self.ui_background(ui),
            Tab::Status => self.ui_status(ui),
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}

impl App {
    fn ui_server(&mut self, ui: &mut egui::Ui) {
        section(ui, "Connexion au serveur Jellyfin", |ui| {
            egui::Grid::new("server_grid")
                .num_columns(2)
                .spacing([16.0, 10.0])
                .show(ui, |ui| {
                    ui.label("URL du serveur");
                    ui.text_edit_singleline(&mut self.config.jellyfin.url);
                    ui.end_row();

                    ui.label("Clé API");
                    ui.add(egui::TextEdit::singleline(&mut self.config.jellyfin.api_key).password(true));
                    ui.end_row();

                    ui.label("Utilisateur(s)");
                    let mut username_text = self.config.jellyfin.username.as_string();
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut username_text)
                                .hint_text("user1, user2"),
                        )
                        .changed()
                    {
                        self.config.jellyfin.username = UsernameField::String(username_text);
                    }
                    ui.end_row();
                });

            ui.add_space(8.0);
            let mut self_signed = self.config.jellyfin.self_signed_cert.unwrap_or(false);
            if ui
                .checkbox(&mut self_signed, "Accepter les certificats auto-signés")
                .changed()
            {
                self.config.jellyfin.self_signed_cert = Some(self_signed);
            }
        });

        section(ui, "Listes noires", |ui| {
            ui.label("Types de médias à ignorer (séparés par des virgules)");
            ui.text_edit_singleline(&mut self.blacklist_media_types_text);
            ui.small("Valeurs possibles : music, movie, episode, livetv, book, audiobook");
            ui.add_space(6.0);
            ui.label("Bibliothèques à ignorer (séparées par des virgules)");
            ui.text_edit_singleline(&mut self.blacklist_libraries_text);
        });

        self.save_buttons(ui);
    }

    fn ui_discord(&mut self, ui: &mut egui::Ui) {
        let discord = self
            .config
            .discord
            .get_or_insert_with(DiscordBuilder::default);

        section(ui, "Discord", |ui| {
            egui::Grid::new("discord_grid")
                .num_columns(2)
                .spacing([16.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Application ID");
                    let mut app_id = discord.application_id.clone().unwrap_or_default();
                    if ui.text_edit_singleline(&mut app_id).changed() {
                        discord.application_id = Some(app_id);
                    }
                    ui.end_row();
                });

            ui.add_space(6.0);
            let mut show_paused = discord.show_paused.unwrap_or(true);
            if ui
                .checkbox(&mut show_paused, "Afficher l'activité lorsque la lecture est en pause")
                .changed()
            {
                discord.show_paused = Some(show_paused);
            }
        });

        section(ui, "Boutons", |ui| {
            ui.label("Un bouton par ligne, au format : nom|url (mettre dynamic|dynamic pour lien automatique)");
            ui.add(
                egui::TextEdit::multiline(&mut self.buttons_text)
                    .desired_rows(4)
                    .desired_width(f32::INFINITY)
                    .hint_text("dynamic|dynamic"),
            );
        });

        section(ui, "Imgur (optionnel)", |ui| {
            let imgur = self.config.imgur.get_or_insert_with(ImgurBuilder::default);
            ui.label("Client ID Imgur");
            let mut cid = imgur.client_id.clone().unwrap_or_default();
            if ui.text_edit_singleline(&mut cid).changed() {
                imgur.client_id = if cid.trim().is_empty() {
                    None
                } else {
                    Some(cid)
                };
            }
        });

        self.save_buttons(ui);
    }

    fn ui_display(&mut self, ui: &mut egui::Ui) {
        section(ui, "Musique", |ui| {
            display_block(
                ui,
                "music",
                &mut self.music_display_text,
                &mut self.music_separator_text,
                &mut self.music_status_display_type,
            );
        });

        section(ui, "Films", |ui| {
            display_block(
                ui,
                "movies",
                &mut self.movies_display_text,
                &mut self.movies_separator_text,
                &mut self.movies_status_display_type,
            );
        });

        section(ui, "Épisodes", |ui| {
            display_block(
                ui,
                "episodes",
                &mut self.episodes_display_text,
                &mut self.episodes_separator_text,
                &mut self.episodes_status_display_type,
            );

            ui.add_space(6.0);
            let mut show_simple = self.config.jellyfin.show_simple.unwrap_or(false);
            let mut append_prefix = self.config.jellyfin.append_prefix.unwrap_or(false);
            let mut add_divider = self.config.jellyfin.add_divider.unwrap_or(false);

            if ui.checkbox(&mut show_simple, "Nom d'épisode simplifié").changed() {
                self.config.jellyfin.show_simple = Some(show_simple);
            }
            if ui
                .checkbox(&mut append_prefix, "Ajouter un 0 devant les numéros < 10")
                .changed()
            {
                self.config.jellyfin.append_prefix = Some(append_prefix);
            }
            if ui
                .checkbox(&mut add_divider, "Séparateur entre les numéros saison/épisode")
                .changed()
            {
                self.config.jellyfin.add_divider = Some(add_divider);
            }
        });

        self.save_buttons(ui);
    }

    fn ui_images(&mut self, ui: &mut egui::Ui) {
        let images = self.config.images.get_or_insert_with(ImagesBuilder::default);

        section(ui, "Images", |ui| {
            let mut enable = images.enable_images.unwrap_or(false);
            if ui
                .checkbox(&mut enable, "Activer l'affichage des images")
                .changed()
            {
                images.enable_images = Some(enable);
            }

            let mut imgur = images.imgur_images.unwrap_or(false);
            if ui
                .checkbox(&mut imgur, "Téléverser vers Imgur (recommandé pour les URLs distantes)")
                .changed()
            {
                images.imgur_images = Some(imgur);
            }

            let mut litter = images.litterbox_images.unwrap_or(false);
            if ui
                .checkbox(&mut litter, "Téléverser vers Litterbox")
                .changed()
            {
                images.litterbox_images = Some(litter);
            }

            let mut process = images.process_images.unwrap_or(true);
            if ui
                .checkbox(&mut process, "Traiter les images (carré + flou)")
                .changed()
            {
                images.process_images = Some(process);
            }

            let mut bg = images.bg.unwrap_or(true);
            if ui
                .checkbox(&mut bg, "Arrière-plan flouté")
                .changed()
            {
                images.bg = Some(bg);
            }

            ui.add_space(8.0);
            egui::Grid::new("images_grid")
                .num_columns(2)
                .spacing([16.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Taille du canvas (px)");
                    let mut size = images.size.unwrap_or(512) as i32;
                    if ui.add(egui::DragValue::new(&mut size).range(64..=2048)).changed() {
                        images.size = Some(size.max(64) as u32);
                    }
                    ui.end_row();

                    ui.label("Flou de l'arrière-plan");
                    let mut blur = images.bg_blur.unwrap_or(3.0);
                    if ui
                        .add(egui::DragValue::new(&mut blur).range(0.0..=20.0).speed(0.1))
                        .changed()
                    {
                        images.bg_blur = Some(blur);
                    }
                    ui.end_row();

                    ui.label("Rayon des coins (%)");
                    let mut radius = images.corner_radius.unwrap_or(4.0);
                    if ui
                        .add(egui::DragValue::new(&mut radius).range(0.0..=50.0).speed(0.1))
                        .changed()
                    {
                        images.corner_radius = Some(radius);
                    }
                    ui.end_row();
                });
        });

        self.save_buttons(ui);
    }

    fn ui_background(&mut self, ui: &mut egui::Ui) {
        section(ui, "Comportement en arrière-plan", |ui| {
            let mut settings_changed = false;

            if ui
                .checkbox(
                    &mut self.settings.minimize_to_tray,
                    "Réduire dans la barre des notifications quand je clique sur X",
                )
                .changed()
            {
                settings_changed = true;
            }
            ui.small("Au lieu de fermer, la fenêtre se cache et l'app continue de tourner.");

            ui.add_space(6.0);
            if ui
                .checkbox(
                    &mut self.settings.start_minimized,
                    "Démarrer minimisé dans la barre des notifications",
                )
                .changed()
            {
                settings_changed = true;
            }
            ui.small("Au lancement, l'app va directement dans la tray sans afficher la fenêtre.");

            ui.add_space(6.0);
            if ui
                .checkbox(
                    &mut self.settings.start_with_windows,
                    "Démarrer automatiquement avec Windows",
                )
                .changed()
            {
                settings_changed = true;
                #[cfg(windows)]
                {
                    if let Err(e) = crate::autostart::sync(self.settings.start_with_windows) {
                        self.push_log(format!("Échec autostart: {e}"));
                    }
                }
            }
            ui.small("Ajoute une entrée dans le registre Windows (HKCU\\…\\Run). Sans effet sur Linux.");

            ui.add_space(6.0);
            if ui
                .checkbox(
                    &mut self.settings.autostart_rpc,
                    "Démarrer le RPC automatiquement au lancement",
                )
                .changed()
            {
                settings_changed = true;
            }
            ui.small("Se connecte à Discord et Jellyfin dès l'ouverture, sans cliquer sur Démarrer.");

            ui.add_space(10.0);
            if settings_changed {
                match self.settings.save() {
                    Ok(_) => self.settings_save_status = "Préférences enregistrées".into(),
                    Err(e) => self.settings_save_status = format!("Erreur: {e}"),
                }
            }
            if !self.settings_save_status.is_empty() {
                ui.colored_label(egui::Color32::from_rgb(120, 200, 120), &self.settings_save_status);
            }
        });

        section(ui, "Astuce", |ui| {
            ui.label("Quand l'app est dans la tray, double-clique sur l'icône pour la rouvrir.");
            ui.label("Tu peux aussi démarrer/arrêter le RPC depuis le menu de l'icône (clic droit).");
        });
    }

    fn ui_status(&mut self, ui: &mut egui::Ui) {
        section(ui, "État", |ui| {
            ui.label(format!("Statut : {}", self.status));
            if let Some(np) = &self.now_playing {
                ui.label(format!("En cours : {np}"));
            } else {
                ui.label("En cours : —");
            }
        });

        section(ui, "Journal", |ui| {
            egui::ScrollArea::vertical()
                .max_height(360.0)
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for line in &self.log {
                        ui.label(egui::RichText::new(line).monospace().size(12.0));
                    }
                });
            ui.add_space(6.0);
            if ui.button("Effacer le journal").clicked() {
                self.log.clear();
            }
        });
    }

    fn save_buttons(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui
                .add(egui::Button::new("💾  Enregistrer la configuration").min_size(egui::vec2(220.0, 32.0)))
                .clicked()
            {
                self.commit_form_to_config();
                match self.config.save() {
                    Ok(_) => {
                        self.config_save_status = format!(
                            "Configuration enregistrée dans {}",
                            crate::config::config_path().display()
                        );
                        self.push_log("Configuration enregistrée".into());
                    }
                    Err(e) => {
                        self.config_save_status = format!("Erreur d'enregistrement: {e}");
                    }
                }
            }
            ui.add_space(10.0);
            if !self.config_save_status.is_empty() {
                ui.colored_label(egui::Color32::from_rgb(120, 200, 120), &self.config_save_status);
            }
        });
    }
}

fn setup_visuals(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.visuals = egui::Visuals::dark();
    style.visuals.window_rounding = 8.0.into();
    style.visuals.widgets.noninteractive.rounding = 6.0.into();
    style.visuals.widgets.inactive.rounding = 6.0.into();
    style.visuals.widgets.hovered.rounding = 6.0.into();
    style.visuals.widgets.active.rounding = 6.0.into();
    style.visuals.panel_fill = egui::Color32::from_rgb(24, 24, 32);
    style.visuals.window_fill = egui::Color32::from_rgb(28, 28, 38);
    style.visuals.extreme_bg_color = egui::Color32::from_rgb(18, 18, 26);
    ctx.set_style(style);
}

fn section<R>(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.add_space(8.0);
    egui::Frame::group(ui.style())
        .fill(egui::Color32::from_rgb(34, 34, 46))
        .rounding(8.0)
        .inner_margin(egui::Margin::same(14.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(title).strong().size(15.0));
            ui.separator();
            add(ui)
        })
        .inner
}

fn status_pill(ui: &mut egui::Ui, status: &str, running: bool) {
    let (bg, fg) = if running {
        (
            egui::Color32::from_rgb(40, 90, 50),
            egui::Color32::from_rgb(180, 240, 190),
        )
    } else if status == "Erreur" {
        (
            egui::Color32::from_rgb(120, 40, 40),
            egui::Color32::from_rgb(255, 200, 200),
        )
    } else {
        (
            egui::Color32::from_rgb(60, 60, 80),
            egui::Color32::from_rgb(200, 200, 220),
        )
    };
    egui::Frame::none()
        .fill(bg)
        .rounding(12.0)
        .inner_margin(egui::Margin::symmetric(10.0, 4.0))
        .show(ui, |ui| {
            ui.colored_label(fg, status);
        });
}

fn display_block(
    ui: &mut egui::Ui,
    id: &str,
    text: &mut String,
    separator: &mut String,
    status_type: &mut String,
) {
    egui::Grid::new(format!("display_grid_{id}"))
        .num_columns(2)
        .spacing([16.0, 10.0])
        .show(ui, |ui| {
            ui.label("Champs à afficher");
            ui.text_edit_singleline(text);
            ui.end_row();
            ui.label("Séparateur");
            ui.add(egui::TextEdit::singleline(separator).desired_width(80.0));
            ui.end_row();
            ui.label("Type d'affichage");
            egui::ComboBox::from_id_salt(format!("status_{id}"))
                .selected_text(if status_type.is_empty() {
                    "(par défaut)".to_string()
                } else {
                    status_type.clone()
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(status_type, "".to_string(), "(par défaut)");
                    ui.selectable_value(status_type, "name".to_string(), "name");
                    ui.selectable_value(status_type, "state".to_string(), "state");
                    ui.selectable_value(status_type, "details".to_string(), "details");
                });
            ui.end_row();
        });
    ui.small("Champs valides: genres, year, album, artists, track, title, original_title, critic_score, community_score, version. Sépare par des virgules.");
}

fn display_to_text(d: Option<&DisplayField>) -> String {
    match d {
        Some(DisplayField::Vec(v)) => v.join(", "),
        Some(DisplayField::String(s)) => s.clone(),
        None => String::new(),
    }
}

fn parse_display_text(text: &str) -> Option<DisplayField> {
    let cleaned = text.trim();
    if cleaned.is_empty() {
        return None;
    }
    let parts: Vec<String> = cleaned
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(DisplayField::Vec(parts))
    }
}

fn empty_to_none(s: &str) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn str_to_media_type(s: &str) -> Option<MediaType> {
    match s {
        "music" => Some(MediaType::Music),
        "movie" => Some(MediaType::Movie),
        "episode" => Some(MediaType::Episode),
        "livetv" => Some(MediaType::LiveTv),
        "book" => Some(MediaType::Book),
        "audiobook" => Some(MediaType::AudioBook),
        _ => None,
    }
}

fn media_type_to_str(m: &MediaType) -> &'static str {
    match m {
        MediaType::Music => "music",
        MediaType::Movie => "movie",
        MediaType::Episode => "episode",
        MediaType::LiveTv => "livetv",
        MediaType::Book => "book",
        MediaType::AudioBook => "audiobook",
        MediaType::None => "none",
    }
}

fn chrono_like_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}
