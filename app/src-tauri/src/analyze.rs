use crate::ai::AiBackend;
use crate::analysis_cache;
use crate::architecture::build_architecture_graph;
use crate::config::resolve_github_token;
use crate::entry_tracer::{build_call_trace, detect_entry_points};
use crate::github::GithubClient;
use crate::onboarding_pack::build_onboarding_pack;
use crate::prompts::build_repo_overview_prompt;
use crate::repo_analyzer::{
    choose_default_workspace_scope, detect_monorepo_info, detect_stack, detect_useful_commands,
    select_analysis_files,
};
use crate::repo_parser::parse_repo_ref;
use crate::symbol_extractor::extract_symbols;
use crate::types::{AnalysisProgress, FetchStatus, OnboardingPack, RepoAnalysis, Settings};
use tauri::Emitter;

const MAX_ANALYSIS_FILES: usize = 500;
const MAX_BYTES_PER_FILE: usize = 200_000;
const MAX_CONTENT_CONCURRENCY: usize = 8;
const MAX_CALL_TRACE_DEPTH: usize = 5;
const TOTAL_STEPS: u8 = 7;

fn emit_progress(
    app: &tauri::AppHandle,
    step: u8,
    label: &str,
    status: FetchStatus,
    detail: Option<String>,
    items: Option<(u32, u32)>,
) {
    let _ = app.emit(
        "analysis-progress",
        AnalysisProgress {
            step,
            total_steps: TOTAL_STEPS,
            label: label.to_string(),
            status,
            detail,
            items_done: items.map(|(done, _)| done),
            items_total: items.map(|(_, total)| total),
        },
    );
}

