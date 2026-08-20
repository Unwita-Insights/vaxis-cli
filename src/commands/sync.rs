use crate::cli::SyncAction;
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions as CapOpenOptions};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
#[cfg(test)]
use std::fs::OpenOptions;
use std::io::{ErrorKind, Read, Write};
use std::path::{Component, Path, PathBuf};

const MANIFEST_FILE: &str = "vaxis.yaml";
const ARCHITECTURE_FILE: &str = "architecture.vaxis.mmd";
const SCHEMA_VERSION: u32 = 2;
const MAX_PORTABLE_MERMAID_BYTES: usize = 512_000;
const MAX_PORTABLE_DIAGRAMS: usize = 256;
const MAX_EXPORT_RESPONSE_BYTES: usize = 2_000_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Manifest {
    schema_version: u32,
    root_diagram_id: String,
    file: String,
    synced_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    remote_revision: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct PortableExport {
    root_id: String,
    mermaid: String,
    diagram_count: usize,
    #[serde(default)]
    revision: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum SyncState { InSync, LocalChanged, RemoteChanged, Conflict, LocalMissing, RemoteMissing, InvalidLocal }

impl SyncState {
    fn as_str(self) -> &'static str {
        match self {
            Self::InSync => "in_sync", Self::LocalChanged => "local_changed",
            Self::RemoteChanged => "remote_changed", Self::Conflict => "conflict",
            Self::LocalMissing => "local_missing", Self::RemoteMissing => "remote_missing",
            Self::InvalidLocal => "invalid_local",
        }
    }
}

#[derive(Debug, Serialize)]
struct StatusItem { file: String, state: SyncState }

pub async fn run(token: &str, action: SyncAction, json: bool) {
    let result = match action {
        SyncAction::Init { root_diagram_id, dir } => init(token, &root_diagram_id, &dir, json).await,
        SyncAction::Status { dir, check } => status(token, &dir, check, json).await.map(|_| ()),
        SyncAction::Pull { dir, dry_run } => pull(token, &dir, dry_run, json).await,
    };
    if let Err((code, message)) = result { fail(code, &message, json); }
}

async fn init(token: &str, root_id: &str, dir: &Path, json: bool) -> Result<(), (&'static str, String)> {
    ensure_git_repository()?;
    validate_sync_dir(dir)?;
    let manifest_path = dir.join(MANIFEST_FILE);
    let architecture_path = dir.join(ARCHITECTURE_FILE);

    let remote = fetch_export(token, root_id).await?;
    if let Err((_, message)) = validate_remote_root(root_id, &remote) {
        return Err(("root_diagram_required", message));
    }
    validate_export_consistency(&remote)?;
    let source = normalized_mermaid(&remote.mermaid);
    let manifest = Manifest {
        schema_version: SCHEMA_VERSION,
        root_diagram_id: remote.root_id.clone(),
        file: ARCHITECTURE_FILE.to_string(),
        synced_hash: content_hash(&source),
        remote_revision: remote.revision,
    };
    let yaml = serde_yaml::to_string(&manifest).map_err(|e| ("manifest_invalid", e.to_string()))?;
    // Re-check after the network request so a swapped directory symlink cannot
    // redirect the delayed write outside the repository.
    let anchor = open_sync_anchor(dir, true)?;
    refuse_existing_targets_anchored(&anchor, &[Path::new(MANIFEST_FILE), Path::new(ARCHITECTURE_FILE)])?;
    atomic_write_all_anchored(
        &anchor,
        &[(PathBuf::from(ARCHITECTURE_FILE), source), (PathBuf::from(MANIFEST_FILE), yaml)],
        &[(PathBuf::from(ARCHITECTURE_FILE), None), (PathBuf::from(MANIFEST_FILE), None)],
    )?;

    if json {
        println!("{}", serde_json::json!({"ok":true,"root_diagram_id":remote.root_id,"diagram_count":remote.diagram_count,"manifest":manifest_path,"file":architecture_path}));
    } else {
        println!("{} Versioned {} diagram(s) in {}", "✓".green(), remote.diagram_count, architecture_path.display());
        println!("{} Review new files with {}, then stage and inspect them with {}.",
            "→".dimmed(),
            format!("git status --short -- {}", dir.display()).yellow(),
            format!("git diff --cached -- {}", dir.display()).yellow(),
        );
    }
    Ok(())
}

async fn status(token: &str, dir: &Path, check: bool, json: bool) -> Result<StatusItem, (&'static str, String)> {
    let (_, manifest, _) = load_manifest(dir)?;
    let file = safe_join(dir, Path::new(&manifest.file))?;
    let remote = match fetch_export(token, &manifest.root_diagram_id).await {
        Err(("remote_diagram_missing", _)) => {
            let item = StatusItem { file: file.display().to_string(), state: SyncState::RemoteMissing };
            if json { println!("{}", serde_json::json!({"ok":true,"file":item.file,"state":item.state})); }
            else { println!("{}  {}", item.state.as_str(), item.file); }
            if check { std::process::exit(2); }
            return Ok(item);
        }
        result => result?,
    };
    validate_remote_root(&manifest.root_diagram_id, &remote)?;
    validate_export_consistency(&remote)?;
    let state = match read_local_bounded(&file) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => SyncState::LocalMissing,
        Err(_) => SyncState::InvalidLocal,
        Ok(local) => match validate_portable_mermaid(&local) {
            Ok(_) => classify(&manifest.synced_hash, &local, &remote.mermaid),
            Err(_) => SyncState::InvalidLocal,
        },
    };
    let item = StatusItem { file: file.display().to_string(), state };
    if json { println!("{}", serde_json::json!({"ok":true,"file":item.file,"state":state})); }
    else { println!("{}  {}", state.as_str(), item.file); }
    if check && state != SyncState::InSync { std::process::exit(2); }
    Ok(item)
}

