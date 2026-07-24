use crate::cli::{SkillAgent, SkillsAction};
use colored::Colorize;
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
// Official full skills shipped before the discovery/core split in v0.4.0.
const LEGACY_MANAGED_SKILL_CHECKSUMS: &[&str] = &[
    "a1671300e28f976c351b45d57649d91a4f37673b95f6026ef1542edbda0bccff",
    "63ff43a3f40980f78e6e854dc64dbdd991e49e8410638294e4bf4b02ed77db0b",
    "6e9682cc4fa7da43a3ab0aa26c68c246c95bc3c1d80eca785d806d096ad43619",
    "6391ed63478a90458bcda67598a999b75512fdcc0858683901e5db4bbe7e67fd",
    "d89a8c653265fb1640720d0e55a050431af86fdfe875b397d9b64f57571a7d52",
    "589690cefc67246b53422867279475588f116ad66c0b89bb0da48954f6c25d6e",
    "af89e6a168bd9dca72a3a5737d9959d329d36a296266b55146fb3d0db57ddb5a",
    "844f61189b4b93818c78ad3dedae4b36398af716114717370d3b082a0807e8ad",
    // The same released files after Git's common Windows LF-to-CRLF checkout conversion.
    "de807b7e04216b27f01ba2f14887c6afb3d040f282815a0166e0a139f5e2c1e8",
    "8f4c7f8fecce1c71fc8c9edd0f16fc84054f799b6365424fabbcaa974f477657",
    "310b925d20b7b1566f1634362febf00ab4e04587aca8c58c3c51a67cfd57b108",
    "0f17a42476648b7cf02163ff8ba659fd164aa714ae84e74eb5fddafc57025763",
    "6665e2003a65e2b914a25ed65960c85842f17c1792086cd847032b816cfd12b0",
    "ce92798bdd21193b389c9003ef73fe6e31ef4c944396fe201846f8c2c4459892",
    "5ce07b88752ff89131fe37e898596d86bb27dd84f4924e3cd60e7ed837737171",
    "8876e0c3e57912014588135439bdc71eb951b3c1ff00e5efc68eef1835513fbe",
    // The final full skill from PR #23, immediately before the v0.4.0 split.
    "7699a8ecf66f1b3490dd07395932332d6c5d52f6436a44300ee736cddcfe4f56",
    "53ac9b4313f66c76d8537d81c60e71b89b2e898697e491dfd19d2a2ff127f281",
];

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
        global_path: ".agents/skills/vaxis/SKILL.md",
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
            require_core(&name, json);
            print_core(json);
        }
        SkillsAction::Path { name } => {
            require_core(&name, json);
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
            require_core(&name, json);
            print_core(json);
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
        fail(
            "invalid_arguments",
            "nothing selected; use `vaxis install --skills`",
            json,
        );
    }

    let interactive =
        !json && !yes && io::stdin().is_terminal() && io::stdout().is_terminal();
    let agents = select_agents(requested_agents, yes, interactive, json);
    let scope = select_scope(project, global, yes, interactive, json);
    let mut results = Vec::new();
    let mut had_errors = false;

    let managed_root = install_root(scope, json);
    for (spec, path) in resolved_targets(&agents, scope, json) {
        match install_one(&path, &managed_root, force, interactive) {
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

fn print_core(json: bool) {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "name": "core",
                "source": embedded_source(),
                "content": CORE_SKILL,
            })
        );
    } else {
        print!("{CORE_SKILL}");
        io::stdout().flush().ok();
    }
}

fn require_core(name: &str, json: bool) {
    if name != "core" {
        fail(
            "unknown_skill",
            &format!("unknown bundled skill `{name}`; available skill: core"),
            json,
        );
    }
}

fn select_agents(
    requested: Vec<SkillAgent>,
    yes: bool,
    interactive: bool,
    json: bool,
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
        fail(
            "invalid_arguments",
            "select at least one `--agent`, or use `--yes` for all supported agents",
            json,
        );
    }

    let labels: Vec<_> = AGENTS.iter().map(|spec| spec.display_name).collect();
    let selected = MultiSelect::new()
        .with_prompt("Install the Vaxis skill for")
        .items(&labels)
        .interact()
        .unwrap_or_else(|error| {
            fail(
                "interactive_error",
                &format!("could not read agent selection: {error}"),
                json,
            )
        });
    if selected.is_empty() {
        fail("invalid_arguments", "no agents selected", json);
    }
    selected.into_iter().map(|index| AGENTS[index].id).collect()
}

fn select_scope(
    project: bool,
    global: bool,
    yes: bool,
    interactive: bool,
    json: bool,
) -> InstallScope {
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
        fail(
            "invalid_arguments",
            "select `--project` or `--global`, or use `--yes` for project scope",
            json,
        );
    }

    match Select::new()
        .with_prompt("Installation scope")
        .items(&["Current project", "Current user (global)"])
        .default(0)
        .interact()
        .unwrap_or_else(|error| {
            fail(
                "interactive_error",
                &format!("could not read scope selection: {error}"),
                json,
            )
        })
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

