use crate::cli::{SkillAgent, SkillsAction};
use dialoguer::{Confirm, MultiSelect, Select};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const CORE_SKILL: &str = include_str!("../../skill-data/core/SKILL.md");
const DISCOVERY_SKILL: &str = include_str!("../../skills/vaxis/SKILL.md");
const CHECKSUM_FILE: &str = ".vaxis-skill.sha256";

#[derive(Clone, Copy)]
enum InstallScope {
    Project,
    Global,
}

#[derive(Clone, Copy)]
struct AgentSpec {
    id: SkillAgent,
    display_name: &'static str,
    project_path: &'static str,
    global_path: &'static str,
    refresh_guidance: &'static str,
}

const AGENTS: &[AgentSpec] = &[
    AgentSpec {
        id: SkillAgent::Agents,
        display_name: "Agent Skills compatible hosts",
        project_path: ".agents/skills/vaxis/SKILL.md",
        global_path: ".agents/skills/vaxis/SKILL.md",
        refresh_guidance: "Start a new session or reload skills.",
    },
    AgentSpec {
        id: SkillAgent::Claude,
        display_name: "Claude Code",
        project_path: ".claude/skills/vaxis/SKILL.md",
        global_path: ".claude/skills/vaxis/SKILL.md",
        refresh_guidance: "Start a new Claude Code session.",
    },
    AgentSpec {
        id: SkillAgent::Codex,
        display_name: "Codex",
        project_path: ".agents/skills/vaxis/SKILL.md",
        global_path: ".codex/skills/vaxis/SKILL.md",
        refresh_guidance: "Start a new Codex session or reload skills.",
    },
];

#[derive(Serialize)]
struct BundledSkill {
    name: &'static str,
    description: &'static str,
    source: String,
}

