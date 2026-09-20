use crate::types::{FileContent, SymbolEntry};
use std::collections::BTreeSet;
use tree_sitter::{Node, Parser};

/// Extract symbols with language parsers instead of line-oriented regular expressions.
/// The parser is deliberately conservative: unsupported syntax is reported as a warning
/// rather than presented as a confidently incorrect symbol.
pub fn extract_symbols(files: &[FileContent]) -> (Vec<SymbolEntry>, Vec<String>) {
    let mut symbols = Vec::new();
    let mut warnings = Vec::new();
    let mut parsed_files = 0usize;
    let mut unsupported_files = 0usize;

    for file in files {
        if file.content.is_empty() {
            if let Some(warning) = &file.warning {
                warnings.push(format!("{}: {}", file.path, warning));
            }
            continue;
        }

        let Some(language) = language_for_path(&file.path) else {
            unsupported_files += 1;
            continue;
        };

        let mut parser = Parser::new();
        if parser.set_language(&language).is_err() {
            warnings.push(format!("{}: parser could not be configured", file.path));
            continue;
        }
        let Some(tree) = parser.parse(&file.content, None) else {
            warnings.push(format!("{}: parser returned no syntax tree", file.path));
            continue;
        };
        parsed_files += 1;
        let before = symbols.len();
        collect_declarations(tree.root_node(), file, &mut symbols);
        if tree.root_node().has_error() {
            warnings.push(format!(
                "{}: syntax errors were present; results may be incomplete",
                file.path
            ));
        }
        if symbols.len() == before && is_supported_source(&file.path) {
            warnings.push(format!("{}: no supported declarations detected", file.path));
        }
    }

    if symbols.is_empty() {
        warnings.push("No parsed symbols detected in selected files".to_string());
    }
    if parsed_files > 0 {
        warnings.push(format!(
            "Parsed {} source file(s) with language-aware syntax trees",
            parsed_files
        ));
    }
    if unsupported_files > 0 {
        warnings.push(format!(
            "Skipped {} file(s) with no supported parser",
            unsupported_files
        ));
    }

    (symbols, warnings)
}

fn language_for_path(path: &str) -> Option<tree_sitter::Language> {
    if path.ends_with(".tsx") {
        Some(tree_sitter_typescript::LANGUAGE_TSX.into())
    } else if path.ends_with(".ts") {
        Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
    } else if path.ends_with(".jsx")
        || path.ends_with(".js")
        || path.ends_with(".mjs")
        || path.ends_with(".cjs")
    {
        Some(tree_sitter_javascript::LANGUAGE.into())
    } else if path.ends_with(".py") {
        Some(tree_sitter_python::LANGUAGE.into())
    } else if path.ends_with(".rs") {
        Some(tree_sitter_rust::LANGUAGE.into())
    } else {
        None
    }
}

fn is_supported_source(path: &str) -> bool {
    language_for_path(path).is_some()
}

fn collect_declarations(node: Node<'_>, file: &FileContent, symbols: &mut Vec<SymbolEntry>) {
    let kind = node.kind();
    if is_declaration_kind(kind) && should_include(node, kind) {
        if let Some(name) = declaration_name(node, file.content.as_bytes()) {
            let start = node.start_position();
            let signature = file
                .content
                .lines()
                .nth(start.row)
                .unwrap_or("")
                .trim()
                .to_string();
            let calls = collect_calls(node, file.content.as_bytes());
            symbols.push(SymbolEntry {
                name,
                kind: declaration_category(kind).to_string(),
                file: file.path.clone(),
                line: (start.row + 1) as u64,
                signature,
                calls,
            });
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_declarations(child, file, symbols);
    }
}

fn is_declaration_kind(kind: &str) -> bool {
    matches!(
        kind,
        "function_declaration"
            | "function_definition"
            | "function_item"
            | "method_definition"
            | "class_declaration"
            | "class_definition"
            | "class_definition_statement"
            | "struct_item"
            | "enum_item"
            | "trait_item"
            | "interface_declaration"
            | "type_alias_declaration"
            | "lexical_declaration"
    )
}

fn declaration_category(kind: &str) -> &'static str {
    match kind {
        "class_declaration"
        | "class_definition"
        | "class_definition_statement"
        | "struct_item"
        | "enum_item"
        | "trait_item" => "class",
        "interface_declaration" => "interface",
        "type_alias_declaration" => "type",
        "lexical_declaration" => "export",
        _ => "function",
    }
}

