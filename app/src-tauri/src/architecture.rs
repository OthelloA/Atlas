use crate::types::{
    ArchitectureEdge, ArchitectureGraph, ArchitectureNode, FileContent, SymbolEntry,
};
use std::collections::{BTreeMap, BTreeSet};

/// Builds a conservative architecture graph from parsed symbols and source evidence.
/// This is intentionally not a business-domain guess: every relationship carries a
/// source reference and confidence that the UI can expose to the user.
pub fn build_architecture_graph(
    files: &[FileContent],
    symbols: &[SymbolEntry],
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
        nodes.entry(id.clone()).or_insert_with(|| ArchitectureNode {
            id: id.clone(),
            name: file.path.clone(),
            node_type: "component".to_string(),
            description: format!("Parsed source component from {}", file.path),
            confidence: "high".to_string(),
            source_refs: vec![file.path.clone()],
        });

        for dependency in imported_dependencies(file) {
            let external_id = format!("external:{}", dependency);
            nodes
                .entry(external_id.clone())
                .or_insert_with(|| ArchitectureNode {
                    id: external_id.clone(),
                    name: dependency.clone(),
                    node_type: "external".to_string(),
                    description: "Dependency detected from source imports or use declarations"
                        .to_string(),
                    confidence: "medium".to_string(),
                    source_refs: vec![file.path.clone()],
                });
            add_edge(
                &mut edges,
                &id,
                &external_id,
                "imports",
                "medium",
                vec![file.path.clone()],
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
                add_edge(
                    &mut edges,
                    &source,
                    &target,
                    "calls",
                    if targets.len() == 1 { "high" } else { "low" },
                    vec![format!("{}:{} calls {}", symbol.file, symbol.line, call)],
                );
            }
        }
    }

    ArchitectureGraph {
        nodes: nodes.into_values().collect(),
        edges: edges.into_values().collect(),
        generated_from: "AST symbols, imports, and call expressions".to_string(),
    }
}

fn add_edge(
    edges: &mut BTreeMap<(String, String, String), ArchitectureEdge>,
    source: &str,
    target: &str,
    relationship: &str,
    confidence: &str,
    evidence: Vec<String>,
) {
    let key = (
        source.to_string(),
        target.to_string(),
        relationship.to_string(),
    );
    edges
        .entry(key)
        .and_modify(|edge| edge.evidence.extend(evidence.clone()))
        .or_insert_with(|| ArchitectureEdge {
            source: source.to_string(),
            target: target.to_string(),
            relationship: relationship.to_string(),
            confidence: confidence.to_string(),
            evidence,
        });
}

fn file_node_id(path: &str) -> String {
    format!("file:{}", path)
}

fn imported_dependencies(file: &FileContent) -> Vec<String> {
    let mut dependencies = BTreeSet::new();
    for line in file.content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("import ")
            || trimmed.starts_with("export ") && trimmed.contains(" from ")
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
    fn graph_contains_evidence_backed_import_edge() {
        let files = vec![FileContent {
            path: "src/app.ts".to_string(),
            content: "import React from 'react'; export function App() {}".to_string(),
            truncated: false,
            warning: None,
        }];
        let (symbols, _) = extract_symbols(&files);
        let graph = build_architecture_graph(&files, &symbols);
        assert!(graph.nodes.iter().any(|node| node.name == "react"));
        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.relationship == "imports" && edge.evidence[0] == "src/app.ts"));
    }
}
