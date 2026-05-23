use crate::config::{ConfigBuilder, DisplayField, UsernameField};
use jellyfin_rpc::{Client, DisplayFormat, EpisodeDisplayOptions, StatusType, VERSION};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

pub enum RpcCommand {
    Start(ConfigBuilder),
    Stop,
    Shutdown,
}

#[derive(Clone, Debug)]
pub enum RpcEvent {
    Status(String),
    Log(String),
    NowPlaying(String),
    Cleared,
    Error(String),
}

pub struct RpcWorker {
    cmd_tx: Sender<RpcCommand>,
    pub evt_rx: Receiver<RpcEvent>,
}

impl RpcWorker {
    pub fn spawn(ctx: egui::Context) -> Self {
        let (cmd_tx, cmd_rx) = channel::<RpcCommand>();
        let (evt_tx, evt_rx) = channel::<RpcEvent>();
        thread::spawn(move || rpc_loop(cmd_rx, evt_tx, ctx));
        Self { cmd_tx, evt_rx }
    }

    pub fn send(&self, cmd: RpcCommand) {
        let _ = self.cmd_tx.send(cmd);
    }
}

fn emit(evt_tx: &Sender<RpcEvent>, ctx: &egui::Context, evt: RpcEvent) {
    let _ = evt_tx.send(evt);
    ctx.request_repaint();
}

fn rpc_loop(cmd_rx: Receiver<RpcCommand>, evt_tx: Sender<RpcEvent>, ctx: egui::Context) {
    let mut active: Option<RunningClient> = None;

    loop {
        let cmd = if active.is_some() {
            match cmd_rx.try_recv() {
                Ok(c) => Some(c),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => return,
            }
        } else {
            match cmd_rx.recv() {
                Ok(c) => Some(c),
                Err(_) => return,
            }
        };

        if let Some(cmd) = cmd {
            match cmd {
                RpcCommand::Shutdown => {
                    if let Some(mut c) = active.take() {
                        let _ = c.client.clear_activity();
                    }
                    return;
                }
                RpcCommand::Stop => {
                    if let Some(mut c) = active.take() {
                        let _ = c.client.clear_activity();
                        emit(&evt_tx, &ctx, RpcEvent::Status("Arrêté".into()));
                        emit(&evt_tx, &ctx, RpcEvent::Cleared);
                    }
                }
                RpcCommand::Start(cfg) => {
                    if let Some(mut c) = active.take() {
                        let _ = c.client.clear_activity();
                    }
                    emit(&evt_tx, &ctx, RpcEvent::Status("Démarrage…".into()));
                    match build_client(&cfg) {
                        Ok(mut client) => {
                            emit(
                                &evt_tx,
                                &ctx,
                                RpcEvent::Log("Connexion à Discord…".into()),
                            );
                            match client.connect() {
                                Ok(_) => {
                                    emit(
                                        &evt_tx,
                                        &ctx,
                                        RpcEvent::Status("Connecté".into()),
                                    );
                                    emit(
                                        &evt_tx,
                                        &ctx,
                                        RpcEvent::Log("Connecté à Discord".into()),
                                    );
                                    active = Some(RunningClient {
                                        client,
                                        last_activity: String::new(),
                                        next_tick: Instant::now(),
                                    });
                                }
                                Err(err) => {
                                    emit(
                                        &evt_tx,
                                        &ctx,
                                        RpcEvent::Error(format!(
                                            "Échec de connexion Discord: {err}"
                                        )),
                                    );
                                    emit(
                                        &evt_tx,
                                        &ctx,
                                        RpcEvent::Status("Erreur".into()),
                                    );
                                }
                            }
                        }
                        Err(err) => {
                            emit(
                                &evt_tx,
                                &ctx,
                                RpcEvent::Error(format!("Configuration invalide: {err}")),
                            );
                            emit(&evt_tx, &ctx, RpcEvent::Status("Erreur".into()));
                        }
                    }
                }
            }
        }

        if let Some(c) = active.as_mut() {
            if Instant::now() >= c.next_tick {
                c.next_tick = Instant::now() + Duration::from_secs(7);
                match c.client.set_activity() {
                    Ok(activity) => {
                        if activity.is_empty() && !c.last_activity.is_empty() {
                            let _ = c.client.clear_activity();
                            emit(&evt_tx, &ctx, RpcEvent::Cleared);
                            c.last_activity.clear();
                        } else if activity != c.last_activity {
                            c.last_activity = activity.clone();
                            emit(&evt_tx, &ctx, RpcEvent::NowPlaying(activity));
                        }
                    }
                    Err(err) => {
                        let msg = err.to_string();
                        if msg == "content is blacklisted" {
                            // skip
                        } else {
                            emit(
                                &evt_tx,
                                &ctx,
                                RpcEvent::Log(format!("Erreur, reconnexion: {msg}")),
                            );
                            if let Err(e) = c.client.reconnect() {
                                emit(
                                    &evt_tx,
                                    &ctx,
                                    RpcEvent::Error(format!("Reconnexion échouée: {e}")),
                                );
                            } else {
                                emit(&evt_tx, &ctx, RpcEvent::Log("Reconnecté".into()));
                            }
                        }
                    }
                }
            }
            thread::sleep(Duration::from_millis(250));
        }
    }
}

struct RunningClient {
    client: Client,
    last_activity: String,
    next_tick: Instant,
}

