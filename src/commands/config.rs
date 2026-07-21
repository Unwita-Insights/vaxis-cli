use colored::Colorize;
use crate::config;
use crate::cli::ConfigAction;

pub fn run(action: ConfigAction) {
    match action {
        ConfigAction::SetUrl { url } => {
            // Store the normalized URL (no trailing slash) so it never builds a
            // `//api/…` path later. Matches what base_url() resolves to.
            let url = config::normalize_base_url(&url);
            let mut cfg = config::load();
            cfg.auth_url = Some(url.clone());
            config::save(&cfg);
            println!("{} Server URL set to {}", "✓".green().bold(), url.cyan());
        }
        ConfigAction::Show => {
            let cfg = config::load();
            match cfg.auth_url.as_deref() {
                // Show the effective (normalized) URL so a stored trailing slash
                // doesn't read back as the value actually used for requests.
                Some(url) => println!("auth_url = {}", config::normalize_base_url(url).cyan()),
                None => println!("auth_url = {} {}", config::DEFAULT_AUTH_URL.cyan(), "(default)".dimmed()),
            }
        }
    }
}