pub async fn analyze_repo_impl(
    repo_url: &str,
    selected_scope: Option<String>,
    settings: &Settings,
    app: &tauri::AppHandle,
) -> Result<RepoAnalysis, String> {
    let parsed = parse_repo_ref(repo_url)?;
    let repo_name = format!("{}/{}", parsed.owner, parsed.repo);
    let mut warnings = Vec::new();

    emit_progress(
        app,
        1,
        "Fetching repository tree",
        FetchStatus::Running,
        Some(repo_name.clone()),
        None,
    );
    let github = GithubClient::new(resolve_github_token(settings));
    let tree = github.get_repo_tree(&parsed.owner, &parsed.repo).await?;
    if tree
        .iter()
        .any(|f| f.path == ".atlas-tree-truncated" || f.path == ".repo-lens-tree-truncated")
    {
        warnings.push("GitHub reported the repository tree as truncated".to_string());
    }
    emit_progress(
        app,
        1,
        "Fetching repository tree",
        FetchStatus::Done,
        Some(format!("{} files", tree.len())),
        None,
    );

    emit_progress(
        app,
        2,
        "Detecting monorepo and selecting files",
        FetchStatus::Running,
        None,
        None,
    );
    let root_manifest_files: Vec<_> = tree
        .iter()
        .filter(|f| !f.path.contains('/') && crate::repo_analyzer::is_manifest_or_key_file(&f.path))
        .cloned()
        .collect();
    let root_contents = github
        .get_file_contents_batch(
            &parsed.owner,
            &parsed.repo,
            &root_manifest_files,
            MAX_CONTENT_CONCURRENCY,
            MAX_BYTES_PER_FILE,
        )
        .await;
    let mut monorepo = detect_monorepo_info(&tree, &root_contents);
    let selected_scope = selected_scope
        .filter(|scope| !scope.trim().is_empty() && scope != "__global__")
        .or_else(|| choose_default_workspace_scope(&monorepo));
    monorepo.selected_scope = selected_scope.clone();
    monorepo.scope_reason = if let Some(scope) = &selected_scope {
        format!("Monorepo detected; analysis scoped to {}", scope)
    } else if monorepo.detected {
        "Monorepo detected; using bounded global analysis".to_string()
    } else {
        "Single-project repository shape detected".to_string()
    };
    if monorepo.detected {
        warnings.push(monorepo.scope_reason.clone());
    }
    let (selected_files, truncated) = select_analysis_files(
        &tree,
        selected_scope.as_deref(),
        MAX_ANALYSIS_FILES,
        &mut warnings,
    );
    emit_progress(
        app,
        2,
        "Detecting monorepo and selecting files",
        FetchStatus::Done,
        Some(format!("{} selected", selected_files.len())),
        None,
    );

    emit_progress(
        app,
        3,
        "Reading selected files",
        FetchStatus::Running,
        None,
        Some((0, selected_files.len() as u32)),
    );
    let contents = github
        .get_file_contents_batch(
            &parsed.owner,
            &parsed.repo,
            &selected_files,
            MAX_CONTENT_CONCURRENCY,
            MAX_BYTES_PER_FILE,
        )
        .await;
    for content in &contents {
        if let Some(warning) = &content.warning {
            warnings.push(format!("{}: {}", content.path, warning));
        }
    }
    emit_progress(
        app,
        3,
        "Reading selected files",
        FetchStatus::Done,
        Some(format!("{} files", contents.len())),
        Some((contents.len() as u32, selected_files.len() as u32)),
    );

    emit_progress(app, 4, "Detecting stack", FetchStatus::Running, None, None);
    let stack = detect_stack(&contents);
    let useful_commands = detect_useful_commands(&contents);
    emit_progress(
        app,
        4,
        "Detecting stack",
        FetchStatus::Done,
        Some(if stack.is_empty() {
            "unknown stack".to_string()
        } else {
            stack.join(" · ")
        }),
        None,
    );

    emit_progress(
        app,
        5,
        "Extracting symbols and call trace",
        FetchStatus::Running,
        None,
        None,
    );
    let (symbols, symbol_warnings) = extract_symbols(&contents);
    warnings.extend(symbol_warnings);
    let architecture = build_architecture_graph(&contents, &symbols);
    let entry_points = detect_entry_points(&contents, &symbols);
    if entry_points.is_empty() {
        warnings.push("No likely entry points detected".to_string());
    }
    let (call_trace, trace_warnings) =
        build_call_trace(&entry_points, &symbols, MAX_CALL_TRACE_DEPTH);
    warnings.extend(trace_warnings);
    emit_progress(
        app,
        5,
        "Extracting symbols and call trace",
        FetchStatus::Done,
        Some(format!("{} symbols", symbols.len())),
        None,
    );

    emit_progress(app, 6, "Writing overview", FetchStatus::Running, None, None);
    let overview_md = if settings.model.trim().is_empty() {
        let warning = "No model configured; deterministic analysis completed without LLM overview"
            .to_string();
        warnings.push(warning.clone());
        format!(
            "# {}\n\n{}\n\nStack: {}\n",
            repo_name,
            warning,
            if stack.is_empty() {
                "unknown".to_string()
            } else {
                stack.join(", ")
            }
        )
    } else {
        match AiBackend::new(&settings.model, &settings.aws_profile).await {
            Ok(ai) => {
                let prompt = build_repo_overview_prompt(
                    &repo_name,
                    &stack,
                    &entry_points,
                    &symbols,
                    &call_trace,
                    &contents,
                    &warnings,
                    Some(&monorepo),
                );
                match ai.invoke(&prompt).await {
                    Ok(text) => text,
                    Err(err) => {
                        let warning = format!("LLM overview failed: {}", err);
                        warnings.push(warning.clone());
                        format!("# {}\n\n{}\n", repo_name, warning)
                    }
                }
            }
            Err(err) => {
                let warning = format!("LLM setup failed: {}", err);
                warnings.push(warning.clone());
                format!("# {}\n\n{}\n", repo_name, warning)
            }
        }
    };
    emit_progress(app, 6, "Writing overview", FetchStatus::Done, None, None);

    emit_progress(
        app,
        7,
        "Building onboarding pack",
        FetchStatus::Running,
        None,
        None,
    );
    let mut analysis = RepoAnalysis {
        repo_url: parsed.normalized_url,
        repo_name,
        stack,
        entry_points,
        symbols,
        call_trace,
        architecture,
        overview_md,
        truncated,
        warnings,
        onboarding_pack: OnboardingPack::default(),
        monorepo: Some(monorepo),
    };
    let pack = build_onboarding_pack(&analysis, &useful_commands)?;
    analysis.onboarding_pack = pack;
    let _ = analysis_cache::save_cached_analysis(&analysis);
    emit_progress(
        app,
        7,
        "Building onboarding pack",
        FetchStatus::Done,
        None,
        None,
    );

    Ok(analysis)
}
