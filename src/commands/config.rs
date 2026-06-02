use colored::Colorize;
use crate::config;
use crate::cli::ConfigAction;

pub fn run(action: ConfigAction) {
    match action {
        ConfigAction::SetUrl { url } => {
            let mut cfg = config::load();
            cfg.auth_url = Some(url.clone());
            config::save(&cfg);
            println!("{} Server URL set to {}", "✓".green().bold(), url.cyan());
        }
        ConfigAction::Show => {
            let cfg = config::load();
            let url = cfg.auth_url.as_deref().unwrap_or("https://vaxis.dev (default)");
            println!("auth_url = {}", url.cyan());
        }
    }
}
