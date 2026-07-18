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
            match cfg.auth_url.as_deref() {
                Some(url) => println!("auth_url = {}", url.cyan()),
                None => println!("auth_url = {} {}", config::DEFAULT_AUTH_URL.cyan(), "(default)".dimmed()),
            }
        }
    }
}
