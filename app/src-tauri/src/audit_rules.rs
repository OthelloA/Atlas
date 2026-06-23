use crate::types::{AuditFinding, FileContent, RepoAnalysis};
use regex::Regex;

pub fn run_audit_rules(analysis: &RepoAnalysis) -> Vec<AuditFinding> {
    let mut findings = Vec::new();

    if analysis.warnings.iter().any(|w| w.to_ascii_lowercase().contains("truncated")) {
        findings.push(finding(
            "Operational Excellence", "P3", "Analysis was truncated",
            "The repository exceeded analysis limits, so audit coverage may be incomplete.",
            "repo-analysis", 0, "Repo analysis warnings include truncation.",
            "Important files or workspaces may be missing from this review.",
            "Narrow scope to a workspace or raise analysis limits for deeper review.",
            "medium", "rule", true,
        ));
    }

    if analysis.monorepo.as_ref().is_some_and(|m| m.detected && m.selected_scope.is_none()) {
        findings.push(finding(
            "Developer Experience", "P3", "Monorepo audit is global and bounded",
            "A monorepo was detected but no selected workspace scope was available.",
            "repo-analysis", 0, "Monorepo detected with bounded global analysis.",
            "Findings may mix unrelated apps/packages and miss package-specific risks.",
            "Select or configure a workspace scope before treating audit output as complete.",
            "medium", "rule", true,
        ));
    }

    findings
}

pub fn run_file_audit_rules(files: &[FileContent]) -> Vec<AuditFinding> {
    let secret_re = Regex::new(r#"(?i)(api[_-]?key|secret|token|password)\s*[:=]\s*[\"'][^\"']{12,}[\"']"#).unwrap();
    let todo_security_re = Regex::new(r"(?i)(todo|fixme).*(auth|security|token|secret|permission)").unwrap();
    let unsafe_re = Regex::new(r"\b(eval|exec|Function)\s*\(").unwrap();
    let fetch_re = Regex::new(r"\b(fetch|axios\.|request\.|reqwest::|urllib|requests\.)").unwrap();
    let timeout_re = Regex::new(r"(?i)(timeout|AbortController|signal|deadline)").unwrap();
    let cors_re = Regex::new(r#"(?i)(allow_origins|origin|Access-Control-Allow-Origin).*(\*|allow_any_origin)"#).unwrap();
    let debug_re = Regex::new(r"\b(console\.log|println!|dbg!|print\()" ).unwrap();

    let mut findings = Vec::new();
    for file in files {
        if file.content.is_empty() || is_test_or_generated(&file.path) {
            continue;
        }
        for (idx, line) in file.content.lines().enumerate() {
            let line_no = (idx + 1) as u64;
            if secret_re.is_match(line) {
                findings.push(finding("Security", "P0", "Possible hardcoded secret", "A source line appears to assign a credential-like value directly in code.", &file.path, line_no, line.trim(), "Hardcoded credentials can leak into source control and production artifacts.", "Move the value to a secret manager or environment variable and rotate the exposed credential if real.", "high", "rule", true));
            }
            if todo_security_re.is_match(line) {
                findings.push(finding("Security", "P1", "Security-related TODO/FIXME", "A TODO/FIXME references security, auth, token, secret, or permissions.", &file.path, line_no, line.trim(), "Security debt called out in code can become a production risk if left unresolved.", "Resolve the TODO or track it explicitly with owner, priority, and acceptance criteria.", "high", "rule", true));
            }
            if unsafe_re.is_match(line) {
                findings.push(finding("Security", "P1", "Dynamic code execution", "The code appears to use eval/exec/Function-style dynamic execution.", &file.path, line_no, line.trim(), "Dynamic execution can enable injection vulnerabilities when inputs are not strictly controlled.", "Replace dynamic execution with explicit parsing/dispatch or tightly validate inputs before execution.", "high", "rule", true));
            }
            if cors_re.is_match(line) {
                findings.push(finding("Security", "P2", "Broad CORS origin", "CORS configuration appears to allow any origin.", &file.path, line_no, line.trim(), "Wildcard origins can expose APIs to unintended browser clients.", "Restrict allowed origins by environment and avoid wildcard origins for authenticated endpoints.", "medium", "rule", true));
            }
            if fetch_re.is_match(line) && !timeout_re.is_match(line) {
                findings.push(finding("Reliability", "P2", "External request without visible timeout", "An HTTP/external request call appears without timeout or cancellation evidence on the same line.", &file.path, line_no, line.trim(), "External calls can hang and consume resources under network failure.", "Use a bounded timeout, cancellation signal, or client-level timeout around external requests.", "medium", "rule", true));
            }
            if debug_re.is_match(line) {
                findings.push(finding("Developer Experience", "P4", "Debug logging in source path", "A debug print/log statement appears in non-test source.", &file.path, line_no, line.trim(), "Debug output can leak noisy or sensitive runtime details and make logs harder to use.", "Replace with structured logging at an appropriate level or remove before release.", "medium", "rule", true));
            }
        }
    }
    findings
}

fn is_test_or_generated(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("test") || lower.contains("spec") || lower.contains("node_modules") || lower.contains("dist/") || lower.contains("target/")
}

fn finding(
    pillar: &str,
    priority: &str,
    title: &str,
    description: &str,
    file: &str,
    line: u64,
    evidence: &str,
    impact: &str,
    recommendation: &str,
    confidence: &str,
    source: &str,
    rule_override: bool,
) -> AuditFinding {
    AuditFinding {
        pillar: pillar.to_string(),
        priority: priority.to_string(),
        title: title.to_string(),
        description: description.to_string(),
        file: file.to_string(),
        line,
        evidence: evidence.to_string(),
        impact: impact.to_string(),
        recommendation: recommendation.to_string(),
        confidence: confidence.to_string(),
        source: source.to_string(),
        rule_override,
    }
}
