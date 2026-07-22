use colored::Colorize;
use dialoguer::{Select, Confirm, theme::ColorfulTheme};
use crate::cli::{DiagramsAction, SessionsAction};
use crate::config;

fn auth_token() -> Option<String> {
    config::load().user.map(|u| u.token)
}

/// Preflight-lint Mermaid before a `--mermaid` generate. Returns `true` to
/// proceed, `false` to abort (errors were found and already reported). Warnings
/// are printed but never block. In `--json` mode an abort prints a machine
/// error object to stdout; warnings go to stderr so stdout stays clean for the
/// eventual result JSON.
fn preflight_mermaid(mermaid: &str, json: bool) -> bool {
    use crate::mermaid_lint::{lint, Level};
    let report = lint(mermaid);
    if report.issues.is_empty() {
        return true;
    }
    let has_errors = report.has_errors();

    if json {
        for w in report.warnings() {
            eprintln!("{} {}", "⚠".yellow(), w.message);
        }
        if has_errors {
            let issues: Vec<serde_json::Value> = report
                .issues
                .iter()
                .filter(|i| i.level == Level::Error)
                .map(|i| serde_json::json!({ "code": i.code, "message": i.message, "line": i.line }))
                .collect();
            println!(
                "{}",
                serde_json::json!({ "error": "mermaid_lint_failed", "issues": issues })
            );
            return false;
        }
        return true;
    }

    for e in report.errors() {
        eprintln!("{} {}", "✗".red(), e.message);
    }
    for w in report.warnings() {
        eprintln!("{} {}", "⚠".yellow(), w.message);
    }
    if has_errors {
        eprintln!(
            "{} Fix the Mermaid and try again — nothing was sent. See {} for the drill rules.",
            "→".dimmed(),
            "vaxis diagrams format".yellow()
        );
        return false;
    }
    true
}

/// First-run preference. The first time a human runs `diagrams generate` in a
/// real terminal, ask whether diagrams should be authored by their own AI
/// (`--mermaid`) or by Vaxis's server AI (`--prompt`), and remember the choice
/// so the assistant can honor it (see skills/SKILL.md — "generation mode").
///
/// Silent no-op — never blocks — when the mode is already set, in `--json` mode,
/// or when there is no interactive terminal (e.g. the CLI is being driven by an
/// assistant). Those cases just proceed with whatever flag was passed.
fn ensure_generation_mode(json: bool) {
    use std::io::IsTerminal;
    if json || config::generation_mode().is_some() {
        return;
    }
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return;
    }
    // Flag first so the labels don't depend on manual column alignment.
    let options = [
        "--mermaid — your own AI (Claude / Codex) writes the diagrams (recommended)",
        "--prompt — Vaxis's own AI generates the diagrams for you",
    ];
    let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("How should Vaxis generate diagrams? (saved — you won't be asked again)")
        .items(&options)
        .default(0)
        .interact();
    let mode = match choice {
        Ok(0) => "mermaid",
        Ok(1) => "prompt",
        _ => return, // cancelled — leave unset so we ask again next time
    };
    config::set_generation_mode(mode);
    println!(
        "{} Saved generation mode: {}. Change it anytime with {}.",
        "✓".green(),
        mode.cyan(),
        "vaxis config set-mode <mermaid|prompt>".yellow()
    );
}