fn install_path(spec: &AgentSpec, scope: InstallScope, json: bool) -> PathBuf {
    let root = install_root(scope, json);
    match scope {
        InstallScope::Project => root.join(spec.project_path),
        InstallScope::Global => root.join(spec.global_path),
    }
}

fn install_root(scope: InstallScope, json: bool) -> PathBuf {
    match scope {
        InstallScope::Project => std::env::current_dir().unwrap_or_else(|error| {
            fail(
                "path_resolution_failed",
                &format!("cannot resolve current directory: {error}"),
                json,
            )
        }),
        InstallScope::Global => dirs::home_dir().unwrap_or_else(|| {
            fail(
                "path_resolution_failed",
                "cannot resolve the current user's home directory",
                json,
            )
        }),
    }
}

fn resolved_targets(
    agents: &[SkillAgent],
    scope: InstallScope,
    json: bool,
) -> Vec<(&'static AgentSpec, PathBuf)> {
    let mut seen_paths = HashSet::new();
    let mut targets = Vec::new();
    for agent in agents {
        let spec = agent_spec(*agent);
        let path = install_path(spec, scope, json);
        let path_key = path.to_string_lossy().to_lowercase();
        if seen_paths.insert(path_key) {
            targets.push((spec, path));
        }
    }
    targets
}

fn install_one(
    destination: &Path,
    managed_root: &Path,
    force: bool,
    interactive: bool,
) -> Result<(&'static str, Option<PathBuf>), String> {
    install_one_with_legacy_checksums(
        destination,
        managed_root,
        force,
        interactive,
        LEGACY_MANAGED_SKILL_CHECKSUMS,
    )
}

