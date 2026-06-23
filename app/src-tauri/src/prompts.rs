use crate::types::{AuditFinding, CallTraceNode, FileContent, MonorepoInfo, RepoAnalysis, SymbolEntry};

pub fn build_repo_overview_prompt(
    repo_name: &str,
    stack: &[String],
    entry_points: &[String],
    symbols: &[SymbolEntry],
    call_trace: &[CallTraceNode],
    files: &[FileContent],
    warnings: &[String],
    monorepo: Option<&MonorepoInfo>,
) -> String {
    let symbol_preview: Vec<_> = symbols.iter().take(80).collect();
    let file_preview: Vec<String> = files
        .iter()
        .take(20)
        .map(|f| format!("{} ({} bytes)", f.path, f.content.len()))
        .collect();
    let call_trace_json = serde_json::to_string_pretty(call_trace).unwrap_or_else(|_| "[]".to_string());
    let monorepo_json = serde_json::to_string_pretty(&monorepo).unwrap_or_else(|_| "null".to_string());

    format!(
        r#"You are writing an onboarding overview for a developer joining repository `{repo_name}`.

Use only the grounded analysis below. Be concise, practical, and honest about limitations. Do not claim compiler-grade precision.

Return markdown with these sections:
# {repo_name}
## What this repo appears to be
## Stack
## How execution likely starts
## Important files to read first
## Best-effort symbols and call trace notes
## Warnings / limits

Stack: {stack:?}
Entry points: {entry_points:?}
Selected files: {file_preview:?}
Symbol preview: {symbol_preview:#?}
Approximate call trace JSON: {call_trace_json}
Monorepo info: {monorepo_json}
Warnings: {warnings:?}
"#
    )
}

pub fn build_well_architected_audit_prompt(
    analysis: &RepoAnalysis,
    files: &[FileContent],
    rule_findings: &[AuditFinding],
) -> String {
    let file_snippets: Vec<String> = files
        .iter()
        .filter(|f| is_audit_key_file(&f.path))
        .take(30)
        .map(|f| {
            let snippet: String = f.content.lines().take(80).collect::<Vec<_>>().join("\n");
            format!("FILE: {}\n{}", f.path, snippet)
        })
        .collect();
    let symbols: Vec<_> = analysis.symbols.iter().take(100).collect();
    let rules_json = serde_json::to_string_pretty(rule_findings).unwrap_or_else(|_| "[]".to_string());
    let monorepo_json = serde_json::to_string_pretty(&analysis.monorepo).unwrap_or_else(|_| "null".to_string());

    format!(
        r#"You are a senior staff engineer performing a repository audit.

Analyze the `{repo}` codebase against AWS Well-Architected pillars plus general UX/DX:
1. Operational Excellence
2. Security
3. Reliability
4. Performance Efficiency
5. Cost Optimization
6. Sustainability
7. Developer Experience
8. User Experience

Rank findings P0-P5:
P0 = production incident or exploitable security risk
P1 = likely severe bug or reliability/security issue
P2 = meaningful reliability or maintainability debt
P3 = maintainability/DX issue
P4 = UX/DX polish issue
P5 = low-priority cleanup

Rules:
- Do not invent files, functions, or line numbers.
- Every finding must cite file:line evidence from provided context or deterministic rule findings.
- Prefer fewer high-confidence findings over speculative findings.
- If evidence is insufficient, omit the finding.
- Deterministic rule findings are trusted evidence.
- Return only valid JSON matching this shape:
{{"findings":[{{"pillar":"Security","priority":"P1","title":"...","description":"...","file":"src/file.ts","line":42,"evidence":"...","impact":"...","recommendation":"...","confidence":"high|medium|low","source":"llm|rule+llm","rule_override":false}}]}}

Repository context:
Repo: {repo}
Stack: {stack:?}
Entry points: {entry_points:?}
Warnings: {warnings:?}
Monorepo: {monorepo_json}
Symbol preview: {symbols:#?}
Deterministic rule findings: {rules_json}
Key file snippets:
{file_snippets}
"#,
        repo = analysis.repo_name,
        stack = analysis.stack,
        entry_points = analysis.entry_points,
        warnings = analysis.warnings,
        file_snippets = file_snippets.join("\n\n---\n\n"),
    )
}

fn is_audit_key_file(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with("package.json")
        || lower.ends_with("pyproject.toml")
        || lower.ends_with("requirements.txt")
        || lower.ends_with("go.mod")
        || lower.ends_with("cargo.toml")
        || lower.contains("auth")
        || lower.contains("config")
        || lower.contains("route")
        || lower.contains("handler")
        || lower.contains("middleware")
        || lower.contains("server")
        || lower.contains("api")
}
