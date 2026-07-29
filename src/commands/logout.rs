use colored::Colorize;
use crate::config;

pub fn run() {
    let mut cfg = config::load();

    if cfg.user.is_none() {
        println!("{} Already logged out.", "!".yellow());
        return;
    }

    cfg.user = None;
    config::save(&cfg);
    println!("{} Logged out successfully.", "✓".green());
}
