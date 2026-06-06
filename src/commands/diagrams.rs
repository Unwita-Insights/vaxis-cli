use colored::Colorize;
use dialoguer::{Select, Confirm, theme::ColorfulTheme};
use crate::cli::DiagramsAction;
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

pub async fn run(action: DiagramsAction, json: bool) {
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
        DiagramsAction::List { app_id }           => list(&token, &app_id, json).await,
        DiagramsAction::Create { app_id, name }   => create(&token, &app_id, &name, json).await,
        DiagramsAction::Generate { id, prompt, mermaid } => {
            if prompt.is_none() && mermaid.is_none() {
                if json {
                    println!("{}", serde_json::json!({"error": "provide --prompt or --mermaid"}));
                } else {
                    eprintln!("{} Provide either --prompt or --mermaid", "✗".red());
                }
                std::process::exit(1);
            }
            generate(&token, &id, prompt.as_deref(), mermaid.as_deref(), json).await
        }
        DiagramsAction::Show { id }               => show(&token, &id, json).await,
        DiagramsAction::Tree { id }               => tree_cmd(&token, &id, json).await,
        DiagramsAction::Undo { id }               => undo(&token, &id, json).await,
        DiagramsAction::Rename { id, name }       => rename(&token, &id, &name, json).await,
        DiagramsAction::Delete { id, app_id, force } => {
            let resolved = resolve_diagram_id(&token, id, app_id, "Select diagram to delete:").await;
            delete(&token, &resolved, force, json).await;
        }
        DiagramsAction::Format                 => format_cmd(json),
        DiagramsAction::Patch { id, diff }     => patch(&token, &id, &diff, json).await,
        DiagramsAction::Import { id, mermaid } => import_cmd(&token, &id, &mermaid, json).await,
    }
}

async fn resolve_diagram_id(
    token: &str,
    id: Option<String>,
    app_id: Option<String>,
    prompt: &str,
) -> String {
    if let Some(id) = id {
        return id;
    }

    let app_id = match app_id {
        Some(a) => a,
        None => {
            let apps = fetch_apps(token).await;
            if apps.is_empty() {
                println!("{}", "No applications found.".dimmed());
                std::process::exit(0);
            }
            let labels: Vec<String> = apps.iter().map(|a| {
                format!("{} ({})", a["name"].as_str().unwrap_or("Untitled"), a["id"].as_str().unwrap_or(""))
            }).collect();
            let idx = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Select application:")
                .items(&labels)
                .default(0)
                .interact()
                .unwrap_or_else(|_| std::process::exit(0));
            apps[idx]["id"].as_str().unwrap_or("").to_string()
        }
    };

    let diagrams = fetch_diagrams(token, &app_id).await;
    if diagrams.is_empty() {
        println!("{}", "No diagrams found in this application.".dimmed());
        std::process::exit(0);
    }

    let labels: Vec<String> = diagrams.iter().map(|d| {
        format!("{} ({})", d["name"].as_str().unwrap_or("Untitled"), d["id"].as_str().unwrap_or(""))
    }).collect();

    let idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .items(&labels)
        .default(0)
        .interact()
        .unwrap_or_else(|_| std::process::exit(0));

    diagrams[idx]["id"].as_str().unwrap_or("").to_string()
}

async fn fetch_apps(token: &str) -> Vec<serde_json::Value> {
    let client = reqwest::Client::new();
    let resp = match client
        .get(format!("{}/api/applications", base_url()))
        .header("Authorization", format!("Bearer {}", token))
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
    resp.json().await.unwrap_or_default()
}

async fn fetch_diagrams(token: &str, app_id: &str) -> Vec<serde_json::Value> {
    let client = reqwest::Client::new();
    let resp = match client
        .get(format!("{}/api/diagrams?applicationId={}", base_url(), app_id))
        .header("Authorization", format!("Bearer {}", token))
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
    resp.json().await.unwrap_or_default()
}

