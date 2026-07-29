use colored::Colorize;
use std::process::Command;

const PACKAGE_NAME: &str = "@unwita-insights/vaxis";
const NPM_REGISTRY_URL: &str =
    "https://registry.npmjs.org/@unwita-insights%2Fvaxis/latest";

pub async fn run(json: bool) {
    let current = env!("CARGO_PKG_VERSION");

    let client = reqwest::Client::new();
    let resp = match client
        .get(NPM_REGISTRY_URL)
        .header("Accept", "application/json")
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            if json {
                eprintln!("{}", serde_json::json!({"error": "network_error", "message": e.to_string()}));
            } else {
                eprintln!("{} Failed to check for updates: {}", "✗".red(), e);
            }
            std::process::exit(1);
        }
    };

    let body: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            if json {
                eprintln!("{}", serde_json::json!({"error": "parse_error", "message": e.to_string()}));
            } else {
                eprintln!("{} Failed to parse version info: {}", "✗".red(), e);
            }
            std::process::exit(1);
        }
    };

    let latest = match body.get("version").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => {
            if json {
                eprintln!("{}", serde_json::json!({"error": "version_not_found"}));
            } else {
                eprintln!("{} Could not determine latest version.", "✗".red());
            }
            std::process::exit(1);
        }
    };

    if current == latest.as_str() {
        if json {
            println!(
                "{}",
                serde_json::json!({"current_version": current, "latest_version": latest, "up_to_date": true})
            );
        } else {
            println!(
                "{} Already on the latest version (v{}).",
                "✓".green(),
                current
            );
        }
        return;
    }

    if !json {
        println!(
            "{} Upgrading from v{} to v{}...",
            "→".cyan(),
            current,
            latest
        );
    }

    let status = Command::new("npm")
        .args(["install", "-g", &format!("{}@latest", PACKAGE_NAME)])
        .status();

    match status {
        Ok(s) if s.success() => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({"current_version": current, "latest_version": latest, "updated": true})
                );
            } else {
                println!("{} Upgraded to v{} successfully.", "✓".green(), latest);
            }
        }
        Ok(_) => {
            if json {
                eprintln!("{}", serde_json::json!({"error": "npm_failed"}));
            } else {
                eprintln!(
                    "{} npm install failed. Run manually: npm install -g {}@latest",
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
                    "{} npm not found in PATH. Run manually: npm install -g {}@latest",
                    "✗".red(),
                    PACKAGE_NAME
                );
            }
            std::process::exit(1);
        }
    }
}
