use colored::Colorize;
use crate::config;
use crate::cli::ConfigAction;

pub fn run(action: ConfigAction, json: bool) {
    match action {
        ConfigAction::SetUrl { url } => {
            // Store the normalized URL (no trailing slash) so it never builds a
            // `//api/…` path later. Matches what base_url() resolves to.
            let url = config::normalize_base_url(&url);
            let mut cfg = config::load();
            cfg.auth_url = Some(url.clone());
            config::save(&cfg);
            if json {
                println!("{}", serde_json::json!({"ok": true, "auth_url": url}));
            } else {
                println!("{} Server URL set to {}", "✓".green().bold(), url.cyan());
            }
        }
        ConfigAction::SetMode { mode } => {
            let mut cfg = config::load();
            match mode.as_str() {
                Some(mode_str) => {
                    cfg.generation_mode = Some(mode_str.to_string());
                    config::save(&cfg);
                    let human = if mode_str == "prompt" { "Vaxis server AI" } else { "your own AI (Claude / Codex)" };
                    if json {
                        println!("{}", serde_json::json!({"ok": true, "generation_mode": mode_str}));
                    } else {
                        println!("{} Generation mode set to {} ({})", "✓".green().bold(), mode_str.cyan(), human);
                    }
                }
                None => {
                    cfg.generation_mode = None;
                    config::save(&cfg);
                    if json {
                        println!("{}", serde_json::json!({"ok": true, "generation_mode": null}));
                    } else {
                        println!("{} Generation mode cleared — you'll be asked on next generate.", "✓".green().bold());
                    }
                }
            }
        }
        ConfigAction::Show => {
            let cfg = config::load();
            // Resolve the URL the same way base_url() does, so what's shown is what
            // requests actually use (env var wins, then stored value, then default).
            let effective_url = std::env::var("VAXIS_AUTH_URL")
                .ok()
                .or_else(|| cfg.auth_url.clone())
                .map(|u| config::normalize_base_url(&u))
                .unwrap_or_else(|| config::DEFAULT_AUTH_URL.to_string());

            if json {
                // Machine-readable form for assistants — SKILL.md reads
                // `generation_mode` here to pick --mermaid vs --prompt (Rule 6:
                // decisions read JSON, never colored text). null when unset.
                println!(
                    "{}",
                    serde_json::json!({
                        "auth_url": effective_url,
                        "generation_mode": cfg.generation_mode,
                    })
                );
                return;
            }

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
