use crate::config::app_config_dir;
use crate::types::{CachedAnalysisInfo, RepoAnalysis};
use chrono::Utc;
use std::fs;
use std::path::PathBuf;

fn cache_dir() -> PathBuf {
    app_config_dir().join("analyses")
}

fn key(repo_url: &str) -> String {
    repo_url
        .trim()
        .trim_end_matches('/')
        .replace("https://github.com/", "")
        .replace('/', "_")
        .replace(':', "_")
}

fn cache_path(repo_url: &str) -> PathBuf {
    cache_dir().join(format!("{}.json", key(repo_url)))
}

fn meta_path(repo_url: &str) -> PathBuf {
    cache_dir().join(format!("{}.meta.json", key(repo_url)))
}

pub fn load_cached_analysis(repo_url: &str) -> Option<RepoAnalysis> {
    let content = fs::read_to_string(cache_path(repo_url)).ok()?;
    serde_json::from_str(&content).ok()
}

pub fn save_cached_analysis(analysis: &RepoAnalysis) -> Result<(), String> {
    let dir = cache_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create analysis cache dir: {}", e))?;
    let json = serde_json::to_string(analysis).map_err(|e| format!("Failed to serialize analysis: {}", e))?;
    let path = cache_path(&analysis.repo_url);
    fs::write(&path, json).map_err(|e| format!("Failed to write analysis cache: {}", e))?;
    set_private_permissions(&path);

    let meta = CachedAnalysisInfo {
        repo_url: analysis.repo_url.clone(),
        repo_name: analysis.repo_name.clone(),
        analyzed_file_count: analysis.symbols.iter().map(|s| s.file.clone()).collect::<std::collections::BTreeSet<_>>().len(),
        cached_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    };
    let meta_json = serde_json::to_string(&meta).map_err(|e| format!("Failed to serialize analysis metadata: {}", e))?;
    let mpath = meta_path(&analysis.repo_url);
    fs::write(&mpath, meta_json).map_err(|e| format!("Failed to write analysis metadata: {}", e))?;
    set_private_permissions(&mpath);
    Ok(())
}

fn set_private_permissions(_path: &PathBuf) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        let _ = fs::set_permissions(_path, perms);
    }
}

