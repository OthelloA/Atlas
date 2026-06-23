use crate::types::{ExportResult, OnboardingPack, RepoAnalysis};
use chrono::Utc;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize)]
struct RepoAnalysisExport<'a> {
    repo_url: &'a str,
    repo_name: &'a str,
    stack: &'a [String],
    entry_points: &'a [String],
    symbols: &'a [crate::types::SymbolEntry],
    call_trace: &'a [crate::types::CallTraceNode],
    overview_md: &'a str,
    truncated: bool,
    warnings: &'a [String],
    monorepo: &'a Option<crate::types::MonorepoInfo>,
}

pub fn build_onboarding_pack(
    analysis: &RepoAnalysis,
    useful_commands: &[String],
) -> Result<OnboardingPack, String> {
    let symbols_json = serde_json::to_string_pretty(&analysis.symbols)
        .map_err(|e| format!("Failed to serialize symbols: {}", e))?;
    let call_trace_json = serde_json::to_string_pretty(&analysis.call_trace)
        .map_err(|e| format!("Failed to serialize call trace: {}", e))?;
    let warnings_json = serde_json::to_string_pretty(&analysis.warnings)
        .map_err(|e| format!("Failed to serialize warnings: {}", e))?;
    let export = RepoAnalysisExport {
        repo_url: &analysis.repo_url,
        repo_name: &analysis.repo_name,
        stack: &analysis.stack,
        entry_points: &analysis.entry_points,
        symbols: &analysis.symbols,
        call_trace: &analysis.call_trace,
        overview_md: &analysis.overview_md,
        truncated: analysis.truncated,
        warnings: &analysis.warnings,
        monorepo: &analysis.monorepo,
    };
    let repo_analysis_json = serde_json::to_string_pretty(&export)
        .map_err(|e| format!("Failed to serialize repo analysis: {}", e))?;

    Ok(OnboardingPack {
        overview_md: analysis.overview_md.clone(),
        start_here_md: build_start_here(analysis, useful_commands),
        symbols_json,
        call_trace_json,
        repo_analysis_json,
        warnings_json,
        claude_snippet_md: build_claude_snippet(),
    })
}

pub fn export_onboarding_pack_to_dir(
    repo_name: &str,
    pack: &OnboardingPack,
    target_dir: &Path,
) -> Result<ExportResult, String> {
    let safe_repo = repo_name.replace('/', "-").replace(' ', "-");
    let dir = target_dir.join(format!("atlas-{}-{}", safe_repo, Utc::now().format("%Y-%m-%d")));
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create export directory: {}", e))?;

    let files = [
        ("overview.md", &pack.overview_md),
        ("start-here.md", &pack.start_here_md),
        ("symbols.json", &pack.symbols_json),
        ("call-trace.json", &pack.call_trace_json),
        ("repo-analysis.json", &pack.repo_analysis_json),
        ("warnings.json", &pack.warnings_json),
        ("claude-snippet.md", &pack.claude_snippet_md),
    ];

    let mut written = Vec::new();
    for (name, content) in files {
        let path = dir.join(name);
        fs::write(&path, content).map_err(|e| format!("Failed to write {}: {}", name, e))?;
        written.push(name.to_string());
    }

    Ok(ExportResult {
        path: Some(dir.to_string_lossy().to_string()),
        files_written: written,
        fallback_required: false,
        message: "Onboarding Pack exported".to_string(),
    })
}

fn build_claude_snippet() -> String {
    r#"## Repo navigation

Generated Atlas maps are available in the onboarding pack:

- `overview.md`
- `start-here.md`
- `symbols.json`
- `call-trace.json`
- `repo-analysis.json`
- `warnings.json`

Before searching broadly, check these files first.

Use:
- `overview.md` for high-level structure
- `start-here.md` for onboarding order
- `symbols.json` to locate functions/classes/types
- `call-trace.json` for approximate entry-point flow
- `repo-analysis.json` for stack, entry points, commands, and monorepo scope
- `warnings.json` to understand incomplete or truncated analysis

Only fall back to `rg`, `find`, or opening many files when the generated maps are incomplete or stale.
"#.to_string()
}

fn build_start_here(analysis: &RepoAnalysis, useful_commands: &[String]) -> String {
    let mut md = format!("# Start Here: {}\n\n", analysis.repo_name);
    if let Some(monorepo) = &analysis.monorepo {
        if monorepo.detected {
            md.push_str("## Monorepo Scope\n");
            md.push_str(&format!("- Selected scope: {}\n", monorepo.selected_scope.as_deref().unwrap_or("global bounded analysis")));
            md.push_str(&format!("- Reason: {}\n", monorepo.scope_reason));
            if !monorepo.workspace_candidates.is_empty() {
                md.push_str("- Workspace candidates:\n");
                for candidate in &monorepo.workspace_candidates {
                    md.push_str(&format!("  - `{}` — {}\n", candidate.path, candidate.reason));
                }
            }
            md.push('\n');
        }
    }

    md.push_str("## First Files To Read\n");
    let mut files: Vec<String> = analysis.symbols.iter().map(|s| s.file.clone()).collect();
    files.sort();
    files.dedup();
    for (idx, file) in files.iter().take(8).enumerate() {
        md.push_str(&format!("{}. `{}`\n", idx + 1, file));
    }
    if files.is_empty() {
        md.push_str("No symbol-bearing files were detected. Start with README and manifest files shown in the UI warnings/context.\n");
    }

    md.push_str("\n## Likely Entry Points\n");
    if analysis.entry_points.is_empty() {
        md.push_str("No likely entry points were detected.\n");
    } else {
        for entry in &analysis.entry_points {
            md.push_str(&format!("- `{}`\n", entry));
        }
    }

    md.push_str("\n## Useful Commands Detected\n");
    if useful_commands.is_empty() {
        md.push_str("No commands detected from manifests.\n");
    } else {
        for command in useful_commands {
            md.push_str(&format!("- `{}`\n", command));
        }
    }

    md.push_str("\n## Warnings\n");
    if analysis.warnings.is_empty() {
        md.push_str("No warnings.\n");
    } else {
        for warning in &analysis.warnings {
            md.push_str(&format!("- {}\n", warning));
        }
    }

    md
}