async fn pull(token: &str, dir: &Path, dry_run: bool, json: bool) -> Result<(), (&'static str, String)> {
    let anchor = open_sync_anchor(dir, false)?;
    let (_, mut manifest, manifest_source) = load_manifest_anchored(&anchor, dir)?;
    let file_relative = validate_relative_file(Path::new(&manifest.file))?;
    let file = dir.join(&file_relative);
    let remote = fetch_export(token, &manifest.root_diagram_id).await?;
    validate_remote_root(&manifest.root_diagram_id, &remote)?;
    validate_export_consistency(&remote)?;
    let (local, local_source) = match read_anchored_bounded(&anchor, &file_relative) {
        Ok(bytes) => {
            let value = String::from_utf8(bytes.clone())
                .map_err(|error| ("invalid_local", format!("cannot read {}: {error}", file.display())))?;
            validate_portable_mermaid(&value)
                .map_err(|(_, message)| ("invalid_local", message))?;
            (Some(value), Some(bytes))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => (None, None),
        Err(error) => return Err(("invalid_local", format!("cannot read {}: {error}", file.display()))),
    };
    let state = local.as_deref().map_or(SyncState::LocalMissing, |value| classify(&manifest.synced_hash, value, &remote.mermaid));
    if state == SyncState::Conflict {
        return Err(("sync_conflict", "local architecture changes would be overwritten; commit, restore, or reconcile them first".to_string()));
    }
    let changed = matches!(state, SyncState::RemoteChanged | SyncState::LocalMissing);
    if changed && !dry_run {
        let source = normalized_mermaid(&remote.mermaid);
        manifest.synced_hash = content_hash(&source);
        manifest.remote_revision = remote.revision;
        let yaml = serde_yaml::to_string(&manifest).map_err(|e| ("manifest_invalid", e.to_string()))?;
        atomic_write_all_anchored(
            &anchor,
            &[(file_relative.clone(), source), (PathBuf::from(MANIFEST_FILE), yaml)],
            &[(file_relative, local_source), (PathBuf::from(MANIFEST_FILE), Some(manifest_source))],
        )?;
    }
    if json { println!("{}", serde_json::json!({"ok":true,"dry_run":dry_run,"updated":changed,"file":file})); }
    else if changed { println!("{} {} {}", "✓".green(), if dry_run { "Would update" } else { "Updated" }, file.display()); }
    else if state == SyncState::LocalChanged { println!("{} Remote is unchanged; local architecture was left untouched.", "✓".green()); }
    else { println!("{} Architecture is already in sync.", "✓".green()); }
    Ok(())
}

async fn fetch_export(token: &str, root_id: &str) -> Result<PortableExport, (&'static str, String)> {
    let mut response = reqwest::Client::new()
        .get(format!("{}/api/diagrams/{root_id}/export/mermaid", crate::config::base_url()))
        .bearer_auth(token).send().await.map_err(|e| ("network_error", e.to_string()))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND { return Err(("remote_diagram_missing", format!("diagram {root_id} was not found"))); }
    if response.status() == reqwest::StatusCode::UNAUTHORIZED { return Err(("session_expired", "session expired; run vaxis login again".to_string())); }
    if !response.status().is_success() { return Err(("remote_error", format!("portable export request returned {}", response.status()))); }
    if response.content_length().is_some_and(|length| length > MAX_EXPORT_RESPONSE_BYTES as u64) {
        return Err(("remote_export_too_large", format!("portable export response exceeds {MAX_EXPORT_RESPONSE_BYTES} bytes")));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|error| ("network_error", error.to_string()))? {
        if body.len().saturating_add(chunk.len()) > MAX_EXPORT_RESPONSE_BYTES {
            return Err(("remote_export_too_large", format!("portable export response exceeds {MAX_EXPORT_RESPONSE_BYTES} bytes")));
        }
        body.extend_from_slice(&chunk);
    }
    let export: PortableExport = serde_json::from_slice(&body).map_err(|error| ("parse_error", error.to_string()))?;
    validate_export_limits(&export)?;
    Ok(export)
}

fn validate_export_consistency(export: &PortableExport) -> Result<(), (&'static str, String)> {
    validate_export_limits(export)?;
    let encoded_count = validate_portable_mermaid(&export.mermaid)?;
    if encoded_count != export.diagram_count {
        return Err(("remote_export_invalid", format!(
            "portable export declares {} diagram(s) but encodes {encoded_count}", export.diagram_count,
        )));
    }
    Ok(())
}

fn validate_export_limits(export: &PortableExport) -> Result<(), (&'static str, String)> {
    if export.diagram_count == 0 || export.diagram_count > MAX_PORTABLE_DIAGRAMS {
        return Err(("remote_export_invalid", format!("portable export diagram count must be between 1 and {MAX_PORTABLE_DIAGRAMS}")));
    }
    if export.mermaid.len() > MAX_PORTABLE_MERMAID_BYTES {
        return Err(("remote_export_too_large", format!("portable Mermaid exceeds {MAX_PORTABLE_MERMAID_BYTES} bytes")));
    }
    Ok(())
}

fn validate_portable_mermaid(value: &str) -> Result<usize, (&'static str, String)> {
    let trimmed = value.trim();
    if trimmed.is_empty() { return Err(("mermaid_unavailable", "portable Mermaid export is empty".to_string())); }
    let lines: Vec<&str> = trimmed.lines().collect();
    let first_drill = lines.iter().position(|line| line.starts_with("%% vaxis:drill ")).unwrap_or(lines.len());
    let root_level = lines[..first_drill].join("\n");
    validate_diagram_level(&root_level, first_drill < lines.len())?;
    let mut levels = HashMap::from([(String::new(), root_level)]);
    let mut diagram_count = 1usize;

    let mut index = first_drill;
    while index < lines.len() {
        let marker = lines[index].strip_prefix("%% vaxis:drill ")
            .filter(|node_id| !node_id.is_empty() && node_id.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'));
        let Some(marker) = marker else {
            return Err(("mermaid_not_renderable", format!("unexpected portable content at line {}", index + 1)));
        };
        index += 1;
        let mut payload = Vec::new();
        while index < lines.len() && lines[index].starts_with("%% vaxis:drill-line") {
            payload.push(lines[index].strip_prefix("%% vaxis:drill-line").unwrap().strip_prefix(' ').unwrap_or(lines[index].strip_prefix("%% vaxis:drill-line").unwrap()));
            index += 1;
        }
        if payload.is_empty() {
            return Err(("mermaid_not_renderable", "drill marker has no encoded payload".to_string()));
        }
        let payload = payload.join("\n");
        validate_diagram_level(&payload, false)?;
        let explicit_path = payload.lines().find_map(|line| line.trim().strip_prefix("%% vaxis:path "));
        let parent = if let Some(path) = explicit_path {
            let (parent_path, final_node) = path.rsplit_once('/').unwrap_or(("", path));
            if final_node != marker {
                return Err(("mermaid_not_renderable", format!("drill path {path} does not end with node {marker}")));
            }
            levels.get(parent_path).ok_or_else(|| (
                "mermaid_not_renderable",
                format!("drill path {path} references a parent level that does not exist"),
            ))?
        } else {
            levels.values().find(|level| portable_level_contains_node(level, marker))
                .ok_or_else(|| ("mermaid_not_renderable", format!("drill marker references missing parent node: {marker}")))?
        };
        validate_diagram_level(parent, true)?;
        if !portable_level_contains_node(parent, marker) {
            return Err(("mermaid_not_renderable", format!("drill marker references missing parent node: {marker}")));
        }
        if let Some(path) = explicit_path { levels.insert(path.to_string(), payload); }
        else { levels.insert(format!("legacy-{index}-{marker}"), payload); }
        diagram_count += 1;
        while index < lines.len() && lines[index].trim().is_empty() { index += 1; }
    }
    Ok(diagram_count)
}

fn portable_level_contains_node(level: &str, node_id: &str) -> bool {
    let diagram_only = level.lines()
        .filter(|line| !line.trim_start().starts_with("%%"))
        .collect::<Vec<_>>()
        .join("\n");
    crate::mermaid_lint::contains_node_reference(&diagram_only, node_id)
}

fn validate_diagram_level(value: &str, require_flowchart: bool) -> Result<(), (&'static str, String)> {
    let first = value.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("%%"))
        .unwrap_or("");
    let is_flowchart = first.starts_with("flowchart ") || first.starts_with("graph ");
    let is_supported = is_flowchart || ["sequenceDiagram", "classDiagram", "erDiagram", "stateDiagram-v2"]
        .iter().any(|header| first == *header || first.starts_with(&format!("{header} ")));
    if require_flowchart && !is_flowchart {
        return Err(("mermaid_not_renderable", "portable drill trees require a Mermaid flowchart header".to_string()));
    }
    if !is_supported {
        return Err(("mermaid_not_renderable", "diagram level has no supported Mermaid header".to_string()));
    }
    let report = crate::mermaid_lint::lint(value);
    if let Some(issue) = report.fixed().next() {
        return Err(("mermaid_not_renderable", format!("portable Mermaid requires repair: {}", issue.message)));
    }
    if let Some(issue) = report.errors().next() {
        let location = issue.line.map(|line| format!(" at line {line}")).unwrap_or_default();
        return Err(("mermaid_not_renderable", format!("{}{location}", issue.message)));
    }
    Ok(())
}

fn validate_remote_root(expected: &str, remote: &PortableExport) -> Result<(), (&'static str, String)> {
    if remote.root_id == expected {
        Ok(())
    } else {
        Err(("remote_root_changed", format!(
            "manifest root {expected} resolved to {}; reinitialize sync with the actual root",
            remote.root_id,
        )))
    }
}

fn load_manifest(dir: &Path) -> Result<(PathBuf, Manifest, Vec<u8>), (&'static str, String)> {
    validate_sync_dir(dir)?;
    validate_sync_root_containment(dir)?;
    let path = dir.join(MANIFEST_FILE);
    let source = fs::read(&path).map_err(|e| if e.kind() == std::io::ErrorKind::NotFound { ("manifest_not_found", format!("{} was not found", path.display())) } else { ("manifest_invalid", e.to_string()) })?;
    let text = String::from_utf8(source.clone()).map_err(|e| ("manifest_invalid", e.to_string()))?;
    let manifest: Manifest = serde_yaml::from_str(&text).map_err(|e| ("manifest_invalid", e.to_string()))?;
    if manifest.schema_version != SCHEMA_VERSION || manifest.root_diagram_id.trim().is_empty() || manifest.synced_hash.trim().is_empty() {
        return Err(("manifest_invalid", "manifest has unsupported schema or empty required fields".to_string()));
    }
    safe_join(dir, Path::new(&manifest.file))?;
    Ok((path, manifest, source))
}

fn load_manifest_anchored(anchor: &Dir, dir: &Path) -> Result<(PathBuf, Manifest, Vec<u8>), (&'static str, String)> {
    let source = read_anchored(anchor, Path::new(MANIFEST_FILE)).map_err(|error| {
        if error.kind() == ErrorKind::NotFound {
            ("manifest_not_found", format!("{} was not found", dir.join(MANIFEST_FILE).display()))
        } else {
            ("manifest_invalid", error.to_string())
        }
    })?;
    let text = String::from_utf8(source.clone()).map_err(|error| ("manifest_invalid", error.to_string()))?;
    let manifest: Manifest = serde_yaml::from_str(&text).map_err(|error| ("manifest_invalid", error.to_string()))?;
    if manifest.schema_version != SCHEMA_VERSION || manifest.root_diagram_id.trim().is_empty() || manifest.synced_hash.trim().is_empty() {
        return Err(("manifest_invalid", "manifest has unsupported schema or empty required fields".to_string()));
    }
    validate_relative_file(Path::new(&manifest.file))?;
    Ok((dir.join(MANIFEST_FILE), manifest, source))
}

fn validate_relative_file(path: &Path) -> Result<PathBuf, (&'static str, String)> {
    if path.as_os_str().is_empty() || path.is_absolute() || path.components().any(|component| {
        matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_))
    }) {
        return Err(("manifest_invalid", format!("unsafe repository path {}", path.display())));
    }
    Ok(path.to_path_buf())
}