pub async fn run(action: DiagramsAction, json: bool) {
    // `format` is a static reference — no auth or network needed. Handle it
    // before the auth gate so a syntax lookup works even when logged out.
    match &action {
        DiagramsAction::Format => {
            format_cmd(json);
            return;
        }
        DiagramsAction::Evaluate { captures, output } => {
            evaluate_cmd(captures, output.as_deref());
            return;
        }
        _ => {}
    }

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
        DiagramsAction::Generate {
            id,
            prompt,
            mermaid,
            intent,
            session,
            direction_policy,
            explicit_direction,
            fresh_generation,
            viewport_width,
            viewport_height,
        } => {
            if prompt.is_none() && mermaid.is_none() {
                if json {
                    println!("{}", serde_json::json!({"error": "provide --prompt or --mermaid"}));
                } else {
                    eprintln!("{} Provide either --prompt or --mermaid", "✗".red());
                }
                std::process::exit(1);
            }
            ensure_generation_mode(json);
            generate(
                &token,
                &id,
                prompt.as_deref(),
                mermaid.as_deref(),
                intent.map(|i| i.as_str()),
                session.as_deref(),
                direction_policy.map(|policy| policy.as_str()),
                explicit_direction.map(|direction| direction.as_str()),
                fresh_generation,
                viewport_width.zip(viewport_height),
                json,
            )
            .await
            // Capture the user's mode preference once (no-op when already set /
            // non-interactive). The flag passed to THIS call always wins — the
            // assistant may still pass --prompt when the stored mode is mermaid,
            // or vice versa; this only sets the default that assistants read from
            // `config show`, it never overrides an explicit flag.
        }
        DiagramsAction::Ask { id, prompt, session } => ask(&token, &id, &prompt, session.as_deref(), json).await,
        DiagramsAction::Sessions { action }       => match action {
            SessionsAction::List { id }                       => sessions_list(&token, &id, json).await,
            SessionsAction::Create { id, title }              => sessions_create(&token, &id, title.as_deref(), json).await,
            SessionsAction::Rename { id, session_id, title }  => sessions_rename(&token, &id, &session_id, &title, json).await,
        },
        DiagramsAction::Share { id, rotate, revoke } => share(&token, &id, rotate, revoke, json).await,
        DiagramsAction::Show { id }               => show(&token, &id, json).await,
        DiagramsAction::Tree { id }               => tree_cmd(&token, &id, json).await,
        DiagramsAction::Undo { id }               => undo(&token, &id, json).await,
        DiagramsAction::Rename { id, name }       => rename(&token, &id, &name, json).await,
        DiagramsAction::Delete { id, app_id, force } => {
            let resolved = resolve_diagram_id(&token, id, app_id, "Select diagram to delete:").await;
            delete(&token, &resolved, force, json).await;
        }
        DiagramsAction::Format                 => format_cmd(json),
        DiagramsAction::RulesCheck             => rules_check(&token, json).await,
        DiagramsAction::Evaluate { captures, output } => evaluate_cmd(&captures, output.as_deref()),
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
        .get(format!("{}/api/applications", crate::config::base_url()))
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
    // Diagrams are listed under the application (root diagrams only). The old
    // `GET /api/diagrams?applicationId=` route was removed in the backend refactor.
    let resp = match client
        .get(format!("{}/api/applications/{}/diagrams", crate::config::base_url(), app_id))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => { eprintln!("{} Could not reach server.", "✗".red()); std::process::exit(1); }
    };
    match resp.status().as_u16() {
        401 => {
            eprintln!("{} Session expired. Run {} again.", "✗".red(), "vaxis login".yellow());
            std::process::exit(1);
        }
        404 => { eprintln!("{} Application not found.", "✗".red()); std::process::exit(1); }
        200 => {}
        s => { eprintln!("{} Could not list diagrams (HTTP {}).", "✗".red(), s); std::process::exit(1); }
    }
    // Report a non-array body instead of silently collapsing it to an empty list
    // (previously `unwrap_or_default()` hid moved endpoints / error objects).
    match resp.json::<serde_json::Value>().await {
        Ok(serde_json::Value::Array(arr)) => arr,
        _ => {
            eprintln!("{} Unexpected response from server (expected a diagram list).", "✗".red());
            std::process::exit(1);
        }
    }
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
        .post(format!("{}/api/diagrams", crate::config::base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "application_id": app_id, "name": name }))
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
        .get(format!("{}/api/diagrams/{}", crate::config::base_url(), id))
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

    // The diagram response now carries `current_mermaid` directly. We no longer
    // make a second `/chat` call: it was an extra round-trip, and taking the last
    // assistant message wrongly surfaced Ask-mode prose answers as if they were
    // the diagram's Mermaid.
    let current_mermaid = diagram["current_mermaid"].as_str().map(|s| s.to_string());

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
        .get(format!("{}/api/diagrams/{}/tree", crate::config::base_url(), id))
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

