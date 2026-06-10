use colored::Colorize;
use crate::config;

pub fn run() {
    let cfg = config::load();

    if cfg.user.is_none() {
        println!("{} Already logged out.", "!".yellow());
        return;
    }

    let mut cfg = config::load();
    cfg.user = None;
    config::save(&cfg);
    println!("{} Logged out successfully.", "✓".green());
}