#[derive(Serialize)]
struct InstallResult {
    agent: &'static str,
    path: String,
    status: &'static str,
    backup: Option<String>,
    refresh_guidance: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub fn run(action: SkillsAction, json: bool) {
    match action {
        SkillsAction::List => {
            let skill = BundledSkill {
                name: "core",
                description: "Complete Vaxis diagram and CLI instructions",
                source: embedded_source(),
            };
            if json {
                println!("{}", serde_json::to_string(&vec![skill]).unwrap());
            } else {
                println!("core\tComplete Vaxis diagram and CLI instructions");
            }
        }
        SkillsAction::Get { name } => {
            require_core(&name);
            print!("{CORE_SKILL}");
            io::stdout().flush().ok();
        }
        SkillsAction::Path { name } => {
            require_core(&name);
            if json {
                println!(
                    "{}",
                    serde_json::json!({"name": "core", "source": embedded_source()})
                );
            } else {
                println!("{}", embedded_source());
            }
        }
        SkillsAction::Preview { name } => {
            require_core(&name);
            print!("{CORE_SKILL}");
            io::stdout().flush().ok();
        }
    }
}

pub fn install(
    skills: bool,
    requested_agents: Vec<SkillAgent>,
    project: bool,
    global: bool,
    yes: bool,
    force: bool,
    json: bool,
) {
    if !skills {
        fail("nothing selected; use `vaxis install --skills`");
    }

    let interactive = !yes && io::stdin().is_terminal() && io::stdout().is_terminal();
    let agents = select_agents(requested_agents, yes, interactive);
    let scope = select_scope(project, global, yes, interactive);
    let mut seen_paths = HashSet::new();
    let mut results = Vec::new();
    let mut had_errors = false;

    for agent in agents {
        let spec = agent_spec(agent);
        let path = install_path(spec, scope);
        let path_key = path.to_string_lossy().to_lowercase();
        if !seen_paths.insert(path_key) {
            continue;
        }

        match install_one(&path, force, interactive) {
            Ok((status, backup)) => results.push(InstallResult {
                agent: spec.id.as_str(),
                path: path.display().to_string(),
                status,
                backup: backup.map(|p| p.display().to_string()),
                refresh_guidance: spec.refresh_guidance,
                error: None,
            }),
            Err(message) => {
                had_errors = true;
                results.push(InstallResult {
                    agent: spec.id.as_str(),
                    path: path.display().to_string(),
                    status: "conflict",
                    backup: None,
                    refresh_guidance: spec.refresh_guidance,
                    error: Some(message),
                });
            }
        }
    }

    if json {
        println!("{}", serde_json::to_string(&results).unwrap());
    } else {
        for result in results {
            println!("{}: {} ({})", result.agent, result.path, result.status);
            if let Some(backup) = result.backup {
                println!("  backup: {backup}");
            }
            if let Some(error) = result.error {
                eprintln!("  error: {error}");
            } else {
                println!("  {}", result.refresh_guidance);
            }
        }
    }
    if had_errors {
        std::process::exit(1);
    }
}

fn embedded_source() -> String {
    format!("embedded:v{}/core", env!("CARGO_PKG_VERSION"))
}

fn require_core(name: &str) {
    if name != "core" {
        fail(&format!("unknown bundled skill `{name}`; available skill: core"));
    }
}

fn select_agents(
    requested: Vec<SkillAgent>,
    yes: bool,
    interactive: bool,
) -> Vec<SkillAgent> {
    if !requested.is_empty() {
        let mut unique = Vec::new();
        for agent in requested {
            if !unique.contains(&agent) {
                unique.push(agent);
            }
        }
        return unique;
    }
    if yes {
        return AGENTS.iter().map(|spec| spec.id).collect();
    }
    if !interactive {
        fail("select at least one `--agent`, or use `--yes` for all supported agents");
    }

    let labels: Vec<_> = AGENTS.iter().map(|spec| spec.display_name).collect();
    let selected = MultiSelect::new()
        .with_prompt("Install the Vaxis skill for")
        .items(&labels)
        .interact()
        .unwrap_or_else(|error| fail(&format!("could not read agent selection: {error}")));
    if selected.is_empty() {
        fail("no agents selected");
    }
    selected.into_iter().map(|index| AGENTS[index].id).collect()
}

fn select_scope(project: bool, global: bool, yes: bool, interactive: bool) -> InstallScope {
    if project {
        return InstallScope::Project;
    }
    if global {
        return InstallScope::Global;
    }
    if yes {
        return InstallScope::Project;
    }
    if !interactive {
        fail("select `--project` or `--global`, or use `--yes` for project scope");
    }

    match Select::new()
        .with_prompt("Installation scope")
        .items(&["Current project", "Current user (global)"])
        .default(0)
        .interact()
        .unwrap_or_else(|error| fail(&format!("could not read scope selection: {error}")))
    {
        0 => InstallScope::Project,
        _ => InstallScope::Global,
    }
}

fn agent_spec(agent: SkillAgent) -> &'static AgentSpec {
    AGENTS
        .iter()
        .find(|spec| spec.id == agent)
        .expect("every SkillAgent must have one canonical mapping")
}

fn install_path(spec: &AgentSpec, scope: InstallScope) -> PathBuf {
    match scope {
        InstallScope::Project => std::env::current_dir()
            .expect("cannot resolve current directory")
            .join(spec.project_path),
        InstallScope::Global => dirs::home_dir()
            .unwrap_or_else(|| fail("cannot resolve the current user's home directory"))
            .join(spec.global_path),
    }
}

