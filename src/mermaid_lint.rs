//! Preflight lint for Mermaid sent via `vaxis diagrams generate --mermaid`.
//!
//! This mirrors the server's drill-block parser (`parseDrillBlocks` in
//! apps/api/src/utils/mermaid.ts + the id filter in generate_service) so drill
//! mistakes are caught deterministically, with a clear and fixable message,
//! BEFORE the network round-trip — instead of the server silently dropping the
//! drills, or the browser failing to render and leaving a blank canvas.
//!
//! The server's drill rules this file encodes:
//!   1. A marker is only recognised if it matches `^%%\s*vaxis:drill\s+<id>\s*$`
//!      — i.e. it starts at COLUMN 0 (no indentation).
//!   2. Everything BEFORE the first marker is the main diagram; everything
//!      after a marker (up to the next) is that node's child content. So markers
//!      belong AFTER the complete main diagram, never between its nodes.
//!   3. A marker's `<id>` is kept only if it is a real node in the main diagram
//!      (`\bid\b` present). Ids that aren't get dropped.
//!   4. Drills apply to flowcharts only (`graph` / `flowchart`).
//!
//! Additional structural checks mirror the server's `isRenderableMermaid` /
//! `processMermaidForType` checks and the frontend's rendering constraints, so
//! the CLI catches format errors before they cause a silent canvas rollback.

use regex::Regex;
use std::collections::HashSet;

// ── Types ──────────────────────────────────────────────────────────────────

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Level {
    Error,
    Warning,
    /// Issue was automatically repaired by `lint`; no action required.
    Fixed,
}

#[derive(Debug, Clone)]
pub struct Issue {
    pub level: Level,
    pub code: &'static str,
    pub message: String,
    /// 1-based line number the issue anchors to, when known.
    pub line: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct LintReport {
    pub issues: Vec<Issue>,
    /// The Mermaid content after all auto-repairs. May equal the original input
    /// when nothing was changed. Use this as the input to `generate --mermaid`
    /// after `vaxis diagrams lint --fix` writes it back to disk.
    pub repaired: String,
}

impl Default for LintReport {
    fn default() -> Self {
        LintReport { issues: Vec::new(), repaired: String::new() }
    }
}

impl LintReport {
    pub fn has_errors(&self) -> bool {
        self.issues.iter().any(|i| i.level == Level::Error)
    }
    pub fn errors(&self) -> impl Iterator<Item = &Issue> {
        self.issues.iter().filter(|i| i.level == Level::Error)
    }
    pub fn warnings(&self) -> impl Iterator<Item = &Issue> {
        self.issues.iter().filter(|i| i.level == Level::Warning)
    }
    pub fn fixed(&self) -> impl Iterator<Item = &Issue> {
        self.issues.iter().filter(|i| i.level == Level::Fixed)
    }
}

// ── Parsing helpers ────────────────────────────────────────────────────────

/// Parse a single line as a column-0 drill marker, returning the node id.
/// Mirrors the JS regex `^%%\s*vaxis:drill\s+([\w-]+)\s*$` (ASCII `\w`).
fn parse_marker(line: &str) -> Option<&str> {
    // `^%%` — the line must START with `%%`; leading whitespace disqualifies it.
    let rest = line.strip_prefix("%%")?;
    // `\s*`
    let rest = rest.trim_start();
    // `vaxis:drill`
    let rest = rest.strip_prefix("vaxis:drill")?;
    // `\s+` — at least one space before the id.
    if !rest.starts_with(|c: char| c.is_whitespace()) {
        return None;
    }
    let rest = rest.trim_start();
    // `([\w-]+)` — ASCII word chars plus hyphen, matching JS `\w`.
    let id_len = rest
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'-')
        .count();
    if id_len == 0 {
        return None;
    }
    let (id, tail) = rest.split_at(id_len);
    // `\s*$` — only trailing whitespace may follow.
    if tail.trim().is_empty() {
        Some(id)
    } else {
        None
    }
}