fn repository_root() -> Result<PathBuf, (&'static str, String)> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|error| ("git_unavailable", error.to_string()))?;
    if !output.status.success() {
        return Err(("repository_not_found", "sync must run inside a Git repository".to_string()));
    }
    PathBuf::from(String::from_utf8_lossy(&output.stdout).trim()).canonicalize()
        .map_err(|error| ("repository_not_found", error.to_string()))
}

fn open_sync_anchor(dir: &Path, create: bool) -> Result<Dir, (&'static str, String)> {
    validate_sync_dir(dir)?;
    let repository = repository_root()?;
    let cwd = std::env::current_dir().map_err(|error| ("invalid_sync_directory", error.to_string()))?
        .canonicalize().map_err(|error| ("invalid_sync_directory", error.to_string()))?;
    let cwd_relative = cwd.strip_prefix(&repository)
        .map_err(|_| ("invalid_sync_directory", "current directory is outside the Git repository".to_string()))?;
    let relative = cwd_relative.join(dir);
    let repository_dir = Dir::open_ambient_dir(&repository, ambient_authority())
        .map_err(|error| ("invalid_sync_directory", error.to_string()))?;
    if create {
        repository_dir.create_dir_all(&relative)
            .map_err(|error| ("write_failed", error.to_string()))?;
    }
    repository_dir.open_dir(&relative)
        .map_err(|error| ("invalid_sync_directory", format!("cannot open sync directory {}: {error}", dir.display())))
}