async fn generate(
    token: &str,
    id: &str,
    prompt: Option<&str>,
    mermaid: Option<&str>,
    intent: Option<&str>,
    session: Option<&str>,
    direction_policy: Option<&str>,
    explicit_direction: Option<&str>,
    fresh_generation: bool,
    viewport: Option<(u32, u32)>,
    json: bool,
) {
    let mut body = if let Some(m) = mermaid {
        // Preflight: catch drill-marker mistakes (indented markers, markers
        // between nodes, unknown ids) deterministically before the round-trip,
        // so the server doesn't silently drop the drills and the browser doesn't
        // get un-renderable Mermaid. Aborts on errors; warnings are advisory.
        if !preflight_mermaid(m, json) {
            std::process::exit(1);
        }
        // Direct-Mermaid path: the server stores the Mermaid without invoking the
        // AI, so `intent` is meaningless here and is ignored (clap also forbids
        // pairing --intent with --mermaid).
        if !json { println!("{}", "Saving diagram...".dimmed()); }
        direct_mermaid_body(
            m,
            direction_policy,
            explicit_direction,
            fresh_generation,
            viewport,
        )
    } else {
        if !json { println!("{}", "Generating...".dimmed()); }
        let mut b = serde_json::json!({ "prompt": prompt.unwrap_or("") });
        if let Some(i) = intent {
            b["intent"] = serde_json::Value::String(i.to_string());
        }
        b
    };
    // A chat session can be targeted on either path.
    if let Some(s) = session {
        body["chat_session_id"] = serde_json::Value::String(s.to_string());
    }

    let client = reqwest::Client::new();
    let resp = match client
        .post(format!("{}/api/diagrams/{}/generate", crate::config::base_url(), id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => { eprintln!("{} Could not reach server.", "✗".red()); std::process::exit(1); }
    };

    let status = resp.status().as_u16();
    let result: serde_json::Value = resp.json().await.unwrap_or_default();

    match status {
        401 => { eprintln!("{} Session expired.", "✗".red()); std::process::exit(1); }
        404 => { eprintln!("{} Diagram not found.", "✗".red()); std::process::exit(1); }
        200 | 201 => {}
        429 => {
            // AI quota / rate limit (AiQuotaException) — surface the server's
            // friendly message instead of a generic "unexpected status".
            let msg = result["error"]["message"].as_str()
                .or_else(|| result["error"].as_str())
                .unwrap_or("You're generating too fast — wait a minute and try again.");
            if json {
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
            } else {
                eprintln!("{} Rate limited: {}", "⚠".yellow(), msg);
            }
            std::process::exit(1);
        }
        _ => {
            let msg = result["error"]["message"].as_str()
                .or_else(|| result["error"].as_str())
                .unwrap_or("unexpected server error");
            if json {
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
            } else {
                eprintln!("{} Generate failed (HTTP {}): {}", "✗".red(), status, msg);
            }
            std::process::exit(1);
        }
    }

    let mermaid = result["mermaid"].as_str().unwrap_or("").to_string();
    let drills  = result["drills"].as_array().cloned().unwrap_or_default();

    // A generate turn does not always edit the diagram. The server routes to Ask
    // whenever intent is "ask" OR intent is "auto" (the default) and the prompt
    // reads as a question — and it answers in prose with `unchanged: true`, an
    // `answer`, and the CURRENT mermaid echoed back. `notice` (no-op/refusal) and
    // `mode_mismatch` (the intent can't do what was asked) are the other no-edit
    // turns. Treating any of these as a successful edit would report "Generated"
    // over unchanged Mermaid and throw the answer away, so they return early.
    let unchanged = result["unchanged"].as_bool().unwrap_or(false);
    let answer    = result["answer"].as_str();
    let notice    = result["notice"].as_str();
    let mismatch  = result["mode_mismatch"]["message"].as_str();
    // The server assigns/echoes the chat session this turn ran in. Surface it so
    // a caller can target the same session later with `--session`.
    let chat_session_id = result["chat_session_id"].as_str();

    // A generate turn is a NON-edit when any of these hold:
    //   - `unchanged`     — hint-fallback / too-large / dropped-nodes / no-op / mode_mismatch
    //   - `answer`        — Ask-mode prose reply
    //   - `mode_mismatch` — the intent can't do what was asked (also sets unchanged)
    //   - `actions`       — a delete confirmation (`delete_self|delete_node|delete_child`);
    //                       these carry `unchanged:false` + a destructive `notice`, so the
    //                       plain `unchanged` check misses them (this was the bug).
    // A real edit can still carry a `notice` (e.g. a truncation "may be incomplete"
    // advisory) — that case has NO actions and unchanged:false, so it correctly falls
    // through to the edit path below, where the notice is surfaced after the diagram.
    let has_actions = result["actions"].as_array().map(|a| !a.is_empty()).unwrap_or(false);

    if unchanged || answer.is_some() || mismatch.is_some() || has_actions {
        if json {
            let mut out = serde_json::json!({
                "diagram_id": id,
                "mermaid":    mermaid,
                "drills":     [],
                "unchanged":  unchanged,
            });
            if let Some(a) = answer   { out["answer"] = serde_json::Value::String(a.to_string()); }
            if let Some(n) = notice   { out["notice"] = serde_json::Value::String(n.to_string()); }
            if let Some(s) = chat_session_id { out["chat_session_id"] = serde_json::Value::String(s.to_string()); }
            if let Some(m) = result.get("mode_mismatch") {
                if !m.is_null() { out["mode_mismatch"] = m.clone(); }
            }
            if has_actions { out["actions"] = result["actions"].clone(); }
            println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
            return;
        }

        match answer.or(mismatch).or(notice) {
            Some(text) => {
                println!("\n{} The diagram was not changed:\n", "ℹ".cyan().bold());
                for line in text.lines() {
                    println!("  {}", line);
                }
                if answer.is_some() {
                    println!("\n{}", "Use `vaxis diagrams ask` for questions, or an explicit \
                                      --intent edit|replace|drill to change the diagram.".dimmed());
                }
            }
            None => println!("{} No change was made to the diagram.", "ℹ".cyan().bold()),
        }
        return;
    }

    // Create child diagrams for every drill block the AI returned
    let mut created_drills: Vec<serde_json::Value> = Vec::new();
    for drill in &drills {
        let node_id = drill["node_id"].as_str().unwrap_or("");
        if node_id.is_empty() { continue; }

        // Seed the child with the drill's Mermaid when the generate response
        // included one, so a drilled-in diagram opens pre-populated instead of
        // empty. The field is ignored by backends that don't support it.
        let mut child_body = serde_json::json!({ "node_id": node_id, "node_label": node_id });
        if let Some(seed) = drill["mermaid"].as_str() {
            if !seed.is_empty() {
                child_body["seed_mermaid"] = serde_json::Value::String(seed.to_string());
            }
        }

        if let Ok(cr) = client
            .post(format!("{}/api/diagrams/{}/children", crate::config::base_url(), id))
            .header("Authorization", format!("Bearer {}", token))
            .json(&child_body)
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
        let mut out = serde_json::json!({
            "diagram_id": id,
            "mermaid":    mermaid,
            "drills":     created_drills
        });
        if let Some(s) = chat_session_id { out["chat_session_id"] = serde_json::Value::String(s.to_string()); }
        // A real edit can still ship an advisory notice (e.g. truncation). Keep it.
        if let Some(n) = notice { out["notice"] = serde_json::Value::String(n.to_string()); }
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
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

    // The edit went through but the server flagged something (usually truncation).
    if let Some(n) = notice {
        println!("\n{} {}", "⚠".yellow(), n);
    }
}

fn direct_mermaid_body(
    mermaid: &str,
    direction_policy: Option<&str>,
    explicit_direction: Option<&str>,
    fresh_generation: bool,
    viewport: Option<(u32, u32)>,
) -> serde_json::Value {
    let mut body = serde_json::json!({ "mermaid": mermaid });
    if direction_policy.is_none()
        && explicit_direction.is_none()
        && !fresh_generation
        && viewport.is_none()
    {
        return body;
    }
    let mut context = serde_json::json!({
        "policy": direction_policy.unwrap_or("preserve"),
        "is_fresh_generation": fresh_generation,
    });
    if let Some(direction) = explicit_direction {
        context["explicit"] = serde_json::Value::String(direction.to_string());
    }
    if let Some((width, height)) = viewport {
        context["viewport"] = serde_json::json!({ "width": width, "height": height });
    }
    body["direction_context"] = context;
    body
}

#[cfg(test)]
mod direct_direction_tests {
    use super::direct_mermaid_body;

    #[test]
    fn old_direct_mermaid_body_stays_unchanged() {
        assert_eq!(
            direct_mermaid_body("flowchart TB\n  a[A]", None, None, false, None),
            serde_json::json!({ "mermaid": "flowchart TB\n  a[A]" })
        );
    }

    #[test]
    fn opt_in_direction_context_is_serialized() {
        assert_eq!(
            direct_mermaid_body(
                "flowchart TB\n  a[A] --> b[B] --> c[C]",
                Some("auto"),
                Some("LR"),
                true,
                Some((1440, 900)),
            )["direction_context"],
            serde_json::json!({
                "policy": "auto",
                "explicit": "LR",
                "is_fresh_generation": true,
                "viewport": { "width": 1440, "height": 900 }
            })
        );
    }
}

// Sharing is per-diagram: one link unlocks this diagram plus the sub-diagrams it
// drills into. `POST /share` always mints a fresh token pair (it is create-OR-ROTATE,
// not get-or-create), so a plain `share` reads the existing link first and only
// POSTs when there is none — otherwise repeat calls would silently invalidate a
// link that is already handed out. `--rotate` asks for that new token explicitly.
async fn share(token: &str, id: &str, rotate: bool, revoke: bool, json: bool) {
    let client = reqwest::Client::new();
    let url = format!("{}/api/diagrams/{}/share", crate::config::base_url(), id);

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
            404 => { eprintln!("{} Diagram not found.", "✗".red()); std::process::exit(1); }
            200 => {
                if json {
                    println!("{}", serde_json::json!({"ok": true, "diagram_id": id, "shared": false}));
                } else {
                    println!("{} Sharing disabled for {}", "✓".green().bold(), id.dimmed());
                }
            }
            s => { eprintln!("{} Unexpected status {}.", "✗".red(), s); std::process::exit(1); }
        }
        return;
    }

    // Always read current state first. A plain `share` returns the existing link
    // (never rotating it), and reading first is also what lets `--rotate` tell
    // whether it actually replaced a live link or just minted the first one — so
    // the "previous link no longer works" warning is only shown when it's true.
    let get_resp = match client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => { eprintln!("{} Could not reach server.", "✗".red()); std::process::exit(1); }
    };
    let had_link = match get_resp.status().as_u16() {
        401 => { eprintln!("{} Session expired. Run {} again.", "✗".red(), "vaxis login".yellow()); std::process::exit(1); }
        404 => { eprintln!("{} Diagram not found.", "✗".red()); std::process::exit(1); }
        200 => {
            let body: serde_json::Value = get_resp.json().await.unwrap_or_default();
            match share_token(&body, "token") {
                Some(tok) => {
                    if !rotate {
                        print_share_links(id, &tok, share_token(&body, "edit_token").as_deref(), false, json);
                        return;
                    }
                    true // a live link exists and the caller asked to rotate it
                }
                None => false, // not shared yet — the POST below mints the first link
            }
        }
        s => { eprintln!("{} Unexpected status {}.", "✗".red(), s); std::process::exit(1); }
    };

    let resp = match client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => { eprintln!("{} Could not reach server.", "✗".red()); std::process::exit(1); }
    };

    match resp.status().as_u16() {
        401 => { eprintln!("{} Session expired. Run {} again.", "✗".red(), "vaxis login".yellow()); std::process::exit(1); }
        404 => { eprintln!("{} Diagram not found.", "✗".red()); std::process::exit(1); }
        200 | 201 => {
            let body: serde_json::Value = resp.json().await.unwrap_or_default();
            let tok = match share_token(&body, "token") {
                Some(t) => t,
                None => { eprintln!("{} Server returned no share token.", "✗".red()); std::process::exit(1); }
            };
            print_share_links(id, &tok, share_token(&body, "edit_token").as_deref(), had_link, json);
        }
        s => { eprintln!("{} Unexpected status {}.", "✗".red(), s); std::process::exit(1); }
    }
}

