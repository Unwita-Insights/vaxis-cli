use colored::Colorize;
use crate::cli::AppsAction;
use crate::config;

const DEFAULT_AUTH_URL: &str = "https://vaxis.dev";

fn base_url() -> String {
    std::env::var("VAXIS_AUTH_URL")
        .ok()
        .or_else(|| config::load().auth_url)
        .unwrap_or_else(|| DEFAULT_AUTH_URL.to_string())
}

fn auth_token() -> Option<String> {
    config::load().user.map(|u| u.token)
}

pub async fn run(action: AppsAction) {
    let token = match auth_token() {
        Some(t) => t,
        None => {
            eprintln!("{} Not logged in. Run {} first.", "✗".red(), "vaxis login".yellow());
            std::process::exit(1);
        }
    };

    match action {
        AppsAction::List => list(&token).await,
        AppsAction::Create { name, description } => create(&token, &name, description.as_deref()).await,
    }
}

async fn list(token: &str) {
    let client = reqwest::Client::new();
    let url = format!("{}/api/applications", base_url());

    let resp = match client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => {
            eprintln!("{} Could not reach server.", "✗".red());
            std::process::exit(1);
        }
    };

    if resp.status() == 401 {
        eprintln!("{} Session expired. Run {} again.", "✗".red(), "vaxis login".yellow());
        std::process::exit(1);
    }

    let apps: Vec<serde_json::Value> = match resp.json().await {
        Ok(v) => v,
        Err(_) => {
            eprintln!("{} Unexpected response from server.", "✗".red());
            std::process::exit(1);
        }
    };

    if apps.is_empty() {
        println!("{}", "No applications yet. Create one with: vaxis apps create <name>".dimmed());
        return;
    }

    println!("{}", "─".repeat(52).dimmed());
    for app in &apps {
        let name = app["name"].as_str().unwrap_or("Untitled");
        let id   = app["id"].as_str().unwrap_or("");
        let desc = app["description"].as_str().unwrap_or("");
        println!("  {}  {}", name.bold(), id.dimmed());
        if !desc.is_empty() {
            println!("  {}", desc.dimmed());
        }
    }
    println!("{}", "─".repeat(52).dimmed());
    println!("  {} application{}", apps.len().to_string().cyan(), if apps.len() == 1 { "" } else { "s" });
}

async fn create(token: &str, name: &str, description: Option<&str>) {
    let client = reqwest::Client::new();
    let url = format!("{}/api/applications", base_url());

    let body = serde_json::json!({
        "name": name,
        "description": description.unwrap_or(""),
    });

    let resp = match client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => {
            eprintln!("{} Could not reach server.", "✗".red());
            std::process::exit(1);
        }
    };

    if resp.status() == 401 {
        eprintln!("{} Session expired. Run {} again.", "✗".red(), "vaxis login".yellow());
        std::process::exit(1);
    }

    let app: serde_json::Value = match resp.json().await {
        Ok(v) => v,
        Err(_) => {
            eprintln!("{} Unexpected response from server.", "✗".red());
            std::process::exit(1);
        }
    };

    println!(
        "{} Created {} {}",
        "✓".green().bold(),
        app["name"].as_str().unwrap_or(name).green(),
        app["id"].as_str().unwrap_or("").dimmed()
    );
}
