use crate::types::{FileContent, MonorepoInfo, RepoFile, WorkspaceCandidate};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub fn is_ignored_path(path: &str) -> bool {
    path.split('/').any(|part| matches!(part,
        "node_modules" | "dist" | "build" | ".git" | "vendor" | "target" | ".next" |
        "coverage" | "__pycache__" | ".turbo" | ".cache"
    ))
}

pub fn is_supported_source(path: &str) -> bool {
    matches!(extension(path).as_deref(), Some("ts" | "tsx" | "js" | "jsx" | "py"))
}

pub fn is_manifest_or_key_file(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    matches!(name.as_str(),
        "package.json" | "pyproject.toml" | "requirements.txt" | "go.mod" | "cargo.toml" |
        "sst.config.ts" | "sst.config.js" | "pnpm-workspace.yaml" | "turbo.json" |
        "nx.json" | "lerna.json" | "rush.json" | "readme.md" | "readme"
    )
}

pub fn is_priority_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    ["main", "index", "app", "server", "route", "routes", "handler", "middleware", "auth", "config", "cmd"]
        .iter()
        .any(|needle| lower.contains(needle))
}

pub fn select_analysis_files(
    tree: &[RepoFile],
    selected_scope: Option<&str>,
    max_files: usize,
    warnings: &mut Vec<String>,
) -> (Vec<RepoFile>, bool) {
    let mut root_context = Vec::new();
    let mut priority = Vec::new();
    let mut normal = Vec::new();

    for file in tree.iter().filter(|f| f.file_type == "blob") {
        if is_ignored_path(&file.path) {
            continue;
        }
        let in_scope = selected_scope
            .map(|scope| file.path.starts_with(&format!("{}/", scope.trim_matches('/'))) || !file.path.contains('/'))
            .unwrap_or(true);
        if !in_scope {
            continue;
        }
        if is_manifest_or_key_file(&file.path) && !file.path.contains('/') {
            root_context.push(file.clone());
        } else if is_manifest_or_key_file(&file.path) || (is_supported_source(&file.path) && is_priority_path(&file.path)) {
            priority.push(file.clone());
        } else if is_supported_source(&file.path) {
            normal.push(file.clone());
        }
    }

    root_context.sort_by(|a, b| a.path.cmp(&b.path));
    priority.sort_by(|a, b| a.path.cmp(&b.path));
    normal.sort_by(|a, b| a.path.cmp(&b.path));

    let mut selected = Vec::new();
    selected.extend(root_context);
    selected.extend(priority);
    selected.extend(normal);

    let truncated = selected.len() > max_files;
    if truncated {
        warnings.push(format!("Analysis truncated to {} selected files", max_files));
        selected.truncate(max_files);
    }
    (selected, truncated)
}

pub fn detect_stack(files: &[FileContent]) -> Vec<String> {
    let mut stack = BTreeSet::new();
    for file in files {
        let path = file.path.to_ascii_lowercase();
        let content = file.content.to_ascii_lowercase();
        if path.ends_with("package.json") {
            stack.insert("javascript/typescript".to_string());
            if content.contains("\"typescript\"") { stack.insert("typescript".to_string()); }
            if content.contains("\"react\"") { stack.insert("react".to_string()); }
            if content.contains("\"next\"") { stack.insert("next.js".to_string()); }
            if content.contains("\"express\"") { stack.insert("express".to_string()); }
            if content.contains("\"vite\"") { stack.insert("vite".to_string()); }
        }
        if path.ends_with("pyproject.toml") || path.ends_with("requirements.txt") {
            stack.insert("python".to_string());
            if content.contains("fastapi") { stack.insert("fastapi".to_string()); }
            if content.contains("flask") { stack.insert("flask".to_string()); }
            if content.contains("django") { stack.insert("django".to_string()); }
        }
        if path.ends_with("go.mod") { stack.insert("go".to_string()); }
        if path.ends_with("cargo.toml") { stack.insert("rust".to_string()); }
        if path.ends_with("sst.config.ts") || path.ends_with("sst.config.js") { stack.insert("sst".to_string()); }
    }
    stack.into_iter().collect()
}

pub fn detect_useful_commands(files: &[FileContent]) -> Vec<String> {
    let mut commands = BTreeSet::new();
    for file in files {
        let name = file.path.rsplit('/').next().unwrap_or(&file.path);
        if name == "package.json" {
            if let Ok(json) = serde_json::from_str::<Value>(&file.content) {
                if let Some(scripts) = json.get("scripts").and_then(|v| v.as_object()) {
                    for key in ["dev", "start", "build", "test", "tauri"] {
                        if scripts.contains_key(key) {
                            commands.insert(format!("npm run {}", key));
                        }
                    }
                }
            }
        } else if name == "pyproject.toml" {
            commands.insert("python -m pytest".to_string());
        } else if name == "go.mod" {
            commands.insert("go test ./...".to_string());
        } else if name == "Cargo.toml" {
            commands.insert("cargo test".to_string());
        }
    }
    commands.into_iter().collect()
}