fn read_anchored(anchor: &Dir, path: &Path) -> std::io::Result<Vec<u8>> {
    let mut file = anchor.open(path)?;
    let mut content = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut content)?;
    Ok(content)
}

fn read_local_bounded(path: &Path) -> std::io::Result<String> {
    let file = fs::File::open(path)?;
    let bytes = read_bounded(file, MAX_PORTABLE_MERMAID_BYTES)?;
    String::from_utf8(bytes).map_err(|error| std::io::Error::new(ErrorKind::InvalidData, error))
}

fn read_anchored_bounded(anchor: &Dir, path: &Path) -> std::io::Result<Vec<u8>> {
    read_bounded(anchor.open(path)?, MAX_PORTABLE_MERMAID_BYTES)
}

fn read_bounded(reader: impl Read, limit: usize) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(std::io::Error::new(ErrorKind::InvalidData, format!("portable Mermaid exceeds {limit} bytes")));
    }
    Ok(bytes)
}

fn refuse_existing_targets_anchored(anchor: &Dir, paths: &[&Path]) -> Result<(), (&'static str, String)> {
    for path in paths {
        match anchor.symlink_metadata(path) {
            Ok(_) => return Err(("target_exists", format!("{} already exists", path.display()))),
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(("target_check_failed", format!("cannot inspect {}: {error}", path.display()))),
        }
    }
    Ok(())
}

fn validate_sync_dir(dir: &Path) -> Result<(), (&'static str, String)> {
    if dir.as_os_str().is_empty() || dir.is_absolute() || dir.components().any(|c| matches!(c, Component::ParentDir | Component::RootDir | Component::Prefix(_))) {
        return Err(("invalid_sync_directory", "sync directory must be a relative path inside the repository".to_string()));
    }
    Ok(())
}

fn safe_join(root: &Path, relative: &Path) -> Result<PathBuf, (&'static str, String)> {
    if relative.is_absolute() || relative.components().any(|c| matches!(c, Component::ParentDir | Component::RootDir | Component::Prefix(_))) {
        return Err(("manifest_invalid", format!("unsafe repository path {}", relative.display())));
    }
    let root_canonical = root.canonicalize()
        .map_err(|e| ("manifest_invalid", format!("cannot resolve sync root {}: {e}", root.display())))?;
    let candidate = root.join(relative);
    let mut existing = candidate.as_path();
    while !existing.exists() {
        existing = existing.parent().ok_or_else(|| ("manifest_invalid", format!("unsafe repository path {}", relative.display())))?;
    }
    let existing_canonical = existing.canonicalize()
        .map_err(|e| ("manifest_invalid", format!("cannot resolve {}: {e}", existing.display())))?;
    if !existing_canonical.starts_with(&root_canonical) {
        return Err(("manifest_invalid", format!("repository path escapes sync directory through a symlink: {}", relative.display())));
    }
    Ok(candidate)
}

fn ensure_git_repository() -> Result<(), (&'static str, String)> {
    let status = std::process::Command::new("git").args(["rev-parse", "--is-inside-work-tree"]).output().map_err(|e| ("git_unavailable", e.to_string()))?;
    if status.status.success() && String::from_utf8_lossy(&status.stdout).trim() == "true" { Ok(()) }
    else { Err(("repository_not_found", "sync init must run inside a Git repository".to_string())) }
}

fn validate_sync_root_containment(dir: &Path) -> Result<(), (&'static str, String)> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| ("git_unavailable", e.to_string()))?;
    if !output.status.success() {
        return Err(("repository_not_found", "sync must run inside a Git repository".to_string()));
    }
    let repository = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    let repository = repository.canonicalize()
        .map_err(|e| ("repository_not_found", e.to_string()))?;
    let candidate = std::env::current_dir().map_err(|e| ("invalid_sync_directory", e.to_string()))?.join(dir);
    let mut existing = candidate.as_path();
    while !existing.exists() {
        existing = existing.parent().ok_or_else(|| ("invalid_sync_directory", "cannot resolve sync directory".to_string()))?;
    }
    let resolved = existing.canonicalize()
        .map_err(|e| ("invalid_sync_directory", e.to_string()))?;
    if !resolved.starts_with(&repository) {
        return Err(("invalid_sync_directory", format!("sync directory escapes the Git repository: {}", dir.display())));
    }
    Ok(())
}

