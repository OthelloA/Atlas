use crate::types::{
    ArchitectureEdge, ArchitectureEvidence, ArchitectureGraph, ArchitectureNode, FileContent,
    SymbolEntry,
};
use std::collections::{BTreeMap, BTreeSet};

/// Builds a conservative architecture graph from parsed symbols and source evidence.
pub fn build_architecture_graph(
    files: &[FileContent],
    symbols: &[SymbolEntry],
    entry_points: &[String],
) -> ArchitectureGraph {
    let mut nodes = BTreeMap::<String, ArchitectureNode>::new();
    let mut edges = BTreeMap::<(String, String, String), ArchitectureEdge>::new();
    let symbols_by_name: BTreeMap<String, Vec<&SymbolEntry>> =
        symbols.iter().fold(BTreeMap::new(), |mut map, symbol| {
            map.entry(symbol.name.clone()).or_default().push(symbol);
            map
        });

    for file in files {
        let id = file_node_id(&file.path);
        let container_id = container_node_id(&file.path);
        nodes
            .entry(container_id.clone())
            .or_insert_with(|| ArchitectureNode {
                id: container_id.clone(),
                name: container_name(&file.path),
                node_type: "container".to_string(),
                c4_level: "container".to_string(),
                parent_id: None,
                is_entry_point: entry_points.iter().any(|entry| entry == &file.path),
                description: "Repository container inferred from the source directory boundary"
                    .to_string(),
                confidence: confidence_label(0.75).to_string(),
                confidence_score: 0.75,
                deterministic: true,
                source_refs: vec![file.path.clone()],
                evidence: vec![evidence(
                    "directory_boundary",
                    "path",
                    vec![file.path.clone()],
                    "Container inferred from the file's top-level directory.",
                )],
            });
        nodes.entry(id.clone()).or_insert_with(|| ArchitectureNode {
            id: id.clone(),
            name: file.path.clone(),
            node_type: "component".to_string(),
            c4_level: "component".to_string(),
            parent_id: Some(container_id.clone()),
            is_entry_point: entry_points
                .iter()
                .any(|entry| entry == &file.path || entry.starts_with(&format!("{}:", file.path))),
            description: format!("Parsed source component from {}", file.path),
            confidence: confidence_label(1.0).to_string(),
            confidence_score: 1.0,
            deterministic: true,
            source_refs: vec![file.path.clone()],
            evidence: vec![evidence(
                "parsed_source_file",
                "ast",
                vec![file.path.clone()],
                "The file was included in the bounded analysis and parsed with a language grammar.",
            )],
        });

        for dependency in imported_dependencies(file) {
            let external_id = format!("external:{}", dependency);
            nodes
                .entry(external_id.clone())
                .or_insert_with(|| ArchitectureNode {
                    id: external_id.clone(),
                    name: dependency.clone(),
                    node_type: "external".to_string(),
                    c4_level: "context".to_string(),
                    parent_id: None,
                    is_entry_point: false,
                    description: "Dependency detected from source imports or use declarations"
                        .to_string(),
                    confidence: confidence_label(0.8).to_string(),
                    confidence_score: 0.8,
                    deterministic: true,
                    source_refs: vec![file.path.clone()],
                    evidence: vec![evidence(
                        "explicit_import",
                        "import",
                        vec![file.path.clone()],
                        "An import or use declaration names this dependency.",
                    )],
                });
            add_edge(
                &mut edges,
                &container_id,
                &id,
                "contains",
                1.0,
                evidence(
                    "directory_contains_component",
                    "path",
                    vec![file.path.clone()],
                    "The component belongs to this inferred container.",
                ),
            );
            add_edge(
                &mut edges,
                &id,
                &external_id,
                "imports",
                0.8,
                evidence(
                    "explicit_import",
                    "import",
                    vec![file.path.clone()],
                    "An import or use declaration creates this dependency relationship.",
                ),
            );
        }
    }

    for symbol in symbols {
        let source = file_node_id(&symbol.file);
        for call in &symbol.calls {
            let Some(targets) = symbols_by_name.get(call) else {
                continue;
            };
            let unique_files: BTreeSet<String> =
                targets.iter().map(|target| target.file.clone()).collect();
            for target_file in unique_files {
                let target = file_node_id(&target_file);
                if target == source {
                    continue;
                }
                let score = if targets.len() == 1 { 0.9 } else { 0.55 };
                add_edge(
                    &mut edges,
                    &source,
                    &target,
                    "calls",
                    score,
                    evidence(
                        "symbol_call_resolution",
                        "ast",
                        vec![format!("{}:{}", symbol.file, symbol.line)],
                        &format!(
                            "The parsed symbol `{}` calls `{}`; resolution is {}.",
                            symbol.name,
                            call,
                            confidence_label(score)
                        ),
                    ),
                );
            }
        }
    }

    ArchitectureGraph {
        nodes: nodes.into_values().collect(),
        edges: edges.into_values().collect(),
        generated_from: "AST symbols, imports, and call expressions; no LLM inference".to_string(),
    }
}