fn build_client(cfg: &ConfigBuilder) -> Result<Client, Box<dyn std::error::Error>> {
    if cfg.jellyfin.url.trim().is_empty() {
        return Err("URL Jellyfin manquante".into());
    }
    if cfg.jellyfin.api_key.trim().is_empty() {
        return Err("Clé API Jellyfin manquante".into());
    }

    let url = if cfg.jellyfin.url.ends_with('/') {
        cfg.jellyfin.url.clone()
    } else {
        format!("{}/", cfg.jellyfin.url)
    };

    let usernames: Vec<String> = match &cfg.jellyfin.username {
        UsernameField::Vec(v) => v.clone(),
        UsernameField::String(s) => s
            .split(',')
            .map(|u| u.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
    };

    if usernames.is_empty() {
        return Err("Nom d'utilisateur Jellyfin manquant".into());
    }

    let mut builder = Client::builder();

    let show_simple = cfg.jellyfin.show_simple.unwrap_or(false);
    let add_divider = cfg.jellyfin.add_divider.unwrap_or(false);
    let append_prefix = cfg.jellyfin.append_prefix.unwrap_or(false);
    let self_signed = cfg.jellyfin.self_signed_cert.unwrap_or(false);

    let urls_file = crate::config::urls_path().to_string_lossy().to_string();

    let images = cfg.images.clone().unwrap_or_default();
    let discord = cfg.discord.clone().unwrap_or_default();

    builder
        .api_key(cfg.jellyfin.api_key.clone())
        .url(url)
        .usernames(usernames)
        .self_signed(self_signed)
        .episode_simple(show_simple)
        .episode_divider(add_divider)
        .episode_prefix(append_prefix)
        .show_paused(discord.show_paused.unwrap_or(true))
        .show_images(images.enable_images.unwrap_or(false))
        .use_imgur(images.imgur_images.unwrap_or(false))
        .use_litterbox(images.litterbox_images.unwrap_or(false))
        .process_images(images.process_images.unwrap_or(true))
        .image_size(images.size)
        .image_background(images.bg.unwrap_or(true))
        .image_background_blur(images.bg_blur.unwrap_or(3.0))
        .image_corner_radius(images.corner_radius.or(Some(4.0)))
        .large_image_text(format!("Jellyfin-RPC v{}", VERSION.unwrap_or("UNKNOWN")))
        .imgur_urls_file_location(urls_file.clone())
        .litterbox_urls_file_location(urls_file);

    apply_display(
        &mut builder,
        cfg.jellyfin.music.as_ref(),
        Kind::Music,
    );
    apply_display(
        &mut builder,
        cfg.jellyfin.movies.as_ref(),
        Kind::Movies,
    );

    if let Some(eps) = cfg.jellyfin.episodes.as_ref() {
        apply_display(&mut builder, Some(eps), Kind::Episodes);
    } else {
        builder.episodes_display(DisplayFormat::from(EpisodeDisplayOptions {
            divider: add_divider,
            prefix: append_prefix,
            simple: show_simple,
        }));
    }

    if let Some(blacklist) = &cfg.jellyfin.blacklist {
        if let Some(mt) = &blacklist.media_types {
            builder.blacklist_media_types(mt.clone());
        }
        if let Some(libs) = &blacklist.libraries {
            builder.blacklist_libraries(libs.clone());
        }
    }

    if let Some(app_id) = discord.application_id.clone() {
        if !app_id.trim().is_empty() {
            builder.client_id(app_id);
        }
    }

    if let Some(buttons) = discord.buttons.clone() {
        builder.buttons(buttons);
    }

    if let Some(imgur) = &cfg.imgur {
        if let Some(client_id) = imgur.client_id.clone() {
            if !client_id.trim().is_empty() {
                builder.imgur_client_id(client_id);
            }
        }
    }

    Ok(builder.build()?)
}

enum Kind {
    Music,
    Movies,
    Episodes,
}

fn apply_display(
    builder: &mut jellyfin_rpc::ClientBuilder,
    opts: Option<&crate::config::DisplayOptionsBuilder>,
    kind: Kind,
) {
    let Some(opts) = opts else { return };

    if let Some(disp) = &opts.display {
        let df = match disp {
            DisplayField::Vec(v) => DisplayFormat::from(v.clone()),
            DisplayField::String(s) => DisplayFormat::from(s.clone()),
        };
        match kind {
            Kind::Music => {
                builder.music_display(df);
            }
            Kind::Movies => {
                builder.movies_display(df);
            }
            Kind::Episodes => {
                builder.episodes_display(df);
            }
        }
    }

    if let Some(sep) = &opts.separator {
        match kind {
            Kind::Music => {
                builder.music_separator(sep.clone());
            }
            Kind::Movies => {
                builder.movies_separator(sep.clone());
            }
            Kind::Episodes => {
                builder.episodes_separator(sep.clone());
            }
        }
    }

    if let Some(sdt) = &opts.status_display_type {
        if let Ok(st) = StatusType::try_from(sdt.clone()) {
            match kind {
                Kind::Music => {
                    builder.music_status_display_type(st);
                }
                Kind::Movies => {
                    builder.movies_status_display_type(st);
                }
                Kind::Episodes => {
                    builder.episodes_status_display_type(st);
                }
            }
        }
    }
}