async fn list(token: &str, app_id: &str, json: bool) {
    let diagrams = fetch_diagrams(token, app_id).await;

    if json {
        println!("{}", serde_json::to_string_pretty(&diagrams).unwrap_or_default());
        return;
    }

    if diagrams.is_empty() {
        println!("{}", "No diagrams yet. Create one with: vaxis diagrams create <appId> <name>".dimmed());
        return;
    }

    println!("{}", "─".repeat(56).dimmed());
    for d in &diagrams {
        let name   = d["name"].as_str().unwrap_or("Untitled");
        let id     = d["id"].as_str().unwrap_or("");
        let parent = d["parent_diagram_id"].as_str().unwrap_or("");
        if parent.is_empty() {
            println!("  {}  {} {}", name.bold(), id.dimmed(), "[root]".cyan());
        } else {
            println!("  {}  {}", name.bold(), id.dimmed());
        }
    }
    println!("{}", "─".repeat(56).dimmed());
    println!("  {} diagram{}", diagrams.len().to_string().cyan(), if diagrams.len() == 1 { "" } else { "s" });
}

async fn create(token: &str, app_id: &str, name: &str, json: bool) {
    let client = reqwest::Client::new();
    let resp = match client
        .post(format!("{}/api/diagrams", base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "applicationId": app_id, "name": name }))
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

    let diagram: serde_json::Value = resp.json().await.unwrap_or_default();

    if json {
        println!("{}", serde_json::to_string_pretty(&diagram).unwrap_or_default());
        return;
    }

    println!(
        "{} Created diagram {} {}",
        "✓".green().bold(),
        diagram["name"].as_str().unwrap_or(name).green(),
        diagram["id"].as_str().unwrap_or("").dimmed()
    );
}

