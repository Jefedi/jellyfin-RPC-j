use jellyfin_rpc::{Button, MediaType};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigBuilder {
    pub jellyfin: JellyfinBuilder,
    #[serde(default)]
    pub discord: Option<DiscordBuilder>,
    #[serde(default)]
    pub imgur: Option<ImgurBuilder>,
    #[serde(default)]
    pub images: Option<ImagesBuilder>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JellyfinBuilder {
    pub url: String,
    pub api_key: String,
    pub username: UsernameField,
    #[serde(default)]
    pub music: Option<DisplayOptionsBuilder>,
    #[serde(default)]
    pub movies: Option<DisplayOptionsBuilder>,
    #[serde(default)]
    pub episodes: Option<DisplayOptionsBuilder>,
    #[serde(default)]
    pub blacklist: Option<Blacklist>,
    #[serde(default)]
    pub self_signed_cert: Option<bool>,
    #[serde(default)]
    pub show_simple: Option<bool>,
    #[serde(default)]
    pub append_prefix: Option<bool>,
    #[serde(default)]
    pub add_divider: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum UsernameField {
    Vec(Vec<String>),
    String(String),
}

impl UsernameField {
    pub fn as_string(&self) -> String {
        match self {
            UsernameField::Vec(v) => v.join(", "),
            UsernameField::String(s) => s.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DisplayOptionsBuilder {
    #[serde(default)]
    pub display: Option<DisplayField>,
    #[serde(default)]
    pub separator: Option<String>,
    #[serde(default)]
    pub status_display_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum DisplayField {
    Vec(Vec<String>),
    String(String),
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Blacklist {
    #[serde(default)]
    pub media_types: Option<Vec<MediaType>>,
    #[serde(default)]
    pub libraries: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DiscordBuilder {
    #[serde(default)]
    pub application_id: Option<String>,
    #[serde(default)]
    pub buttons: Option<Vec<Button>>,
    #[serde(default)]
    pub show_paused: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ImgurBuilder {
    #[serde(default)]
    pub client_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ImagesBuilder {
    #[serde(default)]
    pub enable_images: Option<bool>,
    #[serde(default)]
    pub imgur_images: Option<bool>,
    #[serde(default)]
    pub litterbox_images: Option<bool>,
    #[serde(default)]
    pub process_images: Option<bool>,
    #[serde(default)]
    pub size: Option<u32>,
    #[serde(default)]
    pub bg: Option<bool>,
    #[serde(default)]
    pub bg_blur: Option<f32>,
    #[serde(default)]
    pub corner_radius: Option<f32>,
}

impl Default for JellyfinBuilder {
    fn default() -> Self {
        Self {
            url: String::new(),
            api_key: String::new(),
            username: UsernameField::String(String::new()),
            music: None,
            movies: None,
            episodes: None,
            blacklist: None,
            self_signed_cert: Some(false),
            show_simple: Some(false),
            append_prefix: Some(false),
            add_divider: Some(false),
        }
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self {
            jellyfin: JellyfinBuilder::default(),
            discord: Some(DiscordBuilder {
                application_id: Some("1053747938519679018".into()),
                buttons: None,
                show_paused: Some(true),
            }),
            imgur: None,
            images: Some(ImagesBuilder {
                enable_images: Some(true),
                imgur_images: Some(false),
                litterbox_images: Some(false),
                process_images: Some(true),
                size: Some(512),
                bg: Some(true),
                bg_blur: Some(3.0),
                corner_radius: Some(4.0),
            }),
        }
    }
}

pub fn config_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("jellyfin-rpc");
        }
    }
    #[cfg(not(windows))]
    {
        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join("jellyfin-rpc");
        }
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home).join(".config").join("jellyfin-rpc");
        }
    }
    PathBuf::from(".")
}

pub fn config_path() -> PathBuf {
    config_dir().join("main.json")
}

pub fn urls_path() -> PathBuf {
    config_dir().join("urls.json")
}

impl ConfigBuilder {
    pub fn load_or_default() -> Self {
        let path = config_path();
        match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let s = serde_json::to_string_pretty(self).unwrap();
        std::fs::write(path, s)
    }
}