pub fn detect_monorepo_info(tree: &[RepoFile], files: &[FileContent]) -> MonorepoInfo {
    let mut indicators = BTreeSet::new();
    let mut candidate_paths: BTreeMap<String, String> = BTreeMap::new();

    let package_json_count = tree.iter().filter(|f| f.path.ends_with("package.json")).count();
    let pyproject_count = tree.iter().filter(|f| f.path.ends_with("pyproject.toml")).count();
    let gomod_count = tree.iter().filter(|f| f.path.ends_with("go.mod")).count();

    for file in tree {
        let path = &file.path;
        if matches!(path.as_str(), "pnpm-workspace.yaml" | "turbo.json" | "nx.json" | "lerna.json" | "rush.json") {
            indicators.insert(path.clone());
        }
        if let Some((root, rest)) = path.split_once('/') {
            if matches!(root, "apps" | "packages" | "services" | "libs") {
                if let Some(name) = rest.split('/').next() {
                    candidate_paths.entry(format!("{}/{}", root, name)).or_insert_with(|| format!("Conventional {} workspace", root));
                }
            }
        }
        if path.ends_with("/package.json") || path.ends_with("/pyproject.toml") || path.ends_with("/go.mod") || path.ends_with("/Cargo.toml") {
            if let Some(parent) = path.rsplit_once('/').map(|(p, _)| p.to_string()) {
                candidate_paths.entry(parent).or_insert_with(|| "Nested manifest".to_string());
            }
        }
    }

    if package_json_count > 1 { indicators.insert(format!("{} package.json files", package_json_count)); }
    if pyproject_count > 1 { indicators.insert(format!("{} pyproject.toml files", pyproject_count)); }
    if gomod_count > 1 { indicators.insert(format!("{} go.mod files", gomod_count)); }

    for file in files {
        if file.path == "package.json" && file.content.contains("\"workspaces\"") {
            indicators.insert("package.json workspaces".to_string());
        }
        if file.path == "Cargo.toml" && file.content.contains("[workspace]") {
            indicators.insert("Cargo workspace".to_string());
        }
    }

    let mut workspace_candidates: Vec<WorkspaceCandidate> = candidate_paths
        .into_iter()
        .map(|(path, reason)| WorkspaceCandidate {
            name: path.rsplit('/').next().unwrap_or(&path).to_string(),
            stack: infer_stack_from_path(&path, tree),
            path,
            reason,
        })
        .collect();
    workspace_candidates.sort_by(|a, b| a.path.cmp(&b.path));

    let detected = !indicators.is_empty() || workspace_candidates.len() > 1;
    let selected_scope = if detected { choose_default_workspace_scope_from_candidates(&workspace_candidates) } else { None };
    let scope_reason = if let Some(scope) = &selected_scope {
        format!("Monorepo detected; analysis scoped to {} by default", scope)
    } else if detected {
        "Monorepo detected; no single default workspace selected, using bounded global analysis".to_string()
    } else {
        "Single-project repository shape detected".to_string()
    };

    MonorepoInfo {
        detected,
        indicators: indicators.into_iter().collect(),
        workspace_candidates,
        selected_scope,
        scope_reason,
    }
}

pub fn choose_default_workspace_scope(info: &MonorepoInfo) -> Option<String> {
    choose_default_workspace_scope_from_candidates(&info.workspace_candidates)
}

fn choose_default_workspace_scope_from_candidates(candidates: &[WorkspaceCandidate]) -> Option<String> {
    candidates
        .iter()
        .find(|c| c.path.starts_with("apps/") || c.path.starts_with("services/"))
        .or_else(|| candidates.first())
        .map(|c| c.path.clone())
}

fn infer_stack_from_path(path: &str, tree: &[RepoFile]) -> Vec<String> {
    let prefix = format!("{}/", path.trim_matches('/'));
    let mut stack = BTreeSet::new();
    for file in tree.iter().filter(|f| f.path.starts_with(&prefix)) {
        if file.path.ends_with("package.json") { stack.insert("javascript/typescript".to_string()); }
        if file.path.ends_with("pyproject.toml") || file.path.ends_with("requirements.txt") { stack.insert("python".to_string()); }
        if file.path.ends_with("go.mod") { stack.insert("go".to_string()); }
        if file.path.ends_with("Cargo.toml") { stack.insert("rust".to_string()); }
    }
    stack.into_iter().collect()
}

fn extension(path: &str) -> Option<String> {
    path.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase())
}