fn normalized_mermaid(value: &str) -> String { format!("{}\n", value.replace("\r\n", "\n").trim_end()) }
fn content_hash(value: &str) -> String { format!("{:x}", Sha256::digest(normalized_mermaid(value).as_bytes())) }
fn classify(base: &str, local: &str, remote: &str) -> SyncState {
    let local_changed = content_hash(local) != base;
    let remote_changed = content_hash(remote) != base;
    match (local_changed, remote_changed) { (false,false)=>SyncState::InSync, (true,false)=>SyncState::LocalChanged, (false,true)=>SyncState::RemoteChanged, (true,true) if content_hash(local)==content_hash(remote)=>SyncState::RemoteChanged, (true,true)=>SyncState::Conflict }
}

fn atomic_write_all_anchored(
    anchor: &Dir,
    writes: &[(PathBuf, String)],
    expected: &[(PathBuf, Option<Vec<u8>>)],
) -> Result<(), (&'static str, String)> {
    let mut staged: Vec<(PathBuf, PathBuf, Option<PathBuf>)> = Vec::new();
    for (index, (path, content)) in writes.iter().enumerate() {
        if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
            anchor.create_dir_all(parent).map_err(|error| ("write_failed", error.to_string()))?;
        }
        let mut temporary = None;
        for attempt in 0..1000u32 {
            let candidate = path.with_extension(format!("vaxis-tmp-{}-{index}-{attempt}", std::process::id()));
            let mut options = CapOpenOptions::new();
            options.write(true).create_new(true);
            match anchor.open_with(&candidate, &options) {
                Ok(mut file) => {
                    file.write_all(content.as_bytes()).and_then(|_| file.sync_all())
                        .map_err(|error| ("write_failed", error.to_string()))?;
                    temporary = Some(candidate);
                    break;
                }
                Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(("write_failed", error.to_string())),
            }
        }
        let temporary = temporary.ok_or_else(|| ("write_failed", format!("cannot stage {}", path.display())))?;
        let backup = match anchor.symlink_metadata(path) {
            Ok(_) => Some(path.with_extension(format!("vaxis-bak-{}-{index}", std::process::id()))),
            Err(error) if error.kind() == ErrorKind::NotFound => None,
            Err(error) => return Err(("write_failed", error.to_string())),
        };
        staged.push((temporary, path.clone(), backup));
    }

    let mut installed = 0usize;
    for (temporary, path, backup) in &staged {
        let expected_content = expected.iter().find(|(expected_path, _)| expected_path == path)
            .map(|(_, content)| content)
            .ok_or_else(|| ("write_failed", format!("missing expected snapshot for {}", path.display())))?;
        let result = (|| -> std::io::Result<()> {
            if let Some(backup) = backup {
                anchor.rename(path, anchor, backup)?;
                if expected_content.as_ref() != Some(&read_anchored(anchor, backup)?) {
                    return Err(std::io::Error::new(ErrorKind::WouldBlock, "destination changed during write"));
                }
            } else {
                match anchor.symlink_metadata(path) {
                    Ok(_) => return Err(std::io::Error::new(ErrorKind::WouldBlock, "destination appeared during write")),
                    Err(error) if error.kind() == ErrorKind::NotFound && expected_content.is_none() => {}
                    Err(error) => return Err(error),
                }
            }
            anchor.rename(temporary, anchor, path)?;
            Ok(())
        })();
        if let Err(error) = result {
            let mut restoration_failures = Vec::new();
            if let Some(backup) = backup {
                if anchor.symlink_metadata(path).is_ok() { let _ = anchor.remove_file(path); }
                if anchor.rename(backup, anchor, path).is_err() {
                    restoration_failures.push(path.display().to_string());
                }
            }
            for (_, applied, applied_backup) in staged[..installed].iter().rev() {
                let installed_bytes = writes.iter().find(|(write_path, _)| write_path == applied)
                    .map(|(_, content)| content.as_bytes()).unwrap_or_default();
                match read_anchored(anchor, applied) {
                    Ok(current) if current == installed_bytes => {
                        if anchor.remove_file(applied).is_err() {
                            restoration_failures.push(applied.display().to_string());
                            continue;
                        }
                    }
                    Err(read_error) if read_error.kind() == ErrorKind::NotFound => {}
                    _ => {
                        // A process edited or replaced the installed file. Keep
                        // that concurrent work and retain our backup for manual recovery.
                        restoration_failures.push(applied.display().to_string());
                        continue;
                    }
                }
                if let Some(applied_backup) = applied_backup {
                    if anchor.rename(applied_backup, anchor, applied).is_err() {
                        restoration_failures.push(applied.display().to_string());
                    }
                }
            }
            for (temporary, _, _) in &staged { let _ = anchor.remove_file(temporary); }
            if restoration_failures.is_empty() {
                let code = if error.kind() == ErrorKind::WouldBlock { "local_changed_during_write" } else { "write_failed" };
                return Err((code, format!("{error}; original files restored")));
            }
            return Err(("restoration_incomplete", format!("{error}; could not restore: {}", restoration_failures.join(", "))));
        }
        installed += 1;
    }
    for (_, _, backup) in &staged {
        if let Some(backup) = backup { let _ = anchor.remove_file(backup); }
    }
    Ok(())
}

#[cfg(test)]
fn atomic_write_all(writes: &[(PathBuf, String)]) -> Result<(), (&'static str, String)> {
    atomic_write_all_impl(writes, None, None)
}

#[cfg(test)]
fn atomic_write_all_checked(
    writes: &[(PathBuf, String)],
    expected: &[(PathBuf, Option<Vec<u8>>)],
) -> Result<(), (&'static str, String)> {
    atomic_write_all_impl(writes, None, Some(expected))
}