fn install_one_with_legacy_checksums(
    destination: &Path,
    managed_root: &Path,
    force: bool,
    interactive: bool,
    legacy_checksums: &[&str],
) -> Result<(&'static str, Option<PathBuf>), String> {
    validate_install_path(managed_root, destination)?;
    let parent = destination
        .parent()
        .ok_or_else(|| format!("invalid skill destination: {}", destination.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    validate_write_target(managed_root, destination)?;

    let new_checksum = sha256(DISCOVERY_SKILL.as_bytes());
    let checksum_path = parent.join(CHECKSUM_FILE);
    validate_write_target(managed_root, &checksum_path)?;

    if !destination.exists() {
        write_skill(destination, &checksum_path, &new_checksum, managed_root)?;
        return Ok(("installed", None));
    }

    let existing = fs::read(destination)
        .map_err(|error| format!("cannot read {}: {error}", destination.display()))?;
    let existing_checksum = sha256(&existing);
    if existing_checksum == new_checksum {
        validate_write_target(managed_root, &checksum_path)?;
        fs::write(&checksum_path, &new_checksum)
            .map_err(|error| format!("cannot write {}: {error}", checksum_path.display()))?;
        return Ok(("unchanged", None));
    }

    let managed_checksum = fs::read_to_string(&checksum_path)
        .ok()
        .map(|value| value.trim().to_owned());
    if managed_checksum.as_deref() == Some(existing_checksum.as_str()) {
        write_skill(destination, &checksum_path, &new_checksum, managed_root)?;
        return Ok(("upgraded", None));
    }

    if legacy_checksums.contains(&existing_checksum.as_str()) {
        let backup = back_up_skill(destination, managed_root)?;
        write_skill(destination, &checksum_path, &new_checksum, managed_root)?;
        return Ok(("migrated", Some(backup)));
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

    let backup = back_up_skill(destination, managed_root)?;
    write_skill(destination, &checksum_path, &new_checksum, managed_root)?;
    Ok(("replaced", Some(backup)))
}

fn validate_install_path(managed_root: &Path, destination: &Path) -> Result<(), String> {
    let relative = destination.strip_prefix(managed_root).map_err(|_| {
        format!(
            "refusing to write outside managed root {}: {}",
            managed_root.display(),
            destination.display()
        )
    })?;

    let mut current = managed_root.to_path_buf();
    for component in relative.components() {
        use std::path::Component;
        let Component::Normal(component) = component else {
            return Err(format!(
                "refusing unsafe install path: {}",
                destination.display()
            ));
        };
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(format!(
                    "refusing to write through symlink: {}",
                    current.display()
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(format!("cannot inspect {}: {error}", current.display()));
            }
        }
    }
    Ok(())
}

fn ensure_parent_within_root(managed_root: &Path, parent: &Path) -> Result<(), String> {
    let canonical_root = fs::canonicalize(managed_root)
        .map_err(|error| format!("cannot resolve {}: {error}", managed_root.display()))?;
    let canonical_parent = fs::canonicalize(parent)
        .map_err(|error| format!("cannot resolve {}: {error}", parent.display()))?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err(format!(
            "refusing to write outside managed root {}: {}",
            canonical_root.display(),
            canonical_parent.display()
        ));
    }
    Ok(())
}

fn validate_write_target(managed_root: &Path, target: &Path) -> Result<(), String> {
    validate_install_path(managed_root, target)?;
    let parent = target
        .parent()
        .ok_or_else(|| format!("invalid skill destination: {}", target.display()))?;
    ensure_parent_within_root(managed_root, parent)
}

fn write_skill(
    destination: &Path,
    checksum_path: &Path,
    checksum: &str,
    managed_root: &Path,
) -> Result<(), String> {
    validate_write_target(managed_root, destination)?;
    validate_write_target(managed_root, checksum_path)?;
    fs::write(destination, DISCOVERY_SKILL)
        .map_err(|error| format!("cannot write {}: {error}", destination.display()))?;
    validate_write_target(managed_root, checksum_path)?;
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

fn back_up_skill(destination: &Path, managed_root: &Path) -> Result<PathBuf, String> {
    let backup = backup_path(destination);
    validate_write_target(managed_root, destination)?;
    validate_write_target(managed_root, &backup)?;
    fs::copy(destination, &backup).map_err(|error| {
        format!(
            "cannot back up {} to {}: {error}",
            destination.display(),
            backup.display()
        )
    })?;
    Ok(backup)
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn fail(code: &str, message: &str, json: bool) -> ! {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "error": code,
                "message": message,
            })
        );
    } else {
        eprintln!("{} {message}", "✗".red().bold());
    }
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

    fn managed_root(path: &Path) -> &Path {
        path.parent().unwrap().parent().unwrap()
    }

    #[test]
    fn embedded_skills_have_required_frontmatter() {
        for (name, content) in [("core", CORE_SKILL), ("discovery", DISCOVERY_SKILL)] {
            let content = content.replace("\r\n", "\n");
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
    fn destructive_core_skill_commands_use_json() {
        for (index, line) in CORE_SKILL.lines().enumerate() {
            if line.contains("vaxis ") && line.contains(" delete ") {
                assert!(
                    line.contains("--json"),
                    "destructive command on core skill line {} must use --json: {}",
                    index + 1,
                    line
                );
            }
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

        let first = install_one(&path, managed_root(&path), false, false).unwrap();
        assert_eq!(first.0, "installed");
        assert_eq!(fs::read_to_string(&path).unwrap(), DISCOVERY_SKILL);
        assert_eq!(
            fs::read_to_string(parent.join(CHECKSUM_FILE)).unwrap(),
            sha256(DISCOVERY_SKILL.as_bytes())
        );

        let second = install_one(&path, managed_root(&path), false, false).unwrap();
        assert_eq!(second.0, "unchanged");
        fs::remove_dir_all(parent.parent().unwrap()).unwrap();
    }

    #[test]
    fn modified_skill_requires_force_and_is_backed_up() {
        let path = temporary_skill_path("force");
        let parent = path.parent().unwrap();
        install_one(&path, managed_root(&path), false, false).unwrap();
        fs::write(&path, "user changes").unwrap();

        assert!(install_one(&path, managed_root(&path), false, false).is_err());
        let replaced = install_one(&path, managed_root(&path), true, false).unwrap();
        assert_eq!(replaced.0, "replaced");
        let backup = replaced.1.expect("forced replacement must create a backup");
        assert_eq!(fs::read_to_string(backup).unwrap(), "user changes");
        assert_eq!(fs::read_to_string(&path).unwrap(), DISCOVERY_SKILL);
        fs::remove_dir_all(parent.parent().unwrap()).unwrap();
    }

    #[test]
    fn managed_skill_is_upgraded_when_stored_checksum_matches() {
        let path = temporary_skill_path("managed-upgrade");
        let parent = path.parent().unwrap();
        fs::create_dir_all(parent).unwrap();
        let old_skill = "previous Vaxis discovery skill";
        fs::write(&path, old_skill).unwrap();
        fs::write(parent.join(CHECKSUM_FILE), sha256(old_skill.as_bytes())).unwrap();

        let upgraded = install_one(&path, managed_root(&path), false, false).unwrap();
        assert_eq!(upgraded.0, "upgraded");
        assert!(upgraded.1.is_none());
        assert_eq!(fs::read_to_string(&path).unwrap(), DISCOVERY_SKILL);
        fs::remove_dir_all(parent.parent().unwrap()).unwrap();
    }

    #[test]
    fn official_legacy_skill_is_backed_up_and_migrated_without_force() {
        let path = temporary_skill_path("legacy-migration");
        let parent = path.parent().unwrap();
        fs::create_dir_all(parent).unwrap();
        let legacy_skill = "official legacy Vaxis skill";
        fs::write(&path, legacy_skill).unwrap();
        let legacy_checksum = sha256(legacy_skill.as_bytes());

        let migrated =
            install_one_with_legacy_checksums(
                &path,
                managed_root(&path),
                false,
                false,
                &[&legacy_checksum],
            )
            .unwrap();

        assert_eq!(migrated.0, "migrated");
        let backup = migrated.1.expect("legacy migration must create a backup");
        assert_eq!(fs::read_to_string(backup).unwrap(), legacy_skill);
        assert_eq!(fs::read_to_string(&path).unwrap(), DISCOVERY_SKILL);
        assert_eq!(
            fs::read_to_string(parent.join(CHECKSUM_FILE)).unwrap(),
            sha256(DISCOVERY_SKILL.as_bytes())
        );
        fs::remove_dir_all(parent.parent().unwrap()).unwrap();
    }

    #[cfg(unix)]
    fn create_file_symlink(target: &Path, link: &Path) -> io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(windows)]
    fn create_file_symlink(target: &Path, link: &Path) -> io::Result<()> {
        std::os::windows::fs::symlink_file(target, link)
    }

    #[cfg(unix)]
    fn create_dir_symlink(target: &Path, link: &Path) -> io::Result<()> {
        std::os::unix::fs::symlink(target, link)
    }

    #[cfg(windows)]
    fn create_dir_symlink(target: &Path, link: &Path) -> io::Result<()> {
        std::os::windows::fs::symlink_dir(target, link)
    }

    #[cfg(unix)]
    fn remove_dir_symlink(link: &Path) -> io::Result<()> {
        fs::remove_file(link)
    }

    #[cfg(windows)]
    fn remove_dir_symlink(link: &Path) -> io::Result<()> {
        fs::remove_dir(link)
    }

    #[cfg(any(unix, windows))]
    fn symlink_created(result: io::Result<()>) -> bool {
        match result {
            Ok(()) => true,
            #[cfg(windows)]
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => false,
            Err(error) => panic!("could not create test symlink: {error}"),
        }
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn symlinked_destination_is_rejected_without_touching_target() {
        let path = temporary_skill_path("destination-symlink");
        let root = managed_root(&path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let external = root.with_file_name(format!(
            "vaxis-skills-external-{}",
            std::process::id()
        ));
        fs::write(&external, "external content").unwrap();
        if !symlink_created(create_file_symlink(&external, &path)) {
            fs::remove_file(&external).unwrap();
            fs::remove_dir_all(root).unwrap();
            return;
        }

        let error = install_one(&path, root, true, false).unwrap_err();
        assert!(error.contains("symlink"));
        assert_eq!(fs::read_to_string(&external).unwrap(), "external content");

        fs::remove_file(&path).unwrap();
        fs::remove_file(&external).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn symlinked_parent_is_rejected_without_writing_outside_root() {
        let path = temporary_skill_path("parent-symlink");
        let root = managed_root(&path);
        fs::create_dir_all(root).unwrap();
        let external = root.with_file_name(format!(
            "vaxis-skills-external-dir-{}",
            std::process::id()
        ));
        fs::create_dir_all(&external).unwrap();
        if !symlink_created(create_dir_symlink(&external, path.parent().unwrap())) {
            fs::remove_dir_all(&external).unwrap();
            fs::remove_dir_all(root).unwrap();
            return;
        }

        let error = install_one(&path, root, false, false).unwrap_err();
        assert!(error.contains("symlink"));
        assert!(!external.join("SKILL.md").exists());

        remove_dir_symlink(path.parent().unwrap()).unwrap();
        fs::remove_dir_all(&external).unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn duplicate_agent_skills_destinations_are_suppressed() {
        let project_targets = resolved_targets(
            &[SkillAgent::Agents, SkillAgent::Codex],
            InstallScope::Project,
            false,
        );
        assert_eq!(project_targets.len(), 1);
        assert!(project_targets[0]
            .1
            .ends_with(".agents/skills/vaxis/SKILL.md"));

        let global_targets = resolved_targets(
            &[SkillAgent::Agents, SkillAgent::Codex],
            InstallScope::Global,
            false,
        );
        assert_eq!(global_targets.len(), 1);
        assert!(global_targets[0]
            .1
            .ends_with(".agents/skills/vaxis/SKILL.md"));
    }

    #[test]
    fn project_and_global_scopes_use_canonical_paths() {
        let codex = agent_spec(SkillAgent::Codex);
        assert!(install_path(codex, InstallScope::Project, false)
            .ends_with(".agents/skills/vaxis/SKILL.md"));
        assert!(install_path(codex, InstallScope::Global, false)
            .ends_with(".agents/skills/vaxis/SKILL.md"));
    }
}
