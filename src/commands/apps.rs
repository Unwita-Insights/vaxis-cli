use colored::Colorize;
use dialoguer::{Select, Confirm, Input, theme::ColorfulTheme};
use crate::cli::AppsAction;
use crate::config;

fn auth_token() -> Option<String> {
    config::load().user.map(|u| u.token)
}

pub async fn run(action: AppsAction, json: bool) {
    let token = match auth_token() {
        Some(t) => t,
        None => {
            if json {
                println!("{}", serde_json::json!({"error": "not_authenticated"}));
            } else {
                eprintln!("{} Not logged in. Run {} first.", "✗".red(), "vaxis login".yellow());
            }
            std::process::exit(1);
        }
    };

    match action {
        AppsAction::List => list(&token, json).await,
        AppsAction::Create { name, description } => create(&token, &name, description.as_deref(), json).await,
        AppsAction::Update { id, name, description } => {
            let resolved = resolve_id(&token, id, "Select application to update:").await;
            update(&token, &resolved, name.as_deref(), description.as_deref(), json).await;
        }
        AppsAction::Delete { id, force } => {
            let resolved = resolve_id(&token, id, "Select application to delete:").await;
            delete(&token, &resolved, force, json).await;
        }
        AppsAction::Share { id } => share(&token, &id, json).await,
    }
}

// Fetches app list and lets user pick if no id was provided on the command line.
async fn resolve_id(token: &str, id: Option<String>, prompt: &str) -> String {
    if let Some(id) = id {
        return id;
    }

    let apps = fetch_apps(token).await;
    if apps.is_empty() {
        println!("{}", "No applications found.".dimmed());
        std::process::exit(0);
    }

    let labels: Vec<String> = apps.iter().map(|a| {
        let name = a["name"].as_str().unwrap_or("Untitled");
        let id   = a["id"].as_str().unwrap_or("");
        let desc = a["description"].as_str().unwrap_or("");
        if desc.is_empty() {
            format!("{} ({})", name, id)
        } else {
            format!("{} — {} ({})", name, desc, id)
        }
    }).collect();

    let idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(&labels)
        .default(0)
        .interact()
        .unwrap_or_else(|_| std::process::exit(0));

    apps[idx]["id"].as_str().unwrap_or("").to_string()
}

async fn fetch_apps(token: &str) -> Vec<serde_json::Value> {
    let client = reqwest::Client::new();
    let resp = match client
        .get(format!("{}/api/applications", crate::config::base_url()))
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

    resp.json().await.unwrap_or_default()
}