// The share endpoints return `null` for an unshared diagram, and some payloads
// carry the string "null" — treat both, and the empty string, as "no token".
fn share_token(body: &serde_json::Value, field: &str) -> Option<String> {
    match body[field].as_str() {
        Some(t) if !t.is_empty() && t != "null" => Some(t.to_string()),
        _ => None,
    }
}

// The backend returns tokens only; the CLI builds the URLs, matching the web
// share dialog: /view/<token> is read-only, /collab/<edit_token> can edit.
fn print_share_links(id: &str, token: &str, edit_token: Option<&str>, replaced: bool, json: bool) {
    let base = crate::config::base_url();
    let view_url = format!("{}/view/{}", base, token);
    let edit_url = edit_token.map(|t| format!("{}/collab/{}", base, t));

    if json {
        let mut out = serde_json::json!({
            "diagram_id": id,
            "shared": true,
            "url": view_url,
            "token": token,
        });
        if let (Some(t), Some(u)) = (edit_token, edit_url.as_ref()) {
            out["edit_token"] = serde_json::Value::String(t.to_string());
            out["edit_url"]   = serde_json::Value::String(u.clone());
        }
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
        return;
    }

    if replaced {
        println!("{} New link minted — the previous one no longer works.", "⚠".yellow());
    }
    println!("{} View link: {}", "✓".green().bold(), view_url.cyan());
    if let Some(u) = edit_url {
        println!("{} Edit link: {}", "✓".green().bold(), u.cyan());
    }
}

