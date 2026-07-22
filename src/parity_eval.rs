//! Deterministic structural metrics for the native-vs-direct Mermaid parity eval.
//!
//! This module intentionally does not call a model or the Vaxis API. It supplies
//! stable measurements that can be applied to outputs captured from either path.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MermaidMetrics {
    pub direction: Option<String>,
    pub node_count: usize,
    pub edge_count: usize,
    pub subgraph_count: usize,
    pub max_connections: usize,
    pub cylinder_count: usize,
    pub rhombus_count: usize,
    pub drill_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EvalCase {
    pub id: String,
    pub category: String,
    pub prompt: String,
    pub expect: Expectations,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Expectations {
    pub direction: Option<String>,
    #[serde(default)]
    pub min_nodes: usize,
    #[serde(default)]
    pub min_subgraphs: usize,
    #[serde(default)]
    pub required_labels: Vec<String>,
    #[serde(default)]
    pub requires_storage_shape: bool,
    #[serde(default)]
    pub requires_decision_shape: bool,
    pub max_connections_per_node: Option<usize>,
    #[serde(default)]
    pub min_drills: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct EvalFailure {
    pub rule: &'static str,
    pub message: String,
}

fn main_diagram(mermaid: &str) -> &str {
    let marker = Regex::new(r"(?m)^%%\s*vaxis:drill\s+[A-Za-z0-9_-]+\s*$").unwrap();
    marker.find(mermaid).map_or(mermaid, |m| &mermaid[..m.start()])
}

pub fn measure(mermaid: &str) -> MermaidMetrics {
    let main = main_diagram(mermaid);
    let header = Regex::new(r"(?im)^\s*(?:flowchart|graph)\s+(TB|TD|LR|RL|BT)\b").unwrap();
    let node = Regex::new(r#"\b([A-Za-z_][\w-]*)\s*(?:\[\(|\[|\{|\(\()"#).unwrap();
    let edge = Regex::new(
        r#"\b([A-Za-z_][\w-]*)(?:\s*(?:\[\([^\n]*?\)\]|\[[^\n]*?\]|\{[^\n]*?\}|\(\([^\n]*?\)\)))?\s*(?:-->|-\.->|==>)(?:\|[^|]*\|)?\s*([A-Za-z_][\w-]*)"#,
    )
    .unwrap();
    let cylinder = Regex::new(r#"\b[A-Za-z_][\w-]*\s*\[\("#).unwrap();
    let rhombus = Regex::new(r#"\b[A-Za-z_][\w-]*\s*\{"#).unwrap();
    let drill = Regex::new(r"(?m)^%%\s*vaxis:drill\s+([A-Za-z0-9_-]+)\s*$").unwrap();

    let mut nodes = HashSet::new();
    for line in main.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("subgraph ")
            || trimmed.starts_with("style ")
            || trimmed.starts_with("classDef ")
        {
            continue;
        }
        nodes.extend(node.captures_iter(line).map(|c| c[1].to_string()));
    }

    let mut connections: HashMap<String, usize> = HashMap::new();
    let mut edge_count = 0;
    for captures in edge.captures_iter(main) {
        edge_count += 1;
        *connections.entry(captures[1].to_string()).or_default() += 1;
        *connections.entry(captures[2].to_string()).or_default() += 1;
        nodes.insert(captures[1].to_string());
        nodes.insert(captures[2].to_string());
    }

    MermaidMetrics {
        direction: header.captures(main).map(|c| match &c[1] {
            "TD" => "TB".to_string(),
            other => other.to_uppercase(),
        }),
        node_count: nodes.len(),
        edge_count,
        subgraph_count: main
            .lines()
            .filter(|line| line.trim_start().starts_with("subgraph "))
            .count(),
        max_connections: connections.values().copied().max().unwrap_or(0),
        cylinder_count: cylinder.find_iter(main).count(),
        rhombus_count: rhombus.find_iter(main).count(),
        drill_count: drill
            .captures_iter(mermaid)
            .map(|c| c[1].to_string())
            .collect::<HashSet<_>>()
            .len(),
    }
}

pub fn evaluate(case: &EvalCase, mermaid: &str) -> Vec<EvalFailure> {
    let metrics = measure(mermaid);
    let expected = &case.expect;
    let mut failures = Vec::new();

    if let Some(direction) = &expected.direction {
        if metrics.direction.as_deref() != Some(direction.as_str()) {
            failures.push(EvalFailure {
                rule: "direction",
                message: format!("expected {direction}, got {:?}", metrics.direction),
            });
        }
    }
    if metrics.node_count < expected.min_nodes {
        failures.push(EvalFailure {
            rule: "min_nodes",
            message: format!("expected at least {}, got {}", expected.min_nodes, metrics.node_count),
        });
    }
    if metrics.subgraph_count < expected.min_subgraphs {
        failures.push(EvalFailure {
            rule: "min_subgraphs",
            message: format!(
                "expected at least {}, got {}",
                expected.min_subgraphs, metrics.subgraph_count
            ),
        });
    }
    if expected.requires_storage_shape && metrics.cylinder_count == 0 {
        failures.push(EvalFailure {
            rule: "storage_shape",
            message: "expected at least one cylinder".to_string(),
        });
    }
    if expected.requires_decision_shape && metrics.rhombus_count == 0 {
        failures.push(EvalFailure {
            rule: "decision_shape",
            message: "expected at least one rhombus".to_string(),
        });
    }
    if let Some(max) = expected.max_connections_per_node {
        if metrics.max_connections > max {
            failures.push(EvalFailure {
                rule: "max_connections",
                message: format!("expected at most {max}, got {}", metrics.max_connections),
            });
        }
    }
    if metrics.drill_count < expected.min_drills {
        failures.push(EvalFailure {
            rule: "min_drills",
            message: format!("expected at least {}, got {}", expected.min_drills, metrics.drill_count),
        });
    }
    let lower = mermaid.to_lowercase();
    for label in &expected.required_labels {
        if !lower.contains(&label.to_lowercase()) {
            failures.push(EvalFailure {
                rule: "required_label",
                message: format!("missing required label `{label}`"),
            });
        }
    }

    failures
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn measures_grouped_flowchart() {
        let mermaid = r#"flowchart TB
subgraph backend[Backend]
  api[API] --> auth{Authenticated?}
  auth --> db[(PostgreSQL)]
end
api --> queue[(Queue)]
%% vaxis:drill api"#;
        assert_eq!(
            measure(mermaid),
            MermaidMetrics {
                direction: Some("TB".to_string()),
                node_count: 4,
                edge_count: 3,
                subgraph_count: 1,
                max_connections: 2,
                cylinder_count: 2,
                rhombus_count: 1,
                drill_count: 1,
            }
        );
    }

    #[test]
    fn reports_expectation_failures() {
        let case = EvalCase {
            id: "example".to_string(),
            category: "test".to_string(),
            prompt: "test".to_string(),
            expect: Expectations {
                direction: Some("LR".to_string()),
                min_nodes: 3,
                min_subgraphs: 1,
                required_labels: vec!["Database".to_string()],
                requires_storage_shape: true,
                requires_decision_shape: false,
                max_connections_per_node: Some(4),
                min_drills: 0,
            },
        };
        let rules: Vec<&str> = evaluate(&case, "flowchart TB\n  a[A] --> b[B]")
            .iter()
            .map(|failure| failure.rule)
            .collect();
        assert_eq!(
            rules,
            vec!["direction", "min_nodes", "min_subgraphs", "storage_shape", "required_label"]
        );
    }

    #[test]
    fn parity_case_catalog_is_valid_and_unique() {
        let cases: Vec<EvalCase> = serde_json::from_str(include_str!(
            "../evals/diagram-parity-cases.json"
        ))
        .expect("parity case catalog must be valid JSON");
        assert_eq!(cases.len(), 10, "the initial milestone requires 10 cases");
        let ids: HashSet<&str> = cases.iter().map(|case| case.id.as_str()).collect();
        assert_eq!(ids.len(), cases.len(), "eval case ids must be unique");
        for case in cases {
            assert!(!case.id.trim().is_empty());
            assert!(!case.category.trim().is_empty());
            assert!(!case.prompt.trim().is_empty());
            assert!(case.expect.min_nodes > 0);
        }
    }
}