async fn list(token: &str, json: bool) {
    let apps = fetch_apps(token).await;

    if json {
        println!("{}", serde_json::to_string_pretty(&apps).unwrap_or_default());
        return;
    }

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

async fn create(token: &str, name: &str, description: Option<&str>, json: bool) {
    let client = reqwest::Client::new();
    let resp = match client
        .post(format!("{}/api/applications", crate::config::base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "name": name, "description": description.unwrap_or("") }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => { eprintln!("{} Could not reach server.", "✗".red()); std::process::exit(1); }
    };

    if resp.status() == 401 {
        eprintln!("{} Session expired. Run {} again.", "✗".red(), "vaxis login".yellow());
        std::process::exit(1);
    }

    let app: serde_json::Value = resp.json().await.unwrap_or_default();

    if json {
        println!("{}", serde_json::to_string_pretty(&app).unwrap_or_default());
        return;
    }

    println!(
        "{} Created {} {}",
        "✓".green().bold(),
        app["name"].as_str().unwrap_or(name).green(),
        app["id"].as_str().unwrap_or("").dimmed()
    );
}

async fn fetch_app(token: &str, id: &str) -> serde_json::Value {
    let client = reqwest::Client::new();
    let resp = match client
        .get(format!("{}/api/applications/{}", crate::config::base_url(), id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => { eprintln!("{} Could not reach server.", "✗".red()); std::process::exit(1); }
    };
    if resp.status() == 404 { eprintln!("{} Application not found.", "✗".red()); std::process::exit(1); }
    if resp.status() == 401 { eprintln!("{} Session expired. Run {} again.", "✗".red(), "vaxis login".yellow()); std::process::exit(1); }
    resp.json().await.unwrap_or_default()
}

async fn update(token: &str, id: &str, name: Option<&str>, description: Option<&str>, json: bool) {
    // If flags were provided, use them directly (scripting mode)
    let (final_name, final_desc) = if name.is_some() || description.is_some() {
        (
            name.map(|s| s.to_string()),
            description.map(|s| s.to_string()),
        )
    } else {
        // Interactive mode — fetch current values and let user edit them
        let current = fetch_app(token, id).await;
        let current_name = current["name"].as_str().unwrap_or("").to_string();
        let current_desc = current["description"].as_str().unwrap_or("").to_string();

        let theme = ColorfulTheme::default();

        let new_name: String = Input::with_theme(&theme)
            .with_prompt("Name")
            .with_initial_text(&current_name)
            .interact_text()
            .unwrap_or(current_name.clone());

        let new_desc: String = Input::with_theme(&theme)
            .with_prompt("Description")
            .with_initial_text(&current_desc)
            .allow_empty(true)
            .interact_text()
            .unwrap_or(current_desc.clone());

        let name_changed = new_name != current_name;
        let desc_changed = new_desc != current_desc;

        if !name_changed && !desc_changed {
            println!("{}", "No changes made.".dimmed());
            return;
        }

        (
            if name_changed { Some(new_name) } else { None },
            if desc_changed { Some(new_desc) } else { None },
        )
    };

    if final_name.is_none() && final_desc.is_none() {
        println!("{}", "No changes made.".dimmed());
        return;
    }

    let client = reqwest::Client::new();
    let mut body = serde_json::Map::new();
    if let Some(ref n) = final_name { body.insert("name".into(), n.as_str().into()); }
    if let Some(ref d) = final_desc { body.insert("description".into(), d.as_str().into()); }

    let resp = match client
        .put(format!("{}/api/applications/{}", crate::config::base_url(), id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => { eprintln!("{} Could not reach server.", "✗".red()); std::process::exit(1); }
    };

    match resp.status().as_u16() {
        401 => { eprintln!("{} Session expired. Run {} again.", "✗".red(), "vaxis login".yellow()); std::process::exit(1); }
        404 => { eprintln!("{} Application not found.", "✗".red()); std::process::exit(1); }
        200 => {
            if json {
                println!("{}", serde_json::json!({"ok": true, "id": id}));
            } else {
                let mut parts = vec![];
                if let Some(ref n) = final_name { parts.push(format!("name → {}", n.green().to_string())); }
                if let Some(ref d) = final_desc { parts.push(format!("description → {}", d.green().to_string())); }
                println!("{} Updated {} — {}", "✓".green().bold(), id.dimmed(), parts.join(", "));
            }
        }
        s => { eprintln!("{} Unexpected status {}.", "✗".red(), s); std::process::exit(1); }
    }
}

async fn share(token: &str, id: &str, json: bool) {
    let client = reqwest::Client::new();
    let resp = match client
        .post(format!("{}/api/applications/{}/share", crate::config::base_url(), id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => { eprintln!("{} Could not reach server.", "✗".red()); std::process::exit(1); }
    };

    match resp.status().as_u16() {
        401 => { eprintln!("{} Session expired.", "✗".red()); std::process::exit(1); }
        404 => { eprintln!("{} Application not found.", "✗".red()); std::process::exit(1); }
        200 => {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            if json {
                println!("{}", serde_json::to_string_pretty(&body).unwrap_or_default());
            } else {
                let url = body["url"].as_str().unwrap_or("(no url returned)");
                println!("{} Shareable link: {}", "✓".green().bold(), url.cyan());
            }
        }
        s => { eprintln!("{} Unexpected status {}.", "✗".red(), s); std::process::exit(1); }
    }
}

async fn delete(token: &str, id: &str, force: bool, json: bool) {
    if !force {
        let confirmed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Delete application {}? This cannot be undone.", id.yellow()))
            .default(false)
            .interact()
            .unwrap_or(false);
        if !confirmed {
            println!("Cancelled.");
            return;
        }
    }

    let client = reqwest::Client::new();
    let resp = match client
        .delete(format!("{}/api/applications/{}", crate::config::base_url(), id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => { eprintln!("{} Could not reach server.", "✗".red()); std::process::exit(1); }
    };

    match resp.status().as_u16() {
        401 => { eprintln!("{} Session expired. Run {} again.", "✗".red(), "vaxis login".yellow()); std::process::exit(1); }
        404 => { eprintln!("{} Application not found.", "✗".red()); std::process::exit(1); }
        200 => {
            if json {
                println!("{}", serde_json::json!({"ok": true, "id": id}));
            } else {
                println!("{} Deleted {}", "✓".green().bold(), id.dimmed());
            }
        }
        s   => { eprintln!("{} Unexpected status {}.", "✗".red(), s); std::process::exit(1); }
    }
}