#[cfg(test)]
fn atomic_write_all_impl(
    writes: &[(PathBuf, String)],
    fail_before_install: Option<usize>,
    expected: Option<&[(PathBuf, Option<Vec<u8>>)]>,
) -> Result<(), (&'static str, String)> {
    for (path, _) in writes { if let Some(parent) = path.parent() { fs::create_dir_all(parent).map_err(|e| ("write_failed", e.to_string()))?; } }
    let mut staged: Vec<(PathBuf, PathBuf, Option<PathBuf>)> = Vec::new();
    for (index, (path, content)) in writes.iter().enumerate() {
        let (temporary, mut file) = match create_unique_sibling(path, "tmp", index) {
            Ok(value) => value,
            Err(error) => {
                cleanup_staged_temps(&staged);
                return Err(error);
            }
        };
        if let Err(error) = file.write_all(content.as_bytes()).and_then(|_| file.sync_all()) {
            let _ = fs::remove_file(&temporary);
            cleanup_staged_temps(&staged);
            return Err(("write_failed", error.to_string()));
        }
        let backup = if path.exists() {
            match unique_sibling_path(path, "bak", index) {
                Ok(path) => Some(path),
                Err(error) => {
                    let _ = fs::remove_file(&temporary);
                    cleanup_staged_temps(&staged);
                    return Err(error);
                }
            }
        } else { None };
        staged.push((temporary, path.clone(), backup));
    }
    if let Some(expected) = expected {
        for (path, expected_content) in expected {
            let current = match fs::read(path) {
                Ok(content) => Some(content),
                Err(error) if error.kind() == ErrorKind::NotFound => None,
                Err(error) => {
                    cleanup_staged_temps(&staged);
                    return Err(("local_changed_during_pull", format!("cannot recheck {}: {error}", path.display())));
                }
            };
            if &current != expected_content {
                cleanup_staged_temps(&staged);
                return Err((
                    "local_changed_during_pull",
                    format!("{} changed while pull was preparing; no files were replaced", path.display()),
                ));
            }
        }
    }
    let mut installed = 0usize;
    for (index, (temporary, path, backup)) in staged.iter().enumerate() {
        let expected_content = expected.and_then(|entries| {
            entries.iter().find(|(expected_path, _)| expected_path == path).map(|(_, content)| content)
        });
        let install_result = (|| -> std::io::Result<()> {
            if let Some(backup) = backup {
                fs::rename(path, backup)?;
                if let Some(expected_content) = expected_content {
                    let actual = fs::read(backup)?;
                    if expected_content.as_ref() != Some(&actual) {
                        return Err(std::io::Error::new(ErrorKind::WouldBlock, "destination changed during pull"));
                    }
                }
            } else if let Some(expected_content) = expected_content {
                match fs::symlink_metadata(path) {
                    Ok(_) => return Err(std::io::Error::new(ErrorKind::WouldBlock, "destination appeared during pull")),
                    Err(error) if error.kind() == ErrorKind::NotFound && expected_content.is_none() => {}
                    Err(error) if error.kind() == ErrorKind::NotFound => {
                        return Err(std::io::Error::new(ErrorKind::WouldBlock, "destination disappeared during pull"));
                    }
                    Err(error) => return Err(error),
                }
            }
            if fail_before_install == Some(index) {
                return Err(std::io::Error::other("simulated install failure"));
            }
            if fs::symlink_metadata(path).is_ok() {
                return Err(std::io::Error::new(ErrorKind::WouldBlock, "destination reappeared during pull"));
            }
            fs::rename(temporary, path)?;
            if let (Some(backup), Some(expected_content)) = (backup, expected_content) {
                let actual = fs::read(backup)?;
                if expected_content.as_ref() != Some(&actual) {
                    return Err(std::io::Error::new(ErrorKind::WouldBlock, "destination changed during pull"));
                }
            }
            Ok(())
        })();
        if let Err(error) = install_result {
            let mut restoration_failures = Vec::new();
            // Restore the current destination if its original was already moved.
            if let Some(backup) = backup {
                if backup.exists() {
                    if fs::symlink_metadata(path).is_ok() && fs::remove_file(path).is_err() {
                        restoration_failures.push(path.display().to_string());
                    } else if fs::rename(backup, path).is_err() {
                        restoration_failures.push(path.display().to_string());
                    }
                }
            }
            for (_, applied_path, applied_backup) in staged[..installed].iter().rev() {
                if applied_path.exists() && fs::remove_file(applied_path).is_err() {
                    restoration_failures.push(applied_path.display().to_string());
                    continue;
                }
                if let Some(applied_backup) = applied_backup {
                    if fs::rename(applied_backup, applied_path).is_err() {
                        restoration_failures.push(applied_path.display().to_string());
                    }
                }
            }
            for (temporary, _, _) in &staged { let _ = fs::remove_file(temporary); }
            if restoration_failures.is_empty() {
                let code = if error.kind() == ErrorKind::WouldBlock { "local_changed_during_pull" } else { "write_failed" };
                return Err((code, format!("{error}; original files restored")));
            }
            return Err(("restoration_incomplete", format!("{error}; could not restore: {}", restoration_failures.join(", "))));
        }
        installed += 1;
    }
    for (_, _, backup) in &staged {
        if let Some(backup) = backup { let _ = fs::remove_file(backup); }
    }
    Ok(())
}

#[cfg(test)]
fn cleanup_staged_temps(staged: &[(PathBuf, PathBuf, Option<PathBuf>)]) {
    for (temporary, _, _) in staged { let _ = fs::remove_file(temporary); }
}

#[cfg(test)]
fn create_unique_sibling(path: &Path, kind: &str, index: usize) -> Result<(PathBuf, fs::File), (&'static str, String)> {
    for attempt in 0..1000u32 {
        let candidate = path.with_extension(format!("vaxis-{kind}-{}-{index}-{attempt}", std::process::id()));
        match OpenOptions::new().write(true).create_new(true).open(&candidate) {
            Ok(file) => return Ok((candidate, file)),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(("write_failed", error.to_string())),
        }
    }
    Err(("write_failed", format!("could not allocate a unique {kind} file beside {}", path.display())))
}

#[cfg(test)]
fn unique_sibling_path(path: &Path, kind: &str, index: usize) -> Result<PathBuf, (&'static str, String)> {
    for attempt in 0..1000u32 {
        let candidate = path.with_extension(format!("vaxis-{kind}-{}-{index}-{attempt}", std::process::id()));
        if !candidate.exists() { return Ok(candidate); }
    }
    Err(("write_failed", format!("could not allocate a unique {kind} path beside {}", path.display())))
}

