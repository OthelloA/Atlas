use crate::types::{FileContent, SymbolEntry};
use regex::Regex;
use std::collections::BTreeSet;

pub fn extract_symbols(files: &[FileContent]) -> (Vec<SymbolEntry>, Vec<String>) {
    let mut symbols = Vec::new();
    let mut warnings = Vec::new();

    for file in files {
        if file.content.is_empty() {
            if let Some(warning) = &file.warning {
                warnings.push(format!("{}: {}", file.path, warning));
            }
            continue;
        }
        let start = symbols.len();
        if is_ts_js(&file.path) {
            extract_ts_js(file, &mut symbols);
        } else if file.path.ends_with(".py") {
            extract_python(file, &mut symbols);
        }
        populate_body_calls(file, &mut symbols[start..]);
    }

    if symbols.is_empty() {
        warnings.push("No exported/public symbols detected in selected files".to_string());
    }
    warnings.push("Symbol extraction is best-effort and may miss re-exports, dynamic exports, decorators, overloads, and multiline signatures".to_string());

    (symbols, warnings)
}

fn extract_ts_js(file: &FileContent, symbols: &mut Vec<SymbolEntry>) {
    let patterns: Vec<(Regex, &str)> = vec![
        (Regex::new(r"^\s*export\s+(?:async\s+)?function\s+([A-Za-z_$][\w$]*)").unwrap(), "function"),
        (Regex::new(r"^\s*export\s+class\s+([A-Za-z_$][\w$]*)").unwrap(), "class"),
        (Regex::new(r"^\s*export\s+(?:const|let|var)\s+([A-Za-z_$][\w$]*)").unwrap(), "export"),
        (Regex::new(r"^\s*export\s+default\s+(?:async\s+)?function\s+([A-Za-z_$][\w$]*)?").unwrap(), "function"),
        (Regex::new(r"^\s*export\s+default\s+class\s+([A-Za-z_$][\w$]*)?").unwrap(), "class"),
    ];

    for (idx, line) in file.content.lines().enumerate() {
        for (regex, kind) in &patterns {
            if let Some(caps) = regex.captures(line) {
                let name = caps.get(1).map(|m| m.as_str()).filter(|s| !s.is_empty()).unwrap_or("default");
                symbols.push(SymbolEntry {
                    name: name.to_string(),
                    kind: kind.to_string(),
                    file: file.path.clone(),
                    line: (idx + 1) as u64,
                    signature: line.trim().to_string(),
                    calls: extract_calls(line),
                });
                break;
            }
        }
    }
}

fn extract_python(file: &FileContent, symbols: &mut Vec<SymbolEntry>) {
    let def_re = Regex::new(r"^(async\s+def|def)\s+([A-Za-z_]\w*)").unwrap();
    let class_re = Regex::new(r"^class\s+([A-Za-z_]\w*)").unwrap();
    for (idx, line) in file.content.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.len() != line.len() {
            continue;
        }
        if let Some(caps) = def_re.captures(line) {
            symbols.push(SymbolEntry {
                name: caps[2].to_string(),
                kind: "function".to_string(),
                file: file.path.clone(),
                line: (idx + 1) as u64,
                signature: line.trim().to_string(),
                calls: extract_calls(line),
            });
        } else if let Some(caps) = class_re.captures(line) {
            symbols.push(SymbolEntry {
                name: caps[1].to_string(),
                kind: "class".to_string(),
                file: file.path.clone(),
                line: (idx + 1) as u64,
                signature: line.trim().to_string(),
                calls: extract_calls(line),
            });
        }
    }
}

fn extract_calls(line: &str) -> Vec<String> {
    let call_re = Regex::new(r"\b([A-Za-z_$][\w$]*)\s*\(").unwrap();
    let keywords: BTreeSet<&str> = ["if", "for", "while", "switch", "catch", "function", "return", "def", "class"]
        .into_iter()
        .collect();
    call_re
        .captures_iter(line)
        .filter_map(|caps| caps.get(1).map(|m| m.as_str().to_string()))
        .filter(|name| !keywords.contains(name.as_str()))
        .collect()
}

fn populate_body_calls(file: &FileContent, symbols: &mut [SymbolEntry]) {
    if symbols.is_empty() {
        return;
    }
    let lines: Vec<&str> = file.content.lines().collect();
    let mut starts: Vec<usize> = symbols.iter().map(|s| s.line.saturating_sub(1) as usize).collect();
    starts.push(lines.len());
    for idx in 0..symbols.len() {
        let start = starts[idx];
        let end = starts[idx + 1].min(lines.len());
        let mut calls = BTreeSet::new();
        for line in &lines[start..end] {
            for call in extract_calls(line) {
                if call != symbols[idx].name {
                    calls.insert(call);
                }
            }
        }
        symbols[idx].calls = calls.into_iter().collect();
    }
}

fn is_ts_js(path: &str) -> bool {
    path.ends_with(".ts") || path.ends_with(".tsx") || path.ends_with(".js") || path.ends_with(".jsx")
}