async fn ask(token: &str, id: &str, prompt: &str, session: Option<&str>, json: bool) {
    // Ask mode is the generate endpoint with intent "ask": the server answers in
    // prose and makes no edit to the diagram (`unchanged: true`, `answer: "..."`).
    let mut body = serde_json::json!({ "prompt": prompt, "intent": "ask" });
    if let Some(s) = session {
        body["chat_session_id"] = serde_json::Value::String(s.to_string());
    }
    if !json { println!("{}", "Asking...".dimmed()); }

    let client = reqwest::Client::new();
    let resp = match client
        .post(format!("{}/api/diagrams/{}/generate", crate::config::base_url(), id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => { eprintln!("{} Could not reach server.", "✗".red()); std::process::exit(1); }
    };

    let status = resp.status().as_u16();
    let result: serde_json::Value = resp.json().await.unwrap_or_default();

    match status {
        401 => { eprintln!("{} Session expired.", "✗".red()); std::process::exit(1); }
        404 => { eprintln!("{} Diagram not found.", "✗".red()); std::process::exit(1); }
        200 | 201 => {}
        429 => {
            let msg = result["error"]["message"].as_str()
                .or_else(|| result["error"].as_str())
                .unwrap_or("You're generating too fast — wait a minute and try again.");
            if json {
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
            } else {
                eprintln!("{} Rate limited: {}", "⚠".yellow(), msg);
            }
            std::process::exit(1);
        }
        _ => {
            let msg = result["error"]["message"].as_str()
                .or_else(|| result["error"].as_str())
                .unwrap_or("unexpected server error");
            if json {
                println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
            } else {
                eprintln!("{} Ask failed (HTTP {}): {}", "✗".red(), status, msg);
            }
            std::process::exit(1);
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&result).unwrap_or_default());
        return;
    }

    // `answer` is the prose reply; some no-op turns use `notice` instead. Ask
    // forces intent "ask", so the server should always answer in prose — if it
    // returned neither, say so plainly rather than printing a bare placeholder.
    match result["answer"].as_str().or_else(|| result["notice"].as_str()) {
        Some(text) => {
            println!();
            for line in text.lines() {
                println!("  {}", line);
            }
        }
        None => {
            eprintln!("{} The server returned no answer for this question.", "⚠".yellow());
            std::process::exit(1);
        }
    }
}

