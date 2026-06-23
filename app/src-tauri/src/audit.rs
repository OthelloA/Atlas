use crate::ai::AiBackend;
use crate::audit_rules::{run_audit_rules, run_file_audit_rules};
use crate::github::GithubClient;
use crate::prompts::build_well_architected_audit_prompt;
use crate::repo_analyzer::select_analysis_files;
use crate::repo_parser::parse_repo_ref;
use crate::types::{AuditFinding, AuditResult, AuditSummary, FileContent, RepoAnalysis, Settings};
use std::collections::{BTreeMap, BTreeSet};

const MAX_AUDIT_FILES: usize = 120;
const MAX_AUDIT_BYTES_PER_FILE: usize = 120_000;
const MAX_AUDIT_CONCURRENCY: usize = 6;

pub async fn run_audit_impl(analysis: RepoAnalysis, settings: &Settings) -> Result<AuditResult, String> {
    let mut warnings = Vec::new();
    let parsed = parse_repo_ref(&analysis.repo_url)?;
    let github = GithubClient::new(crate::config::resolve_github_token(settings));

    let tree = github.get_repo_tree(&parsed.owner, &parsed.repo).await?;
    let mut selection_warnings = Vec::new();
    let scope = analysis.monorepo.as_ref().and_then(|m| m.selected_scope.as_deref());
    let (files, truncated) = select_analysis_files(&tree, scope, MAX_AUDIT_FILES, &mut selection_warnings);
    if truncated {
        warnings.push(format!("Audit file selection truncated to {} files", MAX_AUDIT_FILES));
    }
    warnings.extend(selection_warnings);
    let contents = github
        .get_file_contents_batch(&parsed.owner, &parsed.repo, &files, MAX_AUDIT_CONCURRENCY, MAX_AUDIT_BYTES_PER_FILE)
        .await;

    let mut findings = Vec::new();
    findings.extend(run_audit_rules(&analysis));
    findings.extend(run_file_audit_rules(&contents));

    if settings.model.trim().is_empty() {
        warnings.push("No model configured; audit uses deterministic rules only".to_string());
    } else {
        match run_llm_audit(&analysis, &contents, &findings, settings).await {
            Ok(mut llm_findings) => findings.append(&mut llm_findings),
            Err(err) => warnings.push(format!("LLM audit failed; deterministic findings shown: {}", err)),
        }
    }

    findings = dedupe_findings(findings);
    findings.sort_by_key(|f| priority_rank(&f.priority));
    let summary = summarize(&findings);
    Ok(AuditResult { findings, summary, warnings })
}

async fn run_llm_audit(
    analysis: &RepoAnalysis,
    contents: &[FileContent],
    rule_findings: &[AuditFinding],
    settings: &Settings,
) -> Result<Vec<AuditFinding>, String> {
    let ai = AiBackend::new(&settings.model, &settings.aws_profile).await?;
    let prompt = build_well_architected_audit_prompt(analysis, contents, rule_findings);
    let raw = ai.invoke(&prompt).await?;
    let json = extract_json_object(&raw)?;
    let findings_value = json
        .get("findings")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Audit response missing findings array".to_string())?;
    let findings: Vec<AuditFinding> = serde_json::from_value(serde_json::Value::Array(findings_value.clone()))
        .map_err(|e| format!("Failed to parse LLM audit findings: {}", e))?;
    Ok(findings.into_iter().filter(valid_finding).collect())
}

fn extract_json_object(text: &str) -> Result<serde_json::Value, String> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(text.trim()) {
        if value.is_object() {
            return Ok(value);
        }
    }
    if let (Some(start), Some(end)) = (text.find('{'), text.rfind('}')) {
        if end > start {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&text[start..=end]) {
                if value.is_object() {
                    return Ok(value);
                }
            }
        }
    }
    Err(format!("Could not extract JSON object from audit response: {}", &text[..text.len().min(500)]))
}

fn valid_finding(f: &AuditFinding) -> bool {
    !f.file.trim().is_empty()
        && !f.title.trim().is_empty()
        && matches!(f.priority.as_str(), "P0" | "P1" | "P2" | "P3" | "P4" | "P5")
}

fn dedupe_findings(findings: Vec<AuditFinding>) -> Vec<AuditFinding> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for finding in findings {
        let key = format!("{}:{}:{}", finding.file, finding.line, finding.title.to_ascii_lowercase());
        if seen.insert(key) {
            out.push(finding);
        }
    }
    out
}

fn summarize(findings: &[AuditFinding]) -> AuditSummary {
    let mut summary = AuditSummary::default();
    let mut pillars: BTreeMap<String, u32> = BTreeMap::new();
    for finding in findings {
        match finding.priority.as_str() {
            "P0" => summary.p0 += 1,
            "P1" => summary.p1 += 1,
            "P2" => summary.p2 += 1,
            "P3" => summary.p3 += 1,
            "P4" => summary.p4 += 1,
            "P5" => summary.p5 += 1,
            _ => {}
        }
        *pillars.entry(finding.pillar.clone()).or_default() += 1;
    }
    summary.top_pillar = pillars.into_iter().max_by_key(|(_, count)| *count).map(|(pillar, _)| pillar);
    summary
}

fn priority_rank(priority: &str) -> u8 {
    match priority {
        "P0" => 0,
        "P1" => 1,
        "P2" => 2,
        "P3" => 3,
        "P4" => 4,
        "P5" => 5,
        _ => 6,
    }
}
