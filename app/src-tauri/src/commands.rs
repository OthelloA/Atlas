use crate::analysis_cache;
use crate::analyze::analyze_repo_impl;
use crate::audit::run_audit_impl;
use crate::config::{load_settings, save_settings_to_disk};
use crate::github::GithubClient;
use crate::onboarding_pack::export_onboarding_pack_to_dir;
use crate::repo_analyzer::{detect_monorepo_info, is_manifest_or_key_file};
use crate::repo_parser::parse_repo_ref;
use crate::types::{AuditResult, ExportResult, MonorepoInfo, OnboardingPack, RepoAnalysis, Settings};
use std::path::PathBuf;
use tauri::command;

#[command]
pub fn get_settings() -> Settings {
    load_settings()
}

#[command]
pub fn save_settings(settings: Settings) -> Result<(), String> {
    save_settings_to_disk(&settings)
}

#[command]
pub async fn analyze_repo(
    app: tauri::AppHandle,
    repo_url: String,
    selected_scope: Option<String>,
) -> Result<RepoAnalysis, String> {
    let settings = load_settings();
    analyze_repo_impl(&repo_url, selected_scope, &settings, &app).await
}

#[command]
pub async fn detect_repo_scopes(repo_url: String) -> Result<MonorepoInfo, String> {
    let settings = load_settings();
    let parsed = parse_repo_ref(&repo_url)?;
    let github = GithubClient::new(crate::config::resolve_github_token(&settings));
    let tree = github.get_repo_tree(&parsed.owner, &parsed.repo).await?;
    let root_manifest_files: Vec<_> = tree
        .iter()
        .filter(|f| !f.path.contains('/') && is_manifest_or_key_file(&f.path))
        .cloned()
        .collect();
    let root_contents = github
        .get_file_contents_batch(&parsed.owner, &parsed.repo, &root_manifest_files, 6, 200_000)
        .await;
    Ok(detect_monorepo_info(&tree, &root_contents))
}

#[command]
pub fn load_cached_analysis(repo_url: String) -> Option<RepoAnalysis> {
    analysis_cache::load_cached_analysis(&repo_url)
}

#[command]
pub async fn run_audit(analysis: RepoAnalysis) -> Result<AuditResult, String> {
    let settings = load_settings();
    run_audit_impl(analysis, &settings).await
}

#[command]
pub async fn export_onboarding_pack(
    repo_name: String,
    pack: OnboardingPack,
    target_dir: Option<String>,
) -> Result<ExportResult, String> {
    let dir = match target_dir.filter(|d| !d.trim().is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None => downloads_dir()?,
    };
    export_onboarding_pack_to_dir(&repo_name, &pack, &dir)
}

fn downloads_dir() -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "Could not resolve HOME directory for Downloads export".to_string())?;
    let downloads = PathBuf::from(home).join("Downloads");
    if downloads.exists() {
        Ok(downloads)
    } else {
        Err("Downloads directory was not found. Use individual file downloads instead.".to_string())
    }
}
