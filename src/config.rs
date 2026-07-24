use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub auth_url: Option<String>,
    pub user: Option<UserProfile>,
    /// How diagrams are generated: `"mermaid"` (the driving AI — Claude/Codex —
    /// writes the Mermaid itself) or `"prompt"` (Vaxis's server AI generates it).
    /// Set once on the first interactive `diagrams generate`; the assistant reads
    /// it via `config show` and honors it (see skill-data/core/SKILL.md).
    pub generation_mode: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserProfile {
    pub name: String,
    pub email: String,
    pub token: String,
}

fn config_path() -> PathBuf {
    let mut path = dirs::config_dir().expect("cannot find config dir");
    path.push("vaxis");
    path.push("config.toml");
    path
}

pub fn load() -> Config {
    let path = config_path();
    if !path.exists() {
        return Config::default();
    }
    let content = fs::read_to_string(&path).unwrap_or_default();
    toml::from_str(&content).unwrap_or_default()
}

pub fn save(config: &Config) {
    let path = config_path();
    fs::create_dir_all(path.parent().unwrap()).expect("cannot create config dir");
    let content = toml::to_string(config).expect("cannot serialize config");
    fs::write(&path, content).expect("cannot write config file");
}

pub fn clear() {
    let path = config_path();
    if path.exists() {
        fs::remove_file(path).expect("cannot remove config file");
    }
}

/// The stored generation-mode preference, if the user has set one.
pub fn generation_mode() -> Option<String> {
    load().generation_mode
}

/// Persist the generation-mode preference (`"mermaid"` or `"prompt"`).
pub fn set_generation_mode(mode: &str) {
    let mut cfg = load();
    cfg.generation_mode = Some(mode.to_string());
    save(&cfg);
}

pub const DEFAULT_AUTH_URL: &str = "https://app.vaxis.dev";

pub fn base_url() -> String {
    let url = std::env::var("VAXIS_AUTH_URL")
        .ok()
        .or_else(|| load().auth_url)
        .unwrap_or_else(|| DEFAULT_AUTH_URL.to_string());
    // Strip trailing slashes so callers can safely `format!("{}/api/…", base_url())`.
    // A configured `https://app.vaxis.dev/` would otherwise build `…//api/…`, which
    // the backend router 404s ("Server returned an unexpected response" on login).
    normalize_base_url(&url)
}

/// Trim trailing slashes (and surrounding whitespace) from a base URL.
pub fn normalize_base_url(url: &str) -> String {
    url.trim().trim_end_matches('/').to_string()
}