async fn sessions_list(token: &str, id: &str, json: bool) {
    let client = reqwest::Client::new();
    let resp = match client
        .get(format!("{}/api/diagrams/{}/chat/sessions", crate::config::base_url(), id))
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
        200 => {}
        s => { eprintln!("{} Could not list sessions (HTTP {}).", "✗".red(), s); std::process::exit(1); }
    }

    let data: serde_json::Value = resp.json().await.unwrap_or_default();

    if json {
        println!("{}", serde_json::to_string_pretty(&data).unwrap_or_default());
        return;
    }

    let active = data["active_chat_session_id"].as_str().unwrap_or("");
    let sessions = data["sessions"].as_array().cloned().unwrap_or_default();
    if sessions.is_empty() {
        println!("{}", "No chat sessions yet. Start one with: vaxis diagrams sessions create <id>".dimmed());
        return;
    }

    println!("{}", "─".repeat(56).dimmed());
    for s in &sessions {
        let sid   = s["id"].as_str().unwrap_or("");
        let title = s["title"].as_str().unwrap_or("Untitled");
        let count = s["message_count"].as_u64().unwrap_or(0);
        let marker = if sid == active { "●".green().to_string() } else { " ".to_string() };
        println!("  {} {}  {} {}",
            marker,
            title.bold(),
            sid.dimmed(),
            format!("({} msg{})", count, if count == 1 { "" } else { "s" }).dimmed());
    }
    println!("{}", "─".repeat(56).dimmed());
    println!("  {} session{}  {}",
        sessions.len().to_string().cyan(),
        if sessions.len() == 1 { "" } else { "s" },
        "(● = active)".dimmed());
}