async fn show(token: &str, id: &str, json: bool) {
    let client = reqwest::Client::new();

    let resp = match client
        .get(format!("{}/api/diagrams/{}", base_url(), id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => { eprintln!("{} Could not reach server.", "✗".red()); std::process::exit(1); }
    };

    match resp.status().as_u16() {
        401 => { eprintln!("{} Session expired.", "✗".red()); std::process::exit(1); }
        404 => { eprintln!("{} Diagram not found.", "✗".red()); std::process::exit(1); }
        _ => {}
    }

    let mut diagram: serde_json::Value = resp.json().await.unwrap_or_default();

    // Fetch chat history to surface the last Mermaid — this is what Claude needs
    let current_mermaid = if let Ok(cr) = client
        .get(format!("{}/api/diagrams/{}/chat", base_url(), id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
    {
        if let Ok(chat) = cr.json::<serde_json::Value>().await {
            chat["messages"]
                .as_array()
                .and_then(|msgs| {
                    msgs.iter().rev().find(|m| m["role"].as_str() == Some("assistant"))
                })
                .and_then(|m| m["content"].as_str())
                .map(|s| s.to_string())
        } else {
            None
        }
    } else {
        None
    };

    if json {
        if let Some(ref mermaid) = current_mermaid {
            diagram["current_mermaid"] = serde_json::Value::String(mermaid.clone());
        }
        // Remove scene_json — it's Excalidraw binary noise, not useful to Claude
        if let Some(obj) = diagram.as_object_mut() {
            obj.remove("scene_json");
        }
        println!("{}", serde_json::to_string_pretty(&diagram).unwrap_or_default());
        return;
    }

    let name   = diagram["name"].as_str().unwrap_or("Untitled");
    let diag_id = diagram["id"].as_str().unwrap_or(id);
    let parent = diagram["parent_diagram_id"].as_str().unwrap_or("");

    println!("{}", "─".repeat(56).dimmed());
    println!("  {}  {}", name.bold(), diag_id.dimmed());
    if parent.is_empty() {
        println!("  {}", "[root diagram]".cyan());
    } else {
        println!("  Parent: {}", parent.dimmed());
    }

    if let Some(child_nodes) = diagram["child_nodes"].as_object() {
        if !child_nodes.is_empty() {
            println!("\n  Child diagrams:");
            for (node_id, child_id) in child_nodes {
                println!("    {} → {}", node_id.yellow(), child_id.as_str().unwrap_or("").dimmed());
            }
        }
    }

    if let Some(mermaid) = current_mermaid {
        println!("\n  {}:", "Current Mermaid".bold());
        for line in mermaid.lines() {
            println!("    {}", line);
        }
    } else {
        println!("\n  {}", "No content yet — run vaxis diagrams generate to create".dimmed());
    }
    println!("{}", "─".repeat(56).dimmed());
}

async fn tree_cmd(token: &str, id: &str, json: bool) {
    let client = reqwest::Client::new();
    let resp = match client
        .get(format!("{}/api/diagrams/{}/tree", base_url(), id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => { eprintln!("{} Could not reach server.", "✗".red()); std::process::exit(1); }
    };

    match resp.status().as_u16() {
        401 => { eprintln!("{} Session expired.", "✗".red()); std::process::exit(1); }
        404 => { eprintln!("{} Diagram not found.", "✗".red()); std::process::exit(1); }
        _ => {}
    }

    let data: serde_json::Value = resp.json().await.unwrap_or_default();

    if json {
        println!("{}", serde_json::to_string_pretty(&data).unwrap_or_default());
        return;
    }

    println!("{}", "─".repeat(56).dimmed());
    print_tree(&data["tree"], "", true);
    println!("{}", "─".repeat(56).dimmed());
}

fn print_tree(node: &serde_json::Value, prefix: &str, is_last: bool) {
    let name    = node["name"].as_str().unwrap_or("Untitled");
    let id      = node["id"].as_str().unwrap_or("");
    let node_id = node["node_id"].as_str().unwrap_or("");

    let connector = if prefix.is_empty() { "" } else if is_last { "└── " } else { "├── " };
    let label = if node_id.is_empty() {
        format!("{}  {}", name.bold(), id.dimmed())
    } else {
        format!("{}  [{}]  {}", name.bold(), node_id.yellow(), id.dimmed())
    };
    println!("{}{}{}", prefix, connector, label);

    if let Some(children) = node["children"].as_array() {
        let child_prefix = if prefix.is_empty() {
            "".to_string()
        } else if is_last {
            format!("{}    ", prefix)
        } else {
            format!("{}│   ", prefix)
        };
        for (i, child) in children.iter().enumerate() {
            print_tree(child, &child_prefix, i == children.len() - 1);
        }
    }
}

async fn generate(token: &str, id: &str, prompt: Option<&str>, mermaid: Option<&str>, json: bool) {
    let body = if let Some(m) = mermaid {
        if !json { println!("{}", "Saving diagram...".dimmed()); }
        serde_json::json!({ "mermaid": m })
    } else {
        if !json { println!("{}", "Generating...".dimmed()); }
        serde_json::json!({ "prompt": prompt.unwrap_or("") })
    };

    let client = reqwest::Client::new();
    let resp = match client
        .post(format!("{}/api/diagrams/{}/generate", base_url(), id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => { eprintln!("{} Could not reach server.", "✗".red()); std::process::exit(1); }
    };

    match resp.status().as_u16() {
        401 => { eprintln!("{} Session expired.", "✗".red()); std::process::exit(1); }
        404 => { eprintln!("{} Diagram not found.", "✗".red()); std::process::exit(1); }
        _ => {}
    }

    let result: serde_json::Value = resp.json().await.unwrap_or_default();
    let mermaid = result["mermaid"].as_str().unwrap_or("").to_string();
    let drills  = result["drills"].as_array().cloned().unwrap_or_default();

    // Create child diagrams for every drill block the AI returned
    let mut created_drills: Vec<serde_json::Value> = Vec::new();
    for drill in &drills {
        let node_id = drill["nodeId"].as_str().unwrap_or("");
        if node_id.is_empty() { continue; }

        if let Ok(cr) = client
            .post(format!("{}/api/diagrams/{}/children", base_url(), id))
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({ "nodeId": node_id, "nodeLabel": node_id }))
            .send()
            .await
        {
            let status = cr.status().as_u16();
            if let Ok(child) = cr.json::<serde_json::Value>().await {
                if status == 200 || status == 201 {
                    created_drills.push(serde_json::json!({
                        "node_id":    node_id,
                        "diagram_id": child["id"].as_str().unwrap_or(""),
                        "name":       child["name"].as_str().unwrap_or(node_id),
                        "already_exists": child["already_exists"].as_bool().unwrap_or(false)
                    }));
                } else {
                    eprintln!("  {} Failed to create child for '{}' (HTTP {}): {}",
                        "⚠".yellow(), node_id, status,
                        child["error"].as_str().unwrap_or("unknown error"));
                }
            }
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "diagram_id": id,
            "mermaid":    mermaid,
            "drills":     created_drills
        })).unwrap_or_default());
        return;
    }

    println!("{} Generated\n", "✓".green().bold());
    for line in mermaid.lines() {
        println!("  {}", line);
    }

    if !created_drills.is_empty() {
        println!(
            "\n{} {} child diagram{} created:",
            "✓".green().bold(),
            created_drills.len(),
            if created_drills.len() == 1 { "" } else { "s" }
        );
        for d in &created_drills {
            let node = d["node_id"].as_str().unwrap_or("").yellow();
            let cid  = d["diagram_id"].as_str().unwrap_or("").dimmed();
            println!("    {} → {}", node, cid);
        }
    }
}

async fn undo(token: &str, id: &str, json: bool) {
    let client = reqwest::Client::new();
    let resp = match client
        .delete(format!("{}/api/diagrams/{}/chat/last", base_url(), id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => { eprintln!("{} Could not reach server.", "✗".red()); std::process::exit(1); }
    };

    match resp.status().as_u16() {
        401 => { eprintln!("{} Session expired.", "✗".red()); std::process::exit(1); }
        404 => { eprintln!("{} Diagram not found.", "✗".red()); std::process::exit(1); }
        200 => {
            if json {
                println!("{}", serde_json::json!({"ok": true, "diagram_id": id}));
            } else {
                println!("{} Last AI turn removed from {}", "✓".green().bold(), id.dimmed());
            }
        }
        s => { eprintln!("{} Unexpected status {}.", "✗".red(), s); std::process::exit(1); }
    }
}

async fn rename(token: &str, id: &str, name: &str, json: bool) {
    let client = reqwest::Client::new();
    let resp = match client
        .patch(format!("{}/api/diagrams/{}/meta", base_url(), id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "name": name }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => { eprintln!("{} Could not reach server.", "✗".red()); std::process::exit(1); }
    };

    match resp.status().as_u16() {
        401 => { eprintln!("{} Session expired.", "✗".red()); std::process::exit(1); }
        404 => { eprintln!("{} Diagram not found.", "✗".red()); std::process::exit(1); }
        200 => {
            if json {
                println!("{}", serde_json::json!({"ok": true, "diagram_id": id, "name": name}));
            } else {
                println!("{} Renamed to {}", "✓".green().bold(), name.green());
            }
        }
        s => { eprintln!("{} Unexpected status {}.", "✗".red(), s); std::process::exit(1); }
    }
}

async fn delete(token: &str, id: &str, force: bool, json: bool) {
    if !force && !json {
        let confirmed = Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!(
                "Delete diagram {}? This also deletes all child diagrams.",
                id.yellow()
            ))
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
        .delete(format!("{}/api/diagrams/{}", base_url(), id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => { eprintln!("{} Could not reach server.", "✗".red()); std::process::exit(1); }
    };

    match resp.status().as_u16() {
        401 => { eprintln!("{} Session expired.", "✗".red()); std::process::exit(1); }
        404 => { eprintln!("{} Diagram not found.", "✗".red()); std::process::exit(1); }
        200 => {
            if json {
                println!("{}", serde_json::json!({"ok": true, "diagram_id": id}));
            } else {
                println!("{} Deleted {}", "✓".green().bold(), id.dimmed());
            }
        }
        s => { eprintln!("{} Unexpected status {}.", "✗".red(), s); std::process::exit(1); }
    }
}

fn format_cmd(_json: bool) {
    let spec = serde_json::json!({
        "supported_types": [
            {
                "type": "flowchart",
                "keyword": "graph TD / graph LR",
                "when": "Architecture, service maps, general flows",
                "example": "graph TD\n    A[User] --> B[API Gateway]\n    B --> C[Auth]\n    B --> D[Payment]"
            },
            {
                "type": "er",
                "keyword": "erDiagram",
                "when": "Database schema, entity relationships",
                "example": "erDiagram\n    USER ||--o{ ORDER : places\n    ORDER ||--|{ LINE_ITEM : contains"
            },
            {
                "type": "sequence",
                "keyword": "sequenceDiagram",
                "when": "Request/response flows, inter-service calls",
                "example": "sequenceDiagram\n    Client->>API: POST /pay\n    API->>Stripe: charge\n    Stripe-->>API: ok\n    API-->>Client: 200"
            },
            {
                "type": "state",
                "keyword": "stateDiagram-v2",
                "when": "Order lifecycle, auth state, resource states",
                "example": "stateDiagram-v2\n    [*] --> Pending\n    Pending --> Processing\n    Processing --> Complete\n    Processing --> Failed"
            },
            {
                "type": "class",
                "keyword": "classDiagram",
                "when": "Domain model, OOP hierarchy, type relationships",
                "example": "classDiagram\n    Animal <|-- Dog\n    Animal : +name\n    Animal : +speak()"
            },
            {
                "type": "journey",
                "keyword": "journey",
                "when": "User journeys, onboarding flows",
                "example": "journey\n    title Checkout\n    section Cart\n      Add item: 5: User\n    section Pay\n      Enter card: 3: User"
            }
        ],
        "drill_syntax": "%% vaxis:drill <nodeId>",
        "drill_description": "Add this comment after any node to mark it as a drill target. The CLI auto-creates child diagrams for every drill block returned by generate.",
        "node_id_rules": [
            "Alphanumeric and underscores only — no spaces",
            "camelCase or snake_case both fine",
            "Must be unique within a diagram",
            "Keep short — 1 to 3 words"
        ],
        "limits": {
            "max_nodes_per_diagram": 50,
            "max_edges_per_diagram": 60,
            "recommendation": "Use drill blocks when a diagram exceeds 30 nodes"
        },
        "best_practices": [
            "graph TD for architecture (top-down)",
            "graph LR for pipelines and data flows (left-right)",
            "Group related nodes in subgraphs",
            "Label every edge — arrows with labels communicate intent",
            "Root diagrams: broad strokes (services, domains)",
            "Child diagrams: fine detail (functions, data models, steps)"
        ]
    });
    println!("{}", serde_json::to_string_pretty(&spec).unwrap_or_default());
}

async fn patch(token: &str, id: &str, diff: &str, json: bool) {
    let diff_val: serde_json::Value = match serde_json::from_str(diff) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{} Invalid JSON in --diff: {}", "✗".red(), e);
            std::process::exit(1);
        }
    };

    let client = reqwest::Client::new();
    let resp = match client
        .post(format!("{}/api/diagrams/{}/patch", base_url(), id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&diff_val)
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => { eprintln!("{} Could not reach server.", "✗".red()); std::process::exit(1); }
    };

    match resp.status().as_u16() {
        401 => { eprintln!("{} Session expired.", "✗".red()); std::process::exit(1); }
        404 => { eprintln!("{} Diagram not found.", "✗".red()); std::process::exit(1); }
        200 => {
            let result: serde_json::Value = resp.json().await.unwrap_or_default();
            if json {
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
            } else {
                println!("{} Patch applied", "✓".green().bold());
                if let Some(mermaid) = result["mermaid"].as_str() {
                    for line in mermaid.lines() {
                        println!("  {}", line);
                    }
                }
            }
        }
        s => { eprintln!("{} Unexpected status {}.", "✗".red(), s); std::process::exit(1); }
    }
}

async fn import_cmd(token: &str, id: &str, mermaid: &str, json: bool) {
    let client = reqwest::Client::new();
    let resp = match client
        .post(format!("{}/api/diagrams/{}/import", base_url(), id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "mermaid": mermaid }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => { eprintln!("{} Could not reach server.", "✗".red()); std::process::exit(1); }
    };

    match resp.status().as_u16() {
        401 => { eprintln!("{} Session expired.", "✗".red()); std::process::exit(1); }
        404 => { eprintln!("{} Diagram not found.", "✗".red()); std::process::exit(1); }
        200 => {
            if json {
                println!("{}", serde_json::json!({"ok": true, "diagram_id": id}));
            } else {
                println!("{} Mermaid imported to {}", "✓".green().bold(), id.dimmed());
            }
        }
        s => { eprintln!("{} Unexpected status {}.", "✗".red(), s); std::process::exit(1); }
    }
}
