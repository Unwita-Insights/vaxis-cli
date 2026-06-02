use colored::Colorize;
use crate::config;

pub fn run() {
    let cfg = config::load();

    match cfg.user {
        Some(user) => {
            println!("{}", "─────────────────────────".dimmed());
            println!("  {}  {}", "Name:".bold(),  user.name.green());
            println!("  {}  {}", "Email:".bold(), user.email);
            println!("{}", "─────────────────────────".dimmed());
        }
        None => {
            eprintln!(
                "{} Not logged in. Run {} first.",
                "✗".red(),
                "vaxis login".yellow()
            );
            std::process::exit(1);
        }
    }
}
