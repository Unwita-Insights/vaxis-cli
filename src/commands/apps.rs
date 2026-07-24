use colored::Colorize;
use dialoguer::{Select, Confirm, Input, theme::ColorfulTheme};
use crate::cli::AppsAction;
use crate::config;

fn auth_token() -> Option<String> {
    config::load().user.map(|u| u.token)
}

pub async fn run(action: AppsAction, json: bool) {
    validate_json_arguments(&action, json);

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
        AppsAction::Share { id, revoke } => share(&token, &id, revoke, json).await,
    }
}

fn validate_json_arguments(action: &AppsAction, json: bool) {
    if !json {
        return;
    }

    match action {
        AppsAction::Update { id, name, description } => {
            if id.is_none() {
                fail_json("application ID is required with `--json`");
            }
            if name.is_none() && description.is_none() {
                fail_json("`--name` or `--description` is required with `--json`");
            }
        }
        AppsAction::Delete { id, force } => {
            if id.is_none() {
                fail_json("application ID is required with `--json`");
            }
            if !force {
                fail_json("`--force` is required with `--json`");
            }
        }
        _ => {}
    }
}

fn fail_json(message: &str) -> ! {
    println!(
        "{}",
        serde_json::json!({
            "error": "invalid_arguments",
            "message": message,
        })
    );
    std::process::exit(1);
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

// App-wide sharing is retired server-side: creating/rotating an app token is a
// hard 410, because one app link exposes every diagram in the app. Sharing is
// per-diagram now (`vaxis diagrams share`). Reads and revocation stay open so a
// legacy link minted before the cutover can still be found and turned off —
// that, and nothing else, is what this command does.
async fn share(token: &str, id: &str, revoke: bool, json: bool) {
    let client = reqwest::Client::new();
    let url = format!("{}/api/applications/{}/share", crate::config::base_url(), id);

    if revoke {
        let resp = match client
            .delete(&url)
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
                    println!("{}", serde_json::json!({"ok": true, "id": id, "shared": false}));
                } else {
                    println!("{} Legacy app-wide link disabled for {}", "✓".green().bold(), id.dimmed());
                }
            }
            s => { eprintln!("{} Unexpected status {}.", "✗".red(), s); std::process::exit(1); }
        }
        return;
    }

    let resp = match client
        .get(&url)
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
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            let legacy = body["token"].as_str().filter(|t| !t.is_empty() && *t != "null");

            if json {
                let mut out = serde_json::json!({
                    "id": id,
                    "app_share_retired": true,
                    "use_instead": "vaxis diagrams share <diagramId>",
                    "legacy_shared": legacy.is_some(),
                });
                if let Some(tok) = legacy {
                    let base = crate::config::base_url();
                    out["legacy_url"]   = serde_json::Value::String(format!("{}/view/{}", base, tok));
                    out["legacy_token"] = serde_json::Value::String(tok.to_string());
                    if let Some(etok) = body["edit_token"].as_str().filter(|t| !t.is_empty() && *t != "null") {
                        out["legacy_edit_url"]   = serde_json::Value::String(format!("{}/collab/{}", base, etok));
                        out["legacy_edit_token"] = serde_json::Value::String(etok.to_string());
                    }
                }
                println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
                return;
            }

            match legacy {
                Some(tok) => {
                    let base = crate::config::base_url();
                    println!("{} This app still has a legacy app-wide link — it exposes {} in the app.",
                        "⚠".yellow(), "every diagram".yellow());
                    println!("  View link: {}", format!("{}/view/{}", base, tok).cyan());
                    if let Some(etok) = body["edit_token"].as_str().filter(|t| !t.is_empty() && *t != "null") {
                        println!("  Edit link: {}", format!("{}/collab/{}", base, etok).cyan());
                    }
                    println!("\n  Turn it off with: {}", format!("vaxis apps share {} --revoke", id).yellow());
                }
                None => println!("{}", "No legacy app-wide link on this application.".dimmed()),
            }
            println!("  App-wide sharing is retired. Share one diagram with: {}",
                "vaxis diagrams share <diagramId>".yellow());
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
