mod ai;
mod analysis_cache;
mod analyze;
mod architecture;
mod audit;
mod audit_rules;
mod bedrock;
mod commands;
mod config;
mod entry_tracer;
mod github;
mod onboarding_pack;
#[allow(dead_code)]
mod pr_parser;
mod prompts;
mod repo_analyzer;
mod repo_parser;
mod symbol_extractor;
pub mod types;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::analyze_repo,
            commands::detect_repo_scopes,
            commands::run_audit,
            commands::export_onboarding_pack,
            commands::load_cached_analysis,
            commands::get_settings,
            commands::save_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