/// Does this line look like it is TRYING to be a drill marker — a `%%` comment
/// whose content begins with `vaxis:drill`? Used to flag MALFORMED markers
/// (indented, missing id, trailing junk) without false-flagging node labels or
/// prose that merely contain the literal text `vaxis:drill`.
fn looks_like_drill_marker(line: &str) -> bool {
    match line.trim_start().strip_prefix("%%") {
        Some(rest) => rest.trim_start().starts_with("vaxis:drill"),
        None => false,
    }
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// `\bword\b` presence — `word` bounded by non-word chars (or string ends).
fn word_present(haystack: &str, word: &str) -> bool {
    if word.is_empty() {
        return false;
    }
    let bytes = haystack.as_bytes();
    let mut from = 0;
    while let Some(pos) = haystack[from..].find(word) {
        let start = from + pos;
        let end = start + word.len();
        let before_ok = start == 0 || !is_word_byte(bytes[start - 1]);
        let after_ok = end >= bytes.len() || !is_word_byte(bytes[end]);
        if before_ok && after_ok {
            return true;
        }
        from = start + 1;
        if from >= haystack.len() {
            break;
        }
    }
    false
}

/// The first meaningful (non-empty, non-`%%`-comment) line, trimmed.
fn first_content_line(mermaid: &str) -> Option<&str> {
    mermaid
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("%%"))
}

fn is_flowchart(mermaid: &str) -> bool {
    match first_content_line(mermaid) {
        Some(l) => l.starts_with("graph ") || l.starts_with("graph\t") || l.starts_with("flowchart"),
        None => false,
    }
}

struct Analysis {
    /// (1-based line, node id) for every valid column-0 marker.
    markers: Vec<(usize, String)>,
    first_marker_line: Option<usize>,
    /// Everything before the first valid marker — the main diagram.
    main: String,
}

fn analyze(mermaid: &str) -> Analysis {
    let lines: Vec<&str> = mermaid.split('\n').collect();
    let mut markers = Vec::new();
    let mut first_marker_idx: Option<usize> = None;
    for (i, line) in lines.iter().enumerate() {
        if let Some(id) = parse_marker(line) {
            if first_marker_idx.is_none() {
                first_marker_idx = Some(i);
            }
            markers.push((i + 1, id.to_string()));
        }
    }
    let main = match first_marker_idx {
        Some(idx) => lines[..idx].join("\n"),
        None => mermaid.to_string(),
    };
    Analysis {
        markers,
        first_marker_line: first_marker_idx.map(|i| i + 1),
        main,
    }
}

/// The drill node ids the SERVER would keep for this Mermaid.
#[allow(dead_code)]
pub fn drill_node_ids(mermaid: &str) -> Vec<String> {
    let a = analyze(mermaid);
    if !is_flowchart(&a.main) {
        return Vec::new();
    }
    let mut out: Vec<String> = Vec::new();
    for (_, id) in &a.markers {
        if word_present(&a.main, id) && !out.iter().any(|k| k == id) {
            out.push(id.clone());
        }
    }
    out
}

// ── Auto-repair functions ──────────────────────────────────────────────────

/// Replace `flowchart TD` (and `graph TD`) with `flowchart TB` / `graph TB`.
/// TD and TB render identically but only TB is accepted by the Vaxis API.
fn repair_direction(mermaid: &str) -> (String, Vec<Issue>) {
    let re = Regex::new(r"(?i)^(flowchart|graph)\s+TD\b").unwrap();
    let mut fixed: Vec<Issue> = Vec::new();
    let lines: Vec<String> = mermaid
        .split('\n')
        .enumerate()
        .map(|(i, line)| {
            let trimmed = line.trim_start();
            if re.is_match(trimmed) {
                let keyword = re
                    .captures(trimmed)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_else(|| "flowchart".to_string());
                let new_trimmed = re.replace(trimmed, format!("{} TB", keyword)).into_owned();
                let indent_len = line.len() - line.trim_start().len();
                let indent = &line[..indent_len];
                if fixed.is_empty() {
                    fixed.push(Issue {
                        level: Level::Fixed,
                        code: "direction_td_repaired",
                        message: format!(
                            "Line {}: `{} TD` → `{} TB` (Vaxis accepts `flowchart TB` and `flowchart LR` only; TD and TB render identically).",
                            i + 1, keyword, keyword
                        ),
                        line: Some(i + 1),
                    });
                }
                format!("{}{}", indent, new_trimmed)
            } else {
                line.to_string()
            }
        })
        .collect();
    (lines.join("\n"), fixed)
}

