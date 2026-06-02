use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub auth_url: Option<String>,
    pub user: Option<UserProfile>,
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