fn install_one(
    destination: &Path,
    force: bool,
    interactive: bool,
) -> Result<(&'static str, Option<PathBuf>), String> {
    let parent = destination
        .parent()
        .ok_or_else(|| format!("invalid skill destination: {}", destination.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;

    let new_checksum = sha256(DISCOVERY_SKILL.as_bytes());
    let checksum_path = parent.join(CHECKSUM_FILE);

    if !destination.exists() {
        write_skill(destination, &checksum_path, &new_checksum)?;
        return Ok(("installed", None));
    }

    let existing = fs::read(destination)
        .map_err(|error| format!("cannot read {}: {error}", destination.display()))?;
    let existing_checksum = sha256(&existing);
    if existing_checksum == new_checksum {
        fs::write(&checksum_path, &new_checksum)
            .map_err(|error| format!("cannot write {}: {error}", checksum_path.display()))?;
        return Ok(("unchanged", None));
    }

    let managed_checksum = fs::read_to_string(&checksum_path)
        .ok()
        .map(|value| value.trim().to_owned());
    if managed_checksum.as_deref() == Some(existing_checksum.as_str()) {
        write_skill(destination, &checksum_path, &new_checksum)?;
        return Ok(("upgraded", None));
    }

    let replace = if force {
        true
    } else if interactive {
        Confirm::new()
            .with_prompt(format!(
                "{} was modified. Back it up and replace it?",
                destination.display()
            ))
            .default(false)
            .interact()
            .map_err(|error| format!("could not read overwrite confirmation: {error}"))?
    } else {
        false
    };
    if !replace {
        return Err(format!(
            "{} contains user modifications; rerun interactively or pass `--force`",
            destination.display()
        ));
    }

    let backup = backup_path(destination);
    fs::copy(destination, &backup).map_err(|error| {
        format!(
            "cannot back up {} to {}: {error}",
            destination.display(),
            backup.display()
        )
    })?;
    write_skill(destination, &checksum_path, &new_checksum)?;
    Ok(("replaced", Some(backup)))
}

fn write_skill(destination: &Path, checksum_path: &Path, checksum: &str) -> Result<(), String> {
    fs::write(destination, DISCOVERY_SKILL)
        .map_err(|error| format!("cannot write {}: {error}", destination.display()))?;
    fs::write(checksum_path, checksum)
        .map_err(|error| format!("cannot write {}: {error}", checksum_path.display()))
}

fn backup_path(destination: &Path) -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    destination.with_file_name(format!("SKILL.md.{timestamp}.bak"))
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn fail(message: &str) -> ! {
    eprintln!("Error: {message}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_skill_path(test_name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("vaxis-skills-{}-{nonce}", std::process::id()))
            .join(test_name)
            .join("SKILL.md")
    }

    #[test]
    fn embedded_skills_have_required_frontmatter() {
        for (name, content) in [("core", CORE_SKILL), ("discovery", DISCOVERY_SKILL)] {
            assert!(content.starts_with("---\n"), "{name} skill has no frontmatter");
            assert!(content.contains("\nname: "), "{name} skill has no name");
            assert!(
                content.contains("\ndescription: "),
                "{name} skill has no description"
            );
            assert!(
                content[4..].contains("\n---\n"),
                "{name} skill has no closing frontmatter delimiter"
            );
        }
    }

    #[test]
    fn every_agent_has_canonical_paths() {
        assert_eq!(AGENTS.len(), 3);
        for spec in AGENTS {
            assert!(spec.project_path.ends_with("/vaxis/SKILL.md"));
            assert!(spec.global_path.ends_with("/vaxis/SKILL.md"));
            assert!(!spec.refresh_guidance.is_empty());
        }
    }

    #[test]
    fn sha256_matches_known_value() {
        assert_eq!(
            sha256(b"vaxis"),
            "07789e71dc70ace952563427f01ab2398ecb169554a2cde6fbdb826a5968ee31"
        );
    }

    #[test]
    fn install_is_idempotent_and_records_checksum() {
        let path = temporary_skill_path("idempotent");
        let parent = path.parent().unwrap();

        let first = install_one(&path, false, false).unwrap();
        assert_eq!(first.0, "installed");
        assert_eq!(fs::read_to_string(&path).unwrap(), DISCOVERY_SKILL);
        assert_eq!(
            fs::read_to_string(parent.join(CHECKSUM_FILE)).unwrap(),
            sha256(DISCOVERY_SKILL.as_bytes())
        );

        let second = install_one(&path, false, false).unwrap();
        assert_eq!(second.0, "unchanged");
        fs::remove_dir_all(parent.parent().unwrap()).unwrap();
    }

    #[test]
    fn modified_skill_requires_force_and_is_backed_up() {
        let path = temporary_skill_path("force");
        let parent = path.parent().unwrap();
        install_one(&path, false, false).unwrap();
        fs::write(&path, "user changes").unwrap();

        assert!(install_one(&path, false, false).is_err());
        let replaced = install_one(&path, true, false).unwrap();
        assert_eq!(replaced.0, "replaced");
        let backup = replaced.1.expect("forced replacement must create a backup");
        assert_eq!(fs::read_to_string(backup).unwrap(), "user changes");
        assert_eq!(fs::read_to_string(&path).unwrap(), DISCOVERY_SKILL);
        fs::remove_dir_all(parent.parent().unwrap()).unwrap();
    }
}
