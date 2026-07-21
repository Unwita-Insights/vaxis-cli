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
        ConfigAction::SetMode { mode } => {
            let mut cfg = config::load();
            cfg.generation_mode = Some(mode.clone());
            config::save(&cfg);
            let human = if mode == "prompt" { "Vaxis server AI" } else { "your own AI (Claude / Codex)" };
            println!("{} Generation mode set to {} ({})", "✓".green().bold(), mode.cyan(), human);
        }
        ConfigAction::Show => {
            let cfg = config::load();
            match cfg.auth_url.as_deref() {
                // Show the effective (normalized) URL so a stored trailing slash
                // doesn't read back as the value actually used for requests.
                Some(url) => println!("auth_url = {}", config::normalize_base_url(url).cyan()),
                None => println!("auth_url = {} {}", config::DEFAULT_AUTH_URL.cyan(), "(default)".dimmed()),
            }
            match cfg.generation_mode.as_deref() {
                Some(mode) => println!("generation_mode = {}", mode.cyan()),
                None => println!("generation_mode = {}", "(not set — asked on first `diagrams generate`)".dimmed()),
            }
        }
    }
}