fn add_edge(
    edges: &mut BTreeMap<(String, String, String), ArchitectureEdge>,
    source: &str,
    target: &str,
    relationship: &str,
    score: f32,
    item_evidence: ArchitectureEvidence,
) {
    let key = (
        source.to_string(),
        target.to_string(),
        relationship.to_string(),
    );
    edges
        .entry(key)
        .and_modify(|edge| {
            edge.evidence.push(item_evidence.clone());
            if score < edge.confidence_score {
                edge.confidence_score = score;
                edge.confidence = confidence_label(score).to_string();
            }
        })
        .or_insert_with(|| ArchitectureEdge {
            source: source.to_string(),
            target: target.to_string(),
            relationship: relationship.to_string(),
            confidence: confidence_label(score).to_string(),
            confidence_score: score,
            deterministic: true,
            evidence: vec![item_evidence],
        });
}

fn evidence(
    rule: &str,
    kind: &str,
    source_refs: Vec<String>,
    explanation: &str,
) -> ArchitectureEvidence {
    ArchitectureEvidence {
        rule: rule.to_string(),
        kind: kind.to_string(),
        source_refs,
        explanation: explanation.to_string(),
    }
}

fn confidence_label(score: f32) -> &'static str {
    if score >= 0.8 {
        "high"
    } else if score >= 0.6 {
        "medium"
    } else {
        "low"
    }
}

fn file_node_id(path: &str) -> String {
    format!("file:{}", path)
}

fn container_name(path: &str) -> String {
    path.split('/')
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or("root")
        .to_string()
}

fn container_node_id(path: &str) -> String {
    format!("container:{}", container_name(path))
}

fn imported_dependencies(file: &FileContent) -> Vec<String> {
    let mut dependencies = BTreeSet::new();
    for line in file.content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("import ")
            || (trimmed.starts_with("export ") && trimmed.contains(" from "))
        {
            if let Some(value) = quoted_value(trimmed) {
                dependencies.insert(normalize_dependency(&value));
            }
        } else if trimmed.starts_with("use ") {
            if let Some(value) = trimmed.strip_prefix("use ") {
                dependencies.insert(
                    value
                        .trim_end_matches(';')
                        .split("::")
                        .next()
                        .unwrap_or(value)
                        .to_string(),
                );
            }
        } else if trimmed.starts_with("from ") && trimmed.contains(" import ") {
            if let Some(value) = trimmed.strip_prefix("from ") {
                dependencies.insert(value.split_whitespace().next().unwrap_or(value).to_string());
            }
        }
    }
    dependencies
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect()
}

fn quoted_value(line: &str) -> Option<String> {
    let quote = line.find(['\'', '"'])?;
    let delimiter = line.as_bytes()[quote] as char;
    let rest = &line[quote + 1..];
    let end = rest.find(delimiter)?;
    Some(rest[..end].to_string())
}

fn normalize_dependency(value: &str) -> String {
    if value.starts_with('.') {
        value.to_string()
    } else {
        value.split('/').next().unwrap_or(value).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol_extractor::extract_symbols;

    #[test]
    fn graph_contains_explainable_import_evidence() {
        let files = vec![FileContent {
            path: "src/app.ts".to_string(),
            content: "import React from 'react'; export function App() {}".to_string(),
            truncated: false,
            warning: None,
        }];
        let (symbols, _) = extract_symbols(&files);
        let graph = build_architecture_graph(&files, &symbols, &["src/app.ts".to_string()]);
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.c4_level == "container" && node.name == "src"));
        let edge = graph
            .edges
            .iter()
            .find(|edge| edge.relationship == "imports")
            .unwrap();
        assert_eq!(edge.confidence, "high");
        assert_eq!(edge.evidence[0].rule, "explicit_import");
        assert_eq!(edge.evidence[0].source_refs[0], "src/app.ts");
    }
}