/// Strip leading whitespace from drill markers that are only malformed due to
/// indentation — the server's column-0 rule silently drops indented markers.
fn repair_drill_indentation(mermaid: &str) -> (String, Vec<Issue>) {
    let mut fixed: Vec<Issue> = Vec::new();
    let lines: Vec<String> = mermaid
        .split('\n')
        .enumerate()
        .map(|(i, line)| {
            if looks_like_drill_marker(line) && parse_marker(line).is_none() {
                let stripped = line.trim_start().to_string();
                if parse_marker(&stripped).is_some() {
                    fixed.push(Issue {
                        level: Level::Fixed,
                        code: "drill_marker_indent_fixed",
                        message: format!(
                            "Line {}: Removed leading whitespace from drill marker (must start at column 0).",
                            i + 1
                        ),
                        line: Some(i + 1),
                    });
                    return stripped;
                }
            }
            line.to_string()
        })
        .collect();
    (lines.join("\n"), fixed)
}

/// Auto-quote unquoted balanced parentheses in pipe-delimited edge labels.
/// Mirrors the server's `quoteParenthesizedEdgeLabels` repair.
/// e.g. `A -->|Teacher (Extended)| B` → `A -->|"Teacher (Extended)"| B`
fn repair_edge_labels(mermaid: &str) -> (String, Vec<Issue>) {
    if !is_flowchart(mermaid) {
        return (mermaid.to_string(), Vec::new());
    }
    // Match pipe-delimited edge labels that don't already contain quotes.
    let re = Regex::new(r#"\|([^|"]+)\|"#).unwrap();
    let mut fixed_count = 0usize;
    let repaired = re
        .replace_all(mermaid, |caps: &regex::Captures| {
            let label = &caps[1];
            if !label.contains('(') {
                return caps[0].to_string();
            }
            let opens = label.bytes().filter(|&b| b == b'(').count();
            let closes = label.bytes().filter(|&b| b == b')').count();
            if opens > 0 && opens == closes {
                fixed_count += 1;
                format!("|\"{}\"|", label)
            } else {
                caps[0].to_string()
            }
        })
        .into_owned();

    let mut issues = Vec::new();
    if fixed_count > 0 {
        issues.push(Issue {
            level: Level::Fixed,
            code: "edge_label_paren_quoted",
            message: format!(
                "Auto-quoted {} edge label(s) with parentheses — e.g. `|Label (detail)|` → `|\"Label (detail)\"|`.",
                fixed_count
            ),
            line: None,
        });
    }
    (repaired, issues)
}

// ── New structural checks ──────────────────────────────────────────────────

/// Error if the flowchart uses an unsupported direction.
/// After `repair_direction`, only BT/RL and missing direction can remain.
fn check_direction(mermaid: &str, issues: &mut Vec<Issue>) {
    let Some(first) = first_content_line(mermaid) else { return };
    if !first.starts_with("flowchart") && !first.starts_with("graph") {
        return;
    }
    let parts: Vec<&str> = first.split_whitespace().collect();
    if parts.len() < 2 {
        issues.push(Issue {
            level: Level::Warning,
            code: "direction_missing",
            message: "Flowchart has no direction — add `flowchart TB` or `flowchart LR`.".to_string(),
            line: None,
        });
        return;
    }
    match parts[1].to_ascii_uppercase().as_str() {
        "TB" | "LR" => {}
        dir => {
            issues.push(Issue {
                level: Level::Error,
                code: "direction_unsupported",
                message: format!(
                    "Flowchart direction `{}` is not supported by Vaxis — use `flowchart TB` or `flowchart LR`.",
                    dir
                ),
                line: None,
            });
        }
    }
}

/// Error if `subgraph` blocks are present — the server strips them and nodes
/// inside may lose their connections entirely.
fn check_subgraphs(mermaid: &str, issues: &mut Vec<Issue>) {
    if !is_flowchart(mermaid) {
        return;
    }
    for (i, line) in mermaid.split('\n').enumerate() {
        if line.trim().starts_with("subgraph") {
            issues.push(Issue {
                level: Level::Error,
                code: "subgraph_present",
                message: format!(
                    "Line {}: `subgraph` blocks are not supported — the server strips them and nodes inside may lose connections. Replace with flat nodes and edges.",
                    i + 1
                ),
                line: Some(i + 1),
            });
            return; // First occurrence is enough; avoid noise.
        }
    }
}

/// Error if forbidden shape syntax is used — these are not rendered by the
/// Vaxis frontend. Allowed: `[label]` (rectangle), `[(label)]` (cylinder),
/// `{label}` (rhombus). Forbidden: hexagon `{{`, circle `((`, stadium `([`,
/// Mermaid v11 `@{shape:`.
fn check_forbidden_shapes(mermaid: &str, issues: &mut Vec<Issue>) {
    if !is_flowchart(mermaid) {
        return;
    }
    // Match: identifier optionally followed by whitespace, then the forbidden opener.
    let hexagon = Regex::new(r#"\b[\w-]+\s*\{\{"#).unwrap();
    let circle  = Regex::new(r#"\b[\w-]+\s*\(\("#).unwrap();
    let stadium = Regex::new(r#"\b[\w-]+\s*\(\["#).unwrap();
    let v11     = Regex::new(r#"\b[\w-]+@\{"#).unwrap();

    for (i, line) in mermaid.split('\n').enumerate() {
        if line.trim().starts_with("%%") {
            continue;
        }
        if hexagon.is_match(line) {
            issues.push(Issue {
                level: Level::Error,
                code: "hexagon_shape",
                message: format!(
                    "Line {}: hexagon `{{{{...}}}}` is not supported — use `[label]` for services.",
                    i + 1
                ),
                line: Some(i + 1),
            });
        }
        if circle.is_match(line) {
            issues.push(Issue {
                level: Level::Error,
                code: "circle_shape",
                message: format!(
                    "Line {}: circle `((...))` is not supported — use `[label]` for services or `{{label}}` for yes/no decisions.",
                    i + 1
                ),
                line: Some(i + 1),
            });
        }
        if stadium.is_match(line) {
            issues.push(Issue {
                level: Level::Error,
                code: "stadium_shape",
                message: format!(
                    "Line {}: stadium `([...])` is not supported — use `[label]` for services.",
                    i + 1
                ),
                line: Some(i + 1),
            });
        }
        if v11.is_match(line) {
            issues.push(Issue {
                level: Level::Error,
                code: "v11_shape_syntax",
                message: format!(
                    "Line {}: Mermaid v11 `@{{shape:...}}` syntax is not supported — use standard bracket syntax.",
                    i + 1
                ),
                line: Some(i + 1),
            });
        }
    }
}

// ── Main public API ────────────────────────────────────────────────────────

/// Run every preflight check over the Mermaid about to be sent, applying
/// auto-repairs where possible. Always returns a `LintReport` with:
///   - `repaired`: the Mermaid content after all auto-fixes (use this for generate)
///   - `issues`: errors, warnings, and Fixed records for what was auto-repaired
///
/// Callers should proceed only when `!report.has_errors()`.
pub fn lint(mermaid: &str) -> LintReport {
    let mut issues = Vec::new();

    // ── Auto-repairs (applied in order; each step sees the previous result) ──
    let (after_dir,    dir_issues)   = repair_direction(mermaid);
    let (after_drills, drill_issues) = repair_drill_indentation(&after_dir);
    let (repaired,     edge_issues)  = repair_edge_labels(&after_drills);
    issues.extend(dir_issues);
    issues.extend(drill_issues);
    issues.extend(edge_issues);

    // ── New structural checks (run on the repaired content) ──────────────────
    check_direction(&repaired, &mut issues);
    check_subgraphs(&repaired, &mut issues);
    check_forbidden_shapes(&repaired, &mut issues);

    // ── Existing drill checks (run on the repaired content) ──────────────────
    let a = analyze(&repaired);

    // E1 — malformed drill markers (after repair, any remaining ones are truly
    // malformed beyond just indentation).
    for (i, line) in repaired.split('\n').enumerate() {
        if looks_like_drill_marker(line) && parse_marker(line).is_none() {
            issues.push(Issue {
                level: Level::Error,
                code: "drill_marker_not_at_column_0",
                message: format!(
                    "Line {}: not a valid drill marker. Write it exactly as `%% vaxis:drill <nodeId>` at the start of the line (column 0 — no indentation).",
                    i + 1
                ),
                line: Some(i + 1),
            });
        }
    }

    if !a.markers.is_empty() {
        let main_has_content = first_content_line(&a.main).is_some();
        if !main_has_content {
            // E2 — markers appear before any diagram content.
            issues.push(Issue {
                level: Level::Error,
                code: "drill_marker_before_diagram",
                message:
                    "Drill markers appear before any diagram content. Put the complete main diagram FIRST, then the `%% vaxis:drill` markers after it."
                        .to_string(),
                line: a.first_marker_line,
            });
        } else if !is_flowchart(&a.main) {
            // E4 — drills on a non-flowchart.
            issues.push(Issue {
                level: Level::Error,
                code: "drill_on_non_flowchart",
                message:
                    "Drill markers only work on flowcharts (`graph` / `flowchart`). Remove them, or make the main diagram a flowchart."
                        .to_string(),
                line: a.first_marker_line,
            });
        } else {
            // E3 — a marker's id isn't a node in the main diagram.
            for (line_no, id) in &a.markers {
                if !word_present(&a.main, id) {
                    issues.push(Issue {
                        level: Level::Error,
                        code: "drill_node_not_found",
                        message: format!(
                            "Line {}: `%% vaxis:drill {}` references a node not defined in the main diagram. Define `{}` as a node above, or (if markers are between nodes) move ALL markers after the complete diagram.",
                            line_no, id, id
                        ),
                        line: Some(*line_no),
                    });
                }
            }
        }
    }

    // W3 — thin seeded drills.
    let lines: Vec<&str> = repaired.lines().collect();
    let marker_indexes: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| parse_marker(line).map(|_| index))
        .collect();
    let node_re = Regex::new(r#"\b([A-Za-z_][\w-]*)\s*(?:\[\(|\[|\{|\(\()"#).unwrap();
    for (position, marker_index) in marker_indexes.iter().enumerate() {
        let end = marker_indexes.get(position + 1).copied().unwrap_or(lines.len());
        let seed = lines[marker_index + 1..end].join("\n");
        if seed.trim().is_empty() {
            continue;
        }
        let nodes: HashSet<&str> = node_re
            .captures_iter(&seed)
            .map(|captures| captures.get(1).unwrap().as_str())
            .collect();
        if nodes.len() < 3 {
            issues.push(Issue {
                level: Level::Warning,
                code: "thin_seeded_drill",
                message: format!(
                    "Line {}: seeded drill contains {} real node(s); use at least 3 meaningful nodes, or leave the marker bare to create an empty child.",
                    marker_index + 1,
                    nodes.len()
                ),
                line: Some(marker_index + 1),
            });
        }
    }

    // W1/W2 — global bracket & quote balance, flowcharts only.
    if is_flowchart(&repaired) {
        let mut sq = 0i32;
        let mut paren = 0i32;
        let mut brace = 0i32;
        let mut quotes = 0u32;
        for c in repaired.chars() {
            match c {
                '[' => sq += 1,
                ']' => sq -= 1,
                '(' => paren += 1,
                ')' => paren -= 1,
                '{' => brace += 1,
                '}' => brace -= 1,
                '"' => quotes += 1,
                _ => {}
            }
        }
        if sq != 0 || paren != 0 || brace != 0 {
            issues.push(Issue {
                level: Level::Warning,
                code: "unbalanced_brackets",
                message:
                    "Unbalanced brackets in the flowchart (`[ ]`, `( )` or `{ }` don't match). This often fails to render."
                        .to_string(),
                line: None,
            });
        }
        if quotes % 2 != 0 {
            issues.push(Issue {
                level: Level::Warning,
                code: "unbalanced_quotes",
                message: "Odd number of double-quotes in the flowchart — a node label quote is unclosed."
                    .to_string(),
                line: None,
            });
        }
    }

    LintReport { issues, repaired }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_markers_at_column_0_after_diagram() {
        let m = "graph TD\n    api[API]\n    auth[Auth]\n    api --> auth\n%% vaxis:drill auth";
        let r = lint(m);
        assert!(!r.has_errors(), "unexpected errors: {:?}", r.issues);
        assert_eq!(drill_node_ids(m), vec!["auth".to_string()]);
    }

    #[test]
    fn rejects_indented_marker_after_auto_repair_fails() {
        // A marker that is indented AND has trailing junk — can't be repaired.
        let m = "flowchart TB\n    api[API]\n    auth[Auth]\n    %% vaxis:drill auth trailing-junk here";
        let r = lint(m);
        assert!(r.has_errors());
        assert!(r.errors().any(|e| e.code == "drill_marker_not_at_column_0"));
    }

    #[test]
    fn auto_repairs_indented_marker() {
        let m = "flowchart TB\n    api[API]\n    auth[Auth]\n    api --> auth\n    %% vaxis:drill auth";
        let r = lint(m);
        // After repair, should be valid (no errors).
        assert!(!r.has_errors(), "expected auto-repair to fix indentation: {:?}", r.issues);
        assert!(r.fixed().any(|f| f.code == "drill_marker_indent_fixed"));
        assert!(r.repaired.contains("\n%% vaxis:drill auth"));
    }

    #[test]
    fn label_text_mentioning_drill_is_not_flagged() {
        let m = "graph TD\n    note[\"Use %% vaxis:drill auth at column 0\"]";
        let r = lint(m);
        assert!(!r.has_errors(), "unexpected errors: {:?}", r.issues);
    }

    #[test]
    fn comment_merely_mentioning_drill_is_not_flagged() {
        let m = "graph TD\n    a[A] --> b[B]\n%% remember to add vaxis:drill markers after the diagram";
        let r = lint(m);
        assert!(!r.has_errors(), "unexpected errors: {:?}", r.issues);
        assert!(drill_node_ids(m).is_empty());
    }

    #[test]
    fn rejects_marker_for_unknown_node() {
        let m = "graph TD\n    api[API]\n    api --> api\n%% vaxis:drill nope";
        let r = lint(m);
        assert!(r.errors().any(|e| e.code == "drill_node_not_found"));
    }

    #[test]
    fn rejects_inline_placement_via_node_check() {
        let m = "graph TD\n    api[API]\n%% vaxis:drill pay\n    pay[Pay]\n    db[(DB)]\n    pay --> db";
        let r = lint(m);
        assert!(r.errors().any(|e| e.code == "drill_node_not_found"));
    }

    #[test]
    fn rejects_drill_on_non_flowchart() {
        let m = "sequenceDiagram\n    A->>B: hi\n%% vaxis:drill A";
        let r = lint(m);
        assert!(r.errors().any(|e| e.code == "drill_on_non_flowchart"));
    }

    #[test]
    fn clean_flowchart_without_drills_is_ok() {
        let m = "graph TD\n    a[A] --> b[B]";
        let r = lint(m);
        assert!(!r.has_errors());
        assert!(drill_node_ids(m).is_empty());
    }

    #[test]
    fn bare_marker_does_not_warn_as_thin_seed() {
        let m = "flowchart TB\n  api[API]\n  auth[Auth]\n  api --> auth\n%% vaxis:drill auth";
        let report = lint(m);
        assert!(!report.warnings().any(|issue| issue.code == "thin_seeded_drill"));
    }

    #[test]
    fn warns_for_non_empty_seed_below_three_nodes() {
        let m = "flowchart TB\n  api[API]\n  auth[Auth]\n  api --> auth\n%% vaxis:drill auth\nflowchart TB\n  login[Login] --> token[Token]";
        let report = lint(m);
        assert!(report.warnings().any(|issue| issue.code == "thin_seeded_drill"));
        assert!(!report.has_errors());
    }

    #[test]
    fn accepts_seed_with_three_real_nodes() {
        let m = "flowchart TB\n  api[API]\n  auth[Auth]\n  api --> auth\n%% vaxis:drill auth\nflowchart TB\n  login[Login] --> token[Token]\n  token --> session[(Session Store)]";
        let report = lint(m);
        assert!(!report.warnings().any(|issue| issue.code == "thin_seeded_drill"));
        assert!(!report.has_errors());
    }

    #[test]
    fn er_diagram_braces_do_not_warn() {
        let m = "erDiagram\n    USER ||--o{ ORDER : places";
        let r = lint(m);
        assert!(r.issues.is_empty(), "unexpected issues: {:?}", r.issues);
    }

    // ── Direction checks ────────────────────────────────────────────────────

    #[test]
    fn auto_repairs_td_to_tb() {
        let m = "flowchart TD\n    a[A] --> b[B]";
        let r = lint(m);
        assert!(!r.has_errors(), "TD should be auto-repaired: {:?}", r.errors().collect::<Vec<_>>());
        assert!(r.fixed().any(|f| f.code == "direction_td_repaired"));
        assert!(r.repaired.starts_with("flowchart TB"));
    }

    #[test]
    fn auto_repairs_graph_td_to_tb() {
        let m = "graph TD\n    a[A] --> b[B]";
        let r = lint(m);
        assert!(!r.has_errors());
        assert!(r.fixed().any(|f| f.code == "direction_td_repaired"));
        assert!(r.repaired.starts_with("graph TB"));
    }

    #[test]
    fn rejects_bt_direction() {
        let m = "flowchart BT\n    a[A] --> b[B]";
        let r = lint(m);
        assert!(r.errors().any(|e| e.code == "direction_unsupported"));
    }

    #[test]
    fn rejects_rl_direction() {
        let m = "flowchart RL\n    a[A] --> b[B]";
        let r = lint(m);
        assert!(r.errors().any(|e| e.code == "direction_unsupported"));
    }

    #[test]
    fn accepts_tb_and_lr() {
        for dir in &["TB", "LR"] {
            let m = format!("flowchart {}\n    a[A] --> b[B]", dir);
            let r = lint(&m);
            assert!(!r.has_errors(), "direction {} should be accepted: {:?}", dir, r.issues);
        }
    }

    // ── Subgraph check ─────────────────────────────────────────────────────

    #[test]
    fn rejects_subgraph_blocks() {
        let m = "flowchart TB\n    subgraph cluster\n        a[A]\n    end\n    a --> b[B]";
        let r = lint(m);
        assert!(r.errors().any(|e| e.code == "subgraph_present"));
    }

    #[test]
    fn non_flowchart_subgraph_not_flagged() {
        // Subgraph keyword in a non-flowchart context is irrelevant.
        let m = "sequenceDiagram\n    A->>B: subgraph message";
        let r = lint(m);
        assert!(!r.errors().any(|e| e.code == "subgraph_present"));
    }

    // ── Forbidden shape checks ─────────────────────────────────────────────

    #[test]
    fn rejects_hexagon_shape() {
        let m = "flowchart TB\n    a{{Hexagon}} --> b[B]";
        let r = lint(m);
        assert!(r.errors().any(|e| e.code == "hexagon_shape"),
            "expected hexagon_shape error: {:?}", r.issues);
    }

    #[test]
    fn rejects_circle_shape() {
        let m = "flowchart TB\n    a((Circle)) --> b[B]";
        let r = lint(m);
        assert!(r.errors().any(|e| e.code == "circle_shape"),
            "expected circle_shape error: {:?}", r.issues);
    }

    #[test]
    fn rejects_stadium_shape() {
        let m = "flowchart TB\n    a([Stadium]) --> b[B]";
        let r = lint(m);
        assert!(r.errors().any(|e| e.code == "stadium_shape"),
            "expected stadium_shape error: {:?}", r.issues);
    }

    #[test]
    fn allows_cylinder_shape() {
        // Cylinder [(label)] is the storage shape — must be allowed.
        let m = "flowchart TB\n    db[(Database)] --> api[API]";
        let r = lint(m);
        assert!(!r.errors().any(|e| e.code == "circle_shape" || e.code == "stadium_shape"),
            "cylinder shape should be allowed: {:?}", r.issues);
    }

    #[test]
    fn allows_rhombus_shape() {
        // Rhombus {label} is the yes/no decision shape — must be allowed.
        let m = "flowchart TB\n    q{Decision?} -->|yes| a[A]\n    q -->|no| b[B]";
        let r = lint(m);
        assert!(!r.errors().any(|e| e.code == "hexagon_shape"),
            "rhombus shape should be allowed: {:?}", r.issues);
    }

    // ── Edge label quoting ─────────────────────────────────────────────────

    #[test]
    fn auto_repairs_unquoted_edge_label_parens() {
        let m = "flowchart TB\n    a -->|Teacher (Extended)| b[B]";
        let r = lint(m);
        assert!(!r.has_errors(), "should be auto-repaired: {:?}", r.errors().collect::<Vec<_>>());
        assert!(r.fixed().any(|f| f.code == "edge_label_paren_quoted"));
        assert!(r.repaired.contains(r#"|"Teacher (Extended)"|"#));
    }

    #[test]
    fn does_not_double_quote_already_quoted_edge_label() {
        let m = r#"flowchart TB
    a -->|"Teacher (Extended)"| b[B]"#;
        let r = lint(m);
        assert!(!r.has_errors());
        assert!(!r.fixed().any(|f| f.code == "edge_label_paren_quoted"),
            "already-quoted label should not be re-quoted");
    }

    #[test]
    fn does_not_quote_unbalanced_parens_in_edge_label() {
        // Unbalanced parens can't be safely auto-quoted.
        let m = "flowchart TB\n    a -->|Label (unclosed| b[B]";
        let r = lint(m);
        assert!(!r.fixed().any(|f| f.code == "edge_label_paren_quoted"),
            "unbalanced parens should not be auto-quoted");
    }

    // ── Regression: SKILL.md examples ─────────────────────────────────────

    /// Every drill example in the shipped SKILL.md must parse to at least one
    /// drill under the real server rules.
    #[test]
    fn skill_md_drill_examples_produce_drills() {
        let skill = include_str!("../skill-data/core/SKILL.md");
        let blocks = fenced_mermaid_blocks(skill);
        let drill_blocks: Vec<&String> = blocks
            .iter()
            .filter(|b| b.contains("vaxis:drill") && is_flowchart(b))
            .collect();
        assert!(
            !drill_blocks.is_empty(),
            "no mermaid drill examples found in SKILL.md — did the extractor break?"
        );
        for block in drill_blocks {
            let report = lint(block);
            assert!(
                !report.has_errors(),
                "SKILL.md mermaid example has lint errors:\n{}\nerrors: {:?}",
                block,
                report.errors().collect::<Vec<_>>()
            );
            let ids = drill_node_ids(&report.repaired);
            assert!(
                !ids.is_empty(),
                "SKILL.md mermaid example produces zero drills:\n{}",
                block
            );
        }
    }

    fn fenced_mermaid_blocks(md: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut in_block = false;
        let mut keep = false;
        let mut buf = String::new();
        for line in md.lines() {
            if let Some(info) = line.trim_start().strip_prefix("```") {
                if in_block {
                    if keep {
                        out.push(std::mem::take(&mut buf));
                    }
                    buf.clear();
                    in_block = false;
                    keep = false;
                } else {
                    in_block = true;
                    let info = info.trim();
                    keep = info.is_empty() || info.eq_ignore_ascii_case("mermaid");
                }
                continue;
            }
            if in_block && keep {
                buf.push_str(line);
                buf.push('\n');
            }
        }
        out
    }
}
