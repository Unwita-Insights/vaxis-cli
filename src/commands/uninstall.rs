use colored::Colorize;
use dialoguer::Confirm;
use std::process::Command;

const PACKAGE_NAME: &str = "@unwita-insights/vaxis";

pub fn run(force: bool, json: bool) {
    if json && !force {
        eprintln!(
            "{}",
            serde_json::json!({"error": "require_force", "hint": "pass --force to uninstall without a prompt"})
        );
        std::process::exit(1);
    }

    if !force {
        let confirmed = Confirm::new()
            .with_prompt("This will remove vaxis from your system. Continue?")
            .default(false)
            .interact()
            .unwrap_or(false);

        if !confirmed {
            println!("{} Uninstall cancelled.", "!".yellow());
            return;
        }
    }

    let status = Command::new("npm")
        .args(["uninstall", "-g", PACKAGE_NAME])
        .status();

    match status {
        Ok(s) if s.success() => {
            if json {
                println!("{}", serde_json::json!({"ok": true}));
            } else {
                println!("{} vaxis uninstalled successfully.", "✓".green());
            }
        }
        Ok(_) => {
            if json {
                eprintln!("{}", serde_json::json!({"error": "npm_failed"}));
            } else {
                eprintln!(
                    "{} npm uninstall failed. Run manually: npm uninstall -g {}",
                    "✗".red(),
                    PACKAGE_NAME
                );
            }
            std::process::exit(1);
        }
        Err(_) => {
            if json {
                eprintln!("{}", serde_json::json!({"error": "npm_not_found"}));
            } else {
                eprintln!(
                    "{} npm not found in PATH. Run manually: npm uninstall -g {}",
                    "✗".red(),
                    PACKAGE_NAME
                );
            }
            std::process::exit(1);
        }
    }
}