async fn sessions_create(token: &str, id: &str, title: Option<&str>, json: bool) {
    let mut body = serde_json::Map::new();
    if let Some(t) = title {
        body.insert("title".into(), t.into());
    }

    let client = reqwest::Client::new();
    let resp = match client
        .post(format!("{}/api/diagrams/{}/chat/sessions", crate::config::base_url(), id))
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
        200 | 201 => {}
        s => { eprintln!("{} Could not create session (HTTP {}).", "✗".red(), s); std::process::exit(1); }
    }

    let data: serde_json::Value = resp.json().await.unwrap_or_default();

    if json {
        println!("{}", serde_json::to_string_pretty(&data).unwrap_or_default());
        return;
    }

    let session = &data["session"];
    println!(
        "{} Created chat session {} {}",
        "✓".green().bold(),
        session["title"].as_str().unwrap_or("Untitled").green(),
        session["id"].as_str().unwrap_or("").dimmed()
    );
}

async fn sessions_rename(token: &str, id: &str, session_id: &str, title: &str, json: bool) {
    let client = reqwest::Client::new();
    let resp = match client
        .patch(format!("{}/api/diagrams/{}/chat/sessions/{}", crate::config::base_url(), id, session_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "title": title }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => { eprintln!("{} Could not reach server.", "✗".red()); std::process::exit(1); }
    };

    match resp.status().as_u16() {
        401 => { eprintln!("{} Session expired.", "✗".red()); std::process::exit(1); }
        404 => { eprintln!("{} Diagram or session not found.", "✗".red()); std::process::exit(1); }
        200 => {
            if json {
                let data: serde_json::Value = resp.json().await.unwrap_or_default();
                println!("{}", serde_json::to_string_pretty(&data).unwrap_or_default());
            } else {
                println!("{} Session renamed to {}", "✓".green().bold(), title.green());
            }
        }
        s => { eprintln!("{} Unexpected status {}.", "✗".red(), s); std::process::exit(1); }
    }
}

async fn undo(token: &str, id: &str, json: bool) {
    let client = reqwest::Client::new();
    let resp = match client
        .delete(format!("{}/api/diagrams/{}/chat/messages/last", crate::config::base_url(), id))
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
        .patch(format!("{}/api/diagrams/{}", crate::config::base_url(), id))
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
        .delete(format!("{}/api/diagrams/{}", crate::config::base_url(), id))
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

fn format_spec() -> serde_json::Value {
    serde_json::from_str(include_str!("../diagram_format.json"))
        .expect("embedded diagram format contract must be valid JSON")
}

fn format_cmd(_json: bool) {
    println!("{}", serde_json::to_string_pretty(&format_spec()).unwrap_or_default());
}

fn contract_drift(local: &serde_json::Value, remote: &serde_json::Value) -> Vec<String> {
    let mut drift = Vec::new();
    if local["schema_version"] != remote["schema_version"] {
        drift.push(format!(
            "schema version differs: CLI={} server={}",
            local["schema_version"], remote["schema_version"]
        ));
    }
    let normalized_tokens = |value: &serde_json::Value| {
        value
            .as_array()
            .map(|tokens| {
                tokens
                    .iter()
                    .filter_map(|token| token.as_str())
                    .map(str::to_lowercase)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    };
    let local_tokens = normalized_tokens(&local["authoring_contract"]["shapes"]["storage_keywords"]);
    let remote_tokens = normalized_tokens(&remote["shapes"]["storage_keywords"]);
    if local_tokens != remote_tokens {
        drift.push("storage keyword list differs".to_string());
    }
    if local["limits"]["min_seeded_drill_nodes"] != remote["drills"]["seeded_min_nodes"] {
        drift.push("seeded drill minimum differs".to_string());
    }
    drift
}

async fn rules_check(token: &str, json: bool) {
    let response = reqwest::Client::new()
        .get(format!("{}/api/diagrams/rules", crate::config::base_url()))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await;
    let response = match response {
        Ok(response) => response,
        Err(error) => {
            eprintln!("{} Could not fetch server rules: {}", "✗".red(), error);
            std::process::exit(1);
        }
    };
    if !response.status().is_success() {
        eprintln!("{} Server rules request failed (HTTP {}).", "✗".red(), response.status());
        std::process::exit(1);
    }
    let remote: serde_json::Value = response.json().await.unwrap_or_default();
    let drift = contract_drift(&format_spec(), &remote);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": drift.is_empty(),
                "cli_version": format_spec()["schema_version"],
                "server_version": remote["schema_version"],
                "drift": drift,
            }))
            .unwrap_or_default()
        );
    } else if drift.is_empty() {
        println!("{} Diagram authoring rules match the server.", "✓".green());
    } else {
        for item in &drift {
            eprintln!("{} {}", "✗".red(), item);
        }
    }
    if !drift.is_empty() {
        std::process::exit(2);
    }
}