fn fail(code: &'static str, message: &str, json: bool) -> ! {
    if json { println!("{}", serde_json::json!({"ok":false,"error":{"code":code,"message":message}})); } else { eprintln!("{} {message}", "Error:".red().bold()); }
    std::process::exit(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    #[test] fn hashes_normalize_line_endings() { assert_eq!(content_hash("flowchart TB\r\na-->b"), content_hash("flowchart TB\na-->b\n")); }
    #[test] fn classifies_single_file_states() {
        let base = content_hash("flowchart TB\na-->b");
        assert_eq!(classify(&base, "flowchart TB\na-->b", "flowchart TB\na-->b"), SyncState::InSync);
        assert_eq!(classify(&base, "flowchart TB\na-->c", "flowchart TB\na-->b"), SyncState::LocalChanged);
        assert_eq!(classify(&base, "flowchart TB\na-->b", "flowchart TB\na-->c"), SyncState::RemoteChanged);
        assert_eq!(classify(&base, "flowchart TB\na-->d", "flowchart TB\na-->c"), SyncState::Conflict);
    }
    #[test] fn rejects_escaping_file_paths() {
        let root = tempdir().unwrap();
        assert!(safe_join(root.path(), Path::new("../x.mmd")).is_err());
    }
    #[test] fn state_names_are_stable() { assert_eq!(SyncState::InSync.as_str(), "in_sync"); }

    #[test]
    fn accepts_supported_non_flowchart_roots_and_leading_comments() {
        for mermaid in [
            "%% repository architecture\nsequenceDiagram\n  A->>B: hello",
            "classDiagram\n  Animal <|-- Dog",
            "erDiagram\n  USER ||--o{ ORDER : places",
            "stateDiagram-v2\n  [*] --> Ready",
            "%% legal comment\nflowchart TB\n  a[A]",
        ] {
            assert!(validate_portable_mermaid(mermaid).is_ok(), "rejected {mermaid}");
        }
    }

    #[test]
    fn rejects_non_flowchart_drill_trees_and_malformed_markers() {
        assert!(validate_portable_mermaid(
            "sequenceDiagram\n  A->>B: hello\n%% vaxis:drill A\n%% vaxis:drill-line flowchart TB\n%% vaxis:drill-line child[Child]",
        ).is_err());
        assert!(validate_portable_mermaid(
            "flowchart TB\n  service[Service]\n%% vaxis:drill service trailing-junk\n%% vaxis:drill-line flowchart TB\n%% vaxis:drill-line child[Child]",
        ).is_err());
        assert!(validate_portable_mermaid(
            "flowchart TB\n  service[Service]\n  %% vaxis:drill service\n  %% vaxis:drill-line flowchart TB\n  %% vaxis:drill-line child[Child]",
        ).is_err());
        assert!(validate_portable_mermaid(
            "flowchart TB\n  service[Service]\n%% vaxis:drill missing\n%% vaxis:drill-line flowchart TB\n%% vaxis:drill-line child[Child]",
        ).is_err());
    }

    #[test]
    fn rejects_remote_exports_outside_supported_limits() {
        let oversized_count = PortableExport {
            root_id: "root".into(), mermaid: "flowchart TB\n  a[A]".into(),
            diagram_count: MAX_PORTABLE_DIAGRAMS + 1, revision: None,
        };
        assert_eq!(validate_export_limits(&oversized_count).unwrap_err().0, "remote_export_invalid");
        let oversized_mermaid = PortableExport {
            root_id: "root".into(), mermaid: "x".repeat(MAX_PORTABLE_MERMAID_BYTES + 1),
            diagram_count: 1, revision: None,
        };
        assert_eq!(validate_export_limits(&oversized_mermaid).unwrap_err().0, "remote_export_too_large");
    }

    #[test]
    fn anchored_initialization_preserves_a_target_that_appeared() {
        let root = tempdir().unwrap();
        let anchor = Dir::open_ambient_dir(root.path(), ambient_authority()).unwrap();
        fs::write(root.path().join(ARCHITECTURE_FILE), "concurrent file").unwrap();
        let error = atomic_write_all_anchored(
            &anchor,
            &[(PathBuf::from(ARCHITECTURE_FILE), "generated file".into())],
            &[(PathBuf::from(ARCHITECTURE_FILE), None)],
        ).unwrap_err();
        assert_eq!(error.0, "local_changed_during_write");
        assert_eq!(fs::read_to_string(root.path().join(ARCHITECTURE_FILE)).unwrap(), "concurrent file");
    }

    #[cfg(unix)]
    #[test]
    fn anchored_writer_cannot_follow_a_substituted_sync_directory() {
        use std::os::unix::fs::symlink;
        let root = tempdir().unwrap();
        let outside = tempdir().unwrap();
        let sync = root.path().join("architecture");
        let original = root.path().join("original-architecture");
        fs::create_dir(&sync).unwrap();
        let anchor = Dir::open_ambient_dir(&sync, ambient_authority()).unwrap();
        fs::rename(&sync, &original).unwrap();
        symlink(outside.path(), &sync).unwrap();

        atomic_write_all_anchored(
            &anchor,
            &[(PathBuf::from(ARCHITECTURE_FILE), "safe".into())],
            &[(PathBuf::from(ARCHITECTURE_FILE), None)],
        ).unwrap();

        assert_eq!(fs::read_to_string(original.join(ARCHITECTURE_FILE)).unwrap(), "safe");
        assert!(!outside.path().join(ARCHITECTURE_FILE).exists());
    }

    #[test]
    fn checked_install_refuses_content_changed_after_classification() {
        let root = tempdir().unwrap();
        let destination = root.path().join("architecture.vaxis.mmd");
        fs::write(&destination, "concurrent edit").unwrap();

        let error = atomic_write_all_checked(
            &[(destination.clone(), "remote update".into())],
            &[(destination.clone(), Some(b"classified content".to_vec()))],
        ).unwrap_err();

        assert_eq!(error.0, "local_changed_during_pull");
        assert_eq!(fs::read_to_string(destination).unwrap(), "concurrent edit");
    }

    #[cfg(unix)]
    #[test]
    fn initialization_refuses_dangling_symlink_targets() {
        use std::os::unix::fs::symlink;
        let root = tempdir().unwrap();
        let target = root.path().join("architecture.vaxis.mmd");
        symlink(root.path().join("missing-target"), &target).unwrap();
        let anchor = Dir::open_ambient_dir(root.path(), ambient_authority()).unwrap();

        assert_eq!(refuse_existing_targets_anchored(&anchor, &[Path::new("architecture.vaxis.mmd")]).unwrap_err().0, "target_exists");
        assert!(fs::symlink_metadata(target).unwrap().file_type().is_symlink());
    }

    #[test]
    fn replaces_existing_files_on_windows_and_other_platforms() {
        let root = tempdir().unwrap();
        let architecture = root.path().join("architecture.vaxis.mmd");
        let manifest = root.path().join("vaxis.yaml");
        fs::write(&architecture, "old architecture").unwrap();
        fs::write(&manifest, "old manifest").unwrap();

        atomic_write_all(&[
            (architecture.clone(), "new architecture".into()),
            (manifest.clone(), "new manifest".into()),
        ]).unwrap();

        assert_eq!(fs::read_to_string(architecture).unwrap(), "new architecture");
        assert_eq!(fs::read_to_string(manifest).unwrap(), "new manifest");
    }

    #[test]
    fn restores_every_original_when_second_install_fails() {
        let root = tempdir().unwrap();
        let architecture = root.path().join("architecture.vaxis.mmd");
        let manifest = root.path().join("vaxis.yaml");
        fs::write(&architecture, "old architecture").unwrap();
        fs::write(&manifest, "old manifest").unwrap();

        let error = atomic_write_all_impl(&[
            (architecture.clone(), "new architecture".into()),
            (manifest.clone(), "new manifest".into()),
        ], Some(1), None).unwrap_err();

        assert_eq!(error.0, "write_failed");
        assert_eq!(fs::read_to_string(architecture).unwrap(), "old architecture");
        assert_eq!(fs::read_to_string(manifest).unwrap(), "old manifest");
    }

    #[test]
    fn removes_a_new_first_file_when_second_install_fails() {
        let root = tempdir().unwrap();
        let architecture = root.path().join("architecture.vaxis.mmd");
        let manifest = root.path().join("vaxis.yaml");

        let error = atomic_write_all_impl(&[
            (architecture.clone(), "new architecture".into()),
            (manifest.clone(), "new manifest".into()),
        ], Some(1), None).unwrap_err();

        assert_eq!(error.0, "write_failed");
        assert!(!architecture.exists());
        assert!(!manifest.exists());
    }

    #[test]
    fn stale_temp_and_backup_names_do_not_block_a_new_write() {
        let root = tempdir().unwrap();
        let destination = root.path().join("architecture.vaxis.mmd");
        fs::write(&destination, "old").unwrap();
        fs::write(destination.with_extension(format!("vaxis-tmp-{}-0-0", std::process::id())), "stale").unwrap();
        fs::write(destination.with_extension(format!("vaxis-bak-{}-0-0", std::process::id())), "stale").unwrap();

        atomic_write_all(&[(destination.clone(), "new".into())]).unwrap();

        assert_eq!(fs::read_to_string(destination).unwrap(), "new");
    }

    #[test]
    fn rejects_a_remote_response_for_a_different_root() {
        let remote = PortableExport {
            root_id: "other".into(),
            mermaid: "flowchart TB\n  a[A]".into(),
            diagram_count: 1,
            revision: None,
        };
        assert_eq!(validate_remote_root("expected", &remote).unwrap_err().0, "remote_root_changed");
    }

    #[test]
    fn validates_each_level_of_a_recursive_portable_tree_in_its_own_scope() {
        let portable = [
            "flowchart TB",
            "  service[Root Service]",
            "",
            "%% vaxis:drill service",
            "%% vaxis:drill-line flowchart TB",
            "%% vaxis:drill-line   nested[Child Service]",
            "",
            "%% vaxis:drill nested",
            "%% vaxis:drill-line flowchart TB",
            "%% vaxis:drill-line   db[(Database)]",
        ].join("\n");

        assert!(validate_portable_mermaid(&portable).is_ok());
    }

    #[test]
    fn accepts_supported_non_flowchart_leaf_payload() {
        let portable = [
            "flowchart TB",
            "  events[Event Flow]",
            "%% vaxis:drill events",
            "%% vaxis:drill-line sequenceDiagram",
            "%% vaxis:drill-line   Producer->>Consumer: event",
            "%% vaxis:drill-line %% vaxis:path events",
        ].join("\n");
        assert_eq!(validate_portable_mermaid(&portable).unwrap(), 2);
    }

    #[test]
    fn rejects_export_count_that_does_not_match_encoded_tree() {
        let export = PortableExport {
            root_id: "root".into(),
            mermaid: "flowchart TB\n  root[Root]".into(),
            diagram_count: 2,
            revision: None,
        };
        assert_eq!(validate_export_consistency(&export).unwrap_err().0, "remote_export_invalid");
    }

    #[test]
    fn bounded_reader_rejects_oversized_local_content() {
        let content = vec![b'x'; MAX_PORTABLE_MERMAID_BYTES + 1];
        let error = read_bounded(std::io::Cursor::new(content), MAX_PORTABLE_MERMAID_BYTES).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::InvalidData);
    }

    #[test]
    fn rejects_an_invalid_nested_portable_level() {
        let portable = [
            "flowchart TB",
            "  service[Root Service]",
            "%% vaxis:drill service",
            "%% vaxis:drill-line this is not Mermaid",
        ].join("\n");

        assert_eq!(validate_portable_mermaid(&portable).unwrap_err().0, "mermaid_not_renderable");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_parent_that_escapes_sync_root() {
        use std::os::unix::fs::symlink;
        let root = tempdir().unwrap();
        let outside = tempdir().unwrap();
        symlink(outside.path(), root.path().join("escape")).unwrap();
        assert!(safe_join(root.path(), Path::new("escape/file.mmd")).is_err());
    }
}
