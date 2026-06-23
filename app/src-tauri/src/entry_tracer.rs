use crate::types::{CallTraceNode, CallTraceResolution, FileContent, SymbolEntry};
use std::collections::{BTreeMap, BTreeSet, HashSet};

pub fn detect_entry_points(files: &[FileContent], symbols: &[SymbolEntry]) -> Vec<String> {
    let mut entries = BTreeSet::new();

    for file in files {
        let lower = file.path.to_ascii_lowercase();
        if lower.ends_with("main.py") || lower.ends_with("index.ts") || lower.ends_with("index.tsx") || lower.ends_with("index.js") || lower.ends_with("cmd/main.go") {
            entries.insert(file.path.clone());
        }
        if file.content.contains("@app.route") || file.content.contains("@router.get") || file.content.contains("router.post") || file.content.contains("router.get") {
            entries.insert(file.path.clone());
        }
        if file.content.contains("if __name__ == \"__main__\"") || file.content.contains("if __name__ == '__main__'") {
            entries.insert(file.path.clone());
        }
    }

    for symbol in symbols {
        let lower = symbol.name.to_ascii_lowercase();
        if matches!(lower.as_str(), "handler" | "main" | "app" | "server") {
            entries.insert(format!("{}:{}", symbol.file, symbol.name));
        }
    }

    entries.into_iter().collect()
}

pub fn build_call_trace(entry_points: &[String], symbols: &[SymbolEntry], max_depth: usize) -> (Vec<CallTraceNode>, Vec<String>) {
    let mut warnings = Vec::new();
    let mut by_name: BTreeMap<String, Vec<&SymbolEntry>> = BTreeMap::new();
    for symbol in symbols {
        by_name.entry(symbol.name.clone()).or_default().push(symbol);
    }

    let mut roots = Vec::new();
    for entry in entry_points {
        let matched = find_entry_symbol(entry, symbols);
        if let Some(symbol) = matched {
            roots.push(build_node(symbol, &by_name, max_depth, 0, &mut HashSet::new(), &mut warnings, true));
        } else {
            roots.push(CallTraceNode {
                name: entry.clone(),
                file: entry.split(':').next().unwrap_or(entry).to_string(),
                line: 0,
                is_entry_point: true,
                resolution: CallTraceResolution::Unresolved,
                callees: vec![],
            });
        }
    }

    if roots.is_empty() {
        warnings.push("No call trace roots were built because no entry points were detected".to_string());
    }

    (roots, warnings)
}

fn find_entry_symbol<'a>(entry: &str, symbols: &'a [SymbolEntry]) -> Option<&'a SymbolEntry> {
    if let Some((file, name)) = entry.rsplit_once(':') {
        if let Some(symbol) = symbols.iter().find(|s| s.file == file && s.name == name) {
            return Some(symbol);
        }
    }
    symbols.iter().find(|s| entry == s.file || entry.ends_with(&format!(":{}", s.name)))
}

fn build_node(
    symbol: &SymbolEntry,
    by_name: &BTreeMap<String, Vec<&SymbolEntry>>,
    max_depth: usize,
    depth: usize,
    seen: &mut HashSet<String>,
    warnings: &mut Vec<String>,
    is_entry_point: bool,
) -> CallTraceNode {
    let key = format!("{}:{}", symbol.file, symbol.name);
    if seen.contains(&key) {
        warnings.push(format!("Cycle detected at {}", key));
        return node_for(symbol, is_entry_point, CallTraceResolution::Cycle, vec![]);
    }
    if depth >= max_depth {
        warnings.push(format!("Call trace depth capped at {}", key));
        return node_for(symbol, is_entry_point, CallTraceResolution::DepthCapped, vec![]);
    }

    seen.insert(key.clone());
    let mut callees = Vec::new();
    let mut resolution = CallTraceResolution::Resolved;

    for call in &symbol.calls {
        match by_name.get(call) {
            Some(matches) if matches.len() == 1 => {
                callees.push(build_node(matches[0], by_name, max_depth, depth + 1, seen, warnings, false));
            }
            Some(matches) if matches.len() > 1 => {
                resolution = CallTraceResolution::Ambiguous;
                warnings.push(format!("Ambiguous call '{}' from {} matches {} symbols", call, key, matches.len()));
                let first = matches[0];
                callees.push(node_for(first, false, CallTraceResolution::Ambiguous, vec![]));
            }
            _ => {}
        }
    }

    seen.remove(&key);
    node_for(symbol, is_entry_point, resolution, callees)
}

fn node_for(symbol: &SymbolEntry, is_entry_point: bool, resolution: CallTraceResolution, callees: Vec<CallTraceNode>) -> CallTraceNode {
    CallTraceNode {
        name: symbol.name.clone(),
        file: symbol.file.clone(),
        line: symbol.line,
        is_entry_point,
        resolution,
        callees,
    }
}
