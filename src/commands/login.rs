use colored::Colorize;
use crate::config::{self, UserProfile};

const DEFAULT_AUTH_URL: &str = "https://vaxis.dev";
const POLL_INTERVAL_SECS: u64 = 2;
const POLL_MAX_ATTEMPTS: u32 = 150; // 5 minutes

pub async fn run() {
    let base_url = std::env::var("VAXIS_AUTH_URL")
        .ok()
        .or_else(|| config::load().auth_url)
        .unwrap_or_else(|| DEFAULT_AUTH_URL.to_string());

    // Step 1: Create a polling state on the server
    let (state, browser_url) = match start_cli_auth(&base_url).await {
        Ok(pair) => pair,
        Err(msg) => {
            eprintln!("{} {}", "✗".red(), msg);
            std::process::exit(1);
        }
    };

    // Step 2: Open the real domain in the browser
    println!("{}", "Opening browser for login...".cyan());
    open::that(&browser_url).expect("failed to open browser");

    // Step 3: Poll until login is complete
    println!("{}", "Waiting for you to complete login in the browser...".dimmed());
    let result = poll_for_token(&base_url, &state).await;

    match result {
        PollResult::Complete { token, user_name, user_email } => {
            let mut cfg = config::load();
            cfg.user = Some(UserProfile {
                name: user_name.clone(),
                email: user_email.clone(),
                token,
            });
            config::save(&cfg);

            println!(
                "{} Logged in as {} ({})",
                "✓".green().bold(),
                user_name.green(),
                user_email.dimmed()
            );
        }
        PollResult::Expired => {
            eprintln!("{} Login timed out. Run {} again.", "✗".red(), "vaxis login".yellow());
            std::process::exit(1);
        }
        PollResult::Error(msg) => {
            eprintln!("{} {}", "✗".red(), msg);
            std::process::exit(1);
        }
    }
}

async fn start_cli_auth(base_url: &str) -> Result<(String, String), String> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/api/cli/start", base_url))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|_| format!(
            "Cannot connect to server at {base_url}\nSet your server URL with: vaxis config set-url http://localhost:3000"
        ))?;
    let body: serde_json::Value = resp
        .json()
        .await
        .map_err(|_| "Server returned an unexpected response.".to_string())?;
    let state = body["state"].as_str().ok_or("missing state in response")?.to_string();
    let url   = body["url"].as_str().ok_or("missing url in response")?.to_string();
    Ok((state, url))
}

enum PollResult {
    Complete { token: String, user_name: String, user_email: String },
    Expired,
    Error(String),
}

async fn poll_for_token(base_url: &str, state: &str) -> PollResult {
    let client = reqwest::Client::new();
    let poll_url = format!("{}/api/cli/poll?state={}", base_url, state);
    let spinner = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
    let mut attempt = 0u32;

    loop {
        if attempt >= POLL_MAX_ATTEMPTS {
            return PollResult::Expired;
        }

        tokio::time::sleep(tokio::time::Duration::from_secs(POLL_INTERVAL_SECS)).await;

        let spin_char = spinner[(attempt as usize) % spinner.len()];
        print!("\r{} Waiting... ", spin_char);

        let resp = match client.get(&poll_url).send().await {
            Ok(r) => r,
            Err(_) => {
                attempt += 1;
                continue;
            }
        };

        let body: serde_json::Value = match resp.json().await {
            Ok(v) => v,
            Err(_) => {
                attempt += 1;
                continue;
            }
        };

        match body["status"].as_str() {
            Some("complete") => {
                print!("\r");
                let token      = body["token"].as_str().unwrap_or("").to_string();
                let user_name  = body["user_name"].as_str().unwrap_or("").to_string();
                let user_email = body["user_email"].as_str().unwrap_or("").to_string();
                return PollResult::Complete { token, user_name, user_email };
            }
            Some("expired") => {
                print!("\r");
                return PollResult::Expired;
            }
            Some("pending") => {}
            _ => {
                return PollResult::Error("Unexpected response from server.".to_string());
            }
        }

        attempt += 1;
    }
}
