use colored::Colorize;
use crate::config;

pub fn run(json: bool) {
    let cfg = config::load();

    match cfg.user {
        Some(user) => {
            if json {
                println!("{}", serde_json::json!({
                    "name":  user.name,
                    "email": user.email
                }));
                return;
            }
            println!("{}", "─────────────────────────".dimmed());
            println!("  {}  {}", "Name:".bold(),  user.name.green());
            println!("  {}  {}", "Email:".bold(), user.email);
            println!("{}", "─────────────────────────".dimmed());
        }
        None => {
            if json {
                println!("{}", serde_json::json!({"error": "not_authenticated"}));
            } else {
                eprintln!(
                    "{} Not logged in. Run {} first.",
                    "✗".red(),
                    "vaxis login".yellow()
                );
            }
            std::process::exit(1);
        }
    }
}