fn declaration_name(node: Node<'_>, source: &[u8]) -> Option<String> {
    let candidate = node.child_by_field_name("name").or_else(|| {
        if node.kind() == "lexical_declaration" {
            let mut cursor = node.walk();
            let declarator = node
                .named_children(&mut cursor)
                .find(|child| child.kind() == "variable_declarator");
            declarator.and_then(|declarator| declarator.child_by_field_name("name"))
        } else {
            node.child_by_field_name("declarator")
        }
    });
    candidate
        .and_then(|name| name.utf8_text(source).ok())
        .map(str::to_string)
        .filter(|name| !name.contains('(') && !name.contains('='))
}

fn should_include(node: Node<'_>, kind: &str) -> bool {
    // JavaScript/TypeScript projects commonly keep private helpers in the same file.
    // Include declarations, but retain the existing public-only behavior for lexical
    // declarations by requiring an export ancestor.
    if kind == "lexical_declaration" {
        return has_ancestor(node, "export_statement");
    }
    // Python and Rust declarations are useful even when not explicitly exported.
    true
}

fn has_ancestor(mut node: Node<'_>, kind: &str) -> bool {
    while let Some(parent) = node.parent() {
        if parent.kind() == kind {
            return true;
        }
        node = parent;
    }
    false
}

fn collect_calls(node: Node<'_>, source: &[u8]) -> Vec<String> {
    let mut calls = BTreeSet::new();
    collect_calls_inner(node, source, &mut calls);
    calls.into_iter().collect()
}

fn collect_calls_inner(node: Node<'_>, source: &[u8], calls: &mut BTreeSet<String>) {
    if matches!(
        node.kind(),
        "call_expression" | "call" | "function_call" | "macro_invocation"
    ) {
        if let Some(function) = node
            .child_by_field_name("function")
            .or_else(|| node.child_by_field_name("macro"))
            .or_else(|| node.named_child(0))
        {
            if let Ok(text) = function.utf8_text(source) {
                let name = text.trim().trim_start_matches("self.").to_string();
                if is_valid_call_name(&name) {
                    calls.insert(name);
                }
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_calls_inner(child, source, calls);
    }
}

fn is_valid_call_name(name: &str) -> bool {
    !name.is_empty()
        && !matches!(
            name,
            "if" | "for" | "while" | "switch" | "catch" | "function" | "return"
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, content: &str) -> FileContent {
        FileContent {
            path: path.to_string(),
            content: content.to_string(),
            truncated: false,
            warning: None,
        }
    }

    #[test]
    fn parses_typescript_functions_and_calls() {
        let (symbols, warnings) = extract_symbols(&[file(
            "src/app.ts",
            "export function start() { loadConfig(); }\nfunction loadConfig() { return parse(); }",
        )]);
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("syntax trees")));
        assert_eq!(symbols[0].name, "start");
        assert!(symbols[0].calls.contains(&"loadConfig".to_string()));
        assert_eq!(symbols[1].name, "loadConfig");
    }

    #[test]
    fn parses_exported_constants() {
        let (symbols, _) =
            extract_symbols(&[file("src/config.ts", "export const config = loadConfig();")]);
        assert_eq!(symbols[0].name, "config");
        assert_eq!(symbols[0].kind, "export");
        assert!(symbols[0].calls.contains(&"loadConfig".to_string()));
    }

    #[test]
    fn parses_python_and_rust_declarations() {
        let (symbols, _) = extract_symbols(&[
            file(
                "main.py",
                "def run():\n    print('ok')\n\nclass Worker:\n    pass",
            ),
            file("src/lib.rs", "pub fn run() { work(); }\nstruct Worker;"),
        ]);
        assert!(symbols
            .iter()
            .any(|symbol| symbol.name == "run" && symbol.file == "main.py"));
        assert!(symbols
            .iter()
            .any(|symbol| symbol.name == "Worker" && symbol.file == "src/lib.rs"));
    }
}