fn evaluate_cmd(captures: &std::path::Path, output: Option<&std::path::Path>) {
    let report = match crate::parity_eval::evaluate_capture_file(captures) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{} {}", "✗".red(), error);
            std::process::exit(1);
        }
    };
    let json = serde_json::to_string_pretty(&report).expect("parity report must serialize");
    if let Some(path) = output {
        if let Err(error) = std::fs::write(path, &json) {
            eprintln!("{} Could not write {}: {}", "✗".red(), path.display(), error);
            std::process::exit(1);
        }
        eprintln!("{} Wrote parity report to {}", "✓".green(), path.display());
    } else {
        println!("{json}");
    }
    if report.summary.failed_captures > 0 {
        std::process::exit(2);
    }
}

#[cfg(test)]
mod format_tests {
    use super::{contract_drift, format_spec};

    #[test]
    fn format_contract_v1_snapshot() {
        let spec = format_spec();
        assert_eq!(spec["schema_version"], "1.0.0");
        assert_eq!(spec["editable_types"].as_array().map(Vec::len), Some(5));
        assert_eq!(spec["limits"]["min_seeded_drill_nodes"], 3);
        let mut keys: Vec<&str> = spec.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "authoring_contract",
                "best_practices",
                "compatibility",
                "drill_description",
                "drill_syntax",
                "editable_types",
                "editable_types_note",
                "image_fallback_note",
                "image_fallback_types",
                "limits",
                "node_id_rules",
                "preserve_type_on_edit",
                "schema_version",
                "skill_required_phrases",
                "source",
            ]
        );
        assert_eq!(
            spec["authoring_contract"]["shapes"]["storage_keywords"],
            serde_json::json!([
                "db", "database", "store", "storage", "cache", "queue", "bucket",
                "table", "log", "index", "vector store", "blob", "s3", "redis",
                "postgres", "postgresql", "mongo", "mongodb", "mysql", "sqlite",
                "kafka", "sqs", "d1", "kv", "r2", "sql", "nosql", "dynamodb",
                "firestore", "memcached", "elasticsearch", "gcs"
            ])
        );
    }

    #[test]
    fn skill_declares_matching_authoring_contract_version() {
        let spec = format_spec();
        let version = spec["schema_version"].as_str().unwrap();
        let marker = format!("<!-- vaxis-authoring-rules: {version} -->");
        let skill = include_str!("../../skills/SKILL.md");
        assert!(
            skill.contains(&marker),
            "SKILL.md must declare the embedded format contract version"
        );
        for phrase in spec["skill_required_phrases"].as_array().unwrap() {
            let phrase = phrase.as_str().unwrap();
            assert!(skill.contains(phrase), "SKILL.md is missing mirrored rule `{phrase}`");
        }
    }

    #[test]
    fn detects_cross_repository_contract_drift() {
        let local = format_spec();
        let matching = serde_json::json!({
            "schema_version": "1.0.0",
            "shapes": { "storage_keywords": local["authoring_contract"]["shapes"]["storage_keywords"] },
            "drills": { "seeded_min_nodes": 3 }
        });
        assert!(contract_drift(&local, &matching).is_empty());
        let stale = serde_json::json!({
            "schema_version": "2.0.0",
            "shapes": { "storage_keywords": [] },
            "drills": { "seeded_min_nodes": 2 }
        });
        assert_eq!(contract_drift(&local, &stale).len(), 3);
    }
}


async fn import_cmd(token: &str, id: &str, mermaid: &str, json: bool) {
    let client = reqwest::Client::new();
    let resp = match client
        .post(format!("{}/api/diagrams/{}/import", crate::config::base_url(), id))
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
