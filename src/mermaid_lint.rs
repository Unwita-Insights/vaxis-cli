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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Level {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct Issue {
    pub level: Level,
    pub code: &'static str,
    pub message: String,
    /// 1-based line number the issue anchors to, when known.
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct LintReport {
    pub issues: Vec<Issue>,
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
}

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
/// prose that merely contain the literal text `vaxis:drill` (e.g. a diagram that
/// documents Vaxis itself), which are not `%% vaxis:drill …` comments.
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
/// Node ids are ASCII, so byte scanning is safe even when `haystack` holds
/// multi-byte UTF-8 label text (a lead/continuation byte is >= 0x80, never a
/// word byte, so it reads as a boundary).
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

/// The drill node ids the SERVER would keep for this Mermaid — valid column-0
/// markers whose id is a real node in a flowchart main diagram, de-duplicated
/// in first-seen order. Used by the regression test to assert the documented
/// examples actually produce drills; also a stable hook for callers that want
/// to preview the drill set without a round-trip.
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

/// Run every preflight check over the Mermaid about to be sent.
pub fn lint(mermaid: &str) -> LintReport {
    let mut issues = Vec::new();
    let a = analyze(mermaid);

    // E1 — a line that means to be a drill marker (a `%% vaxis:drill …` comment)
    // but the server won't recognise it: indented, or otherwise malformed. This
    // is the exact bug that made every documented drill example silently produce
    // zero drills. Gated on looks_like_drill_marker so a node label or prose that
    // merely contains the text `vaxis:drill` is NOT flagged.
    for (i, line) in mermaid.split('\n').enumerate() {
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
            // E2 — markers appear before any diagram content: the main diagram
            // is empty and everything got parsed as child content.
            issues.push(Issue {
                level: Level::Error,
                code: "drill_marker_before_diagram",
                message:
                    "Drill markers appear before any diagram content. Put the complete main diagram FIRST, then the `%% vaxis:drill` markers after it."
                        .to_string(),
                line: a.first_marker_line,
            });
        } else if !is_flowchart(&a.main) {
            // E4 — drills on a non-flowchart: the server drops them entirely.
            issues.push(Issue {
                level: Level::Error,
                code: "drill_on_non_flowchart",
                message:
                    "Drill markers only work on flowcharts (`graph` / `flowchart`). Remove them, or make the main diagram a flowchart."
                        .to_string(),
                line: a.first_marker_line,
            });
        } else {
            // E3 — a marker's id isn't a node in the main diagram, so the server
            // drops that drill. Also catches markers placed BETWEEN nodes: the
            // nodes after the first marker fall out of the main diagram, so their
            // ids no longer resolve.
            for (line, id) in &a.markers {
                if !word_present(&a.main, id) {
                    issues.push(Issue {
                        level: Level::Error,
                        code: "drill_node_not_found",
                        message: format!(
                            "Line {}: `%% vaxis:drill {}` references a node not defined in the main diagram. Define `{}` as a node above, or (if markers are between nodes) move ALL markers after the complete diagram.",
                            line, id, id
                        ),
                        line: Some(*line),
                    });
                }
            }
        }
    }

    // W1/W2 — bracket & quote balance, flowcharts only. Other diagram types use
    // characters like `{`/`}` for their own syntax (ER cardinality `||--o{`),
    // which would false-positive, so we scope these to flowcharts.
    if is_flowchart(mermaid) {
        let mut sq = 0i32; // [ ]
        let mut paren = 0i32; // ( )
        let mut brace = 0i32; // { }
        let mut quotes = 0u32;
        for c in mermaid.chars() {
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

    LintReport { issues }
}

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
    fn rejects_indented_marker() {
        // The original SKILL.md bug: indented marker → server ignores it.
        let m = "graph TD\n    api[API]\n    auth[Auth]\n    %% vaxis:drill auth";
        let r = lint(m);
        assert!(r.has_errors());
        assert!(r.errors().any(|e| e.code == "drill_marker_not_at_column_0"));
        assert!(drill_node_ids(m).is_empty());
    }

    #[test]
    fn label_text_mentioning_drill_is_not_flagged() {
        // The literal text `vaxis:drill` inside a node label must not be mistaken
        // for a malformed marker — it isn't a `%% vaxis:drill …` comment. Common
        // in diagrams that document Vaxis itself.
        let m = "graph TD\n    note[\"Use %% vaxis:drill auth at column 0\"]";
        let r = lint(m);
        assert!(!r.has_errors(), "unexpected errors: {:?}", r.issues);
    }

    #[test]
    fn comment_merely_mentioning_drill_is_not_flagged() {
        // A genuine comment that references the term but isn't a marker.
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
        // Marker at col 0 but BETWEEN nodes: `pay`/`db` fall into child content,
        // so the main diagram is just `graph TD / api`, and `pay` no longer
        // resolves as a main-diagram node.
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
        assert!(!lint(m).has_errors());
        assert!(drill_node_ids(m).is_empty());
    }

    #[test]
    fn er_diagram_braces_do_not_warn() {
        // ER cardinality uses `{`/`}` — must not trip the bracket-balance check.
        let m = "erDiagram\n    USER ||--o{ ORDER : places";
        let r = lint(m);
        assert!(r.issues.is_empty(), "unexpected issues: {:?}", r.issues);
    }

    /// Regression guard (task B): every drill example in the shipped SKILL.md
    /// must parse to at least one drill under the real server rules. If someone
    /// re-indents a marker or slips one between nodes, this fails loudly instead
    /// of the docs silently teaching broken syntax again.
    #[test]
    fn skill_md_drill_examples_produce_drills() {
        let skill = include_str!("../skills/SKILL.md");
        let blocks = fenced_mermaid_blocks(skill);
        // Real drill examples are flowcharts. Gating on that skips prose blocks
        // that merely mention `%% vaxis:drill` in a sentence (e.g. workflow steps).
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
            assert!(
                !drill_node_ids(block).is_empty(),
                "SKILL.md mermaid example produces zero drills:\n{}",
                block
            );
        }
    }

    /// Extract fenced code blocks whose info string marks them as plain Mermaid
    /// (empty or `mermaid`) — skips ```bash command examples, which wrap Mermaid
    /// in shell syntax and aren't valid standalone diagrams.
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
