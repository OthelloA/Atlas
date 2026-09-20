use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum FetchStatus {
    Running,
    Done,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FetchProgress {
    pub step: u8,
    pub total_steps: u8,
    pub label: String,
    pub status: FetchStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr_title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_done: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files_total: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Highlight {
    pub start_line: u64,
    pub end_line: u64,
    pub severity: String,
    pub comment: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileDiff {
    pub path: String,
    pub classification: String,
    pub reason: String,
    pub category: String,
    #[serde(default = "default_risk_level")]
    pub risk_level: String,
    pub diff_type: String,
    pub base_content: String,
    pub head_content: String,
    pub unified_diff: String,
    #[serde(default)]
    pub additions: u64,
    #[serde(default)]
    pub deletions: u64,
    #[serde(default)]
    pub highlights: Vec<Highlight>,
    #[serde(default)]
    pub hunk_scores: Vec<String>,
    #[serde(default)]
    pub diff_hash: String,
}

fn default_risk_level() -> String {
    "medium".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChangeGroup {
    pub label: String,
    pub description: String,
    pub file_paths: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReviewManifest {
    pub pr_title: String,
    pub pr_url: String,
    pub pr_number: u64,
    pub base_ref: String,
    pub head_ref: String,
    pub base_sha: String,
    pub head_sha: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub change_groups: Vec<ChangeGroup>,
    pub files: Vec<FileDiff>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub model: String,
    #[serde(default)]
    pub github_token: String,
    #[serde(default)]
    pub aws_profile: String,
    #[serde(default = "default_true")]
    pub filter_older: bool,
    #[serde(default = "default_true")]
    pub filter_team: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct FileClassification {
    pub path: String,
    pub classification: String,
    #[serde(default)]
    pub category: String,
    #[serde(default = "default_risk_level")]
    pub risk_level: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct HighlightResult {
    pub path: String,
    pub start_line: u64,
    pub end_line: u64,
    #[serde(default = "default_info")]
    pub severity: String,
    #[serde(default)]
    pub comment: String,
}

fn default_info() -> String {
    "info".to_string()
}

#[derive(Debug, Deserialize)]
pub struct PrMetadata {
    pub title: String,
    pub html_url: String,
    pub number: u64,
    pub base: PrRef,
    pub head: PrRef,
}

#[derive(Debug, Deserialize)]
pub struct PrRef {
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub sha: String,
}

#[derive(Debug, Deserialize)]
pub struct PrFile {
    pub filename: String,
    pub status: String,
    #[serde(default)]
    pub additions: u64,
    #[serde(default)]
    pub deletions: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommentAuthor {
    pub login: String,
    pub avatar_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReviewComment {
    pub id: String,
    pub body: String,
    pub author: CommentAuthor,
    pub created_at: String,
    pub updated_at: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReviewThread {
    pub id: String,
    pub is_resolved: bool,
    pub is_outdated: bool,
    pub path: String,
    pub line: Option<u64>,
    pub original_line: Option<u64>,
    pub diff_hunk: String,
    pub comments: Vec<ReviewComment>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrUpdateStatus {
    pub has_changes: bool,
    pub head_sha_changed: bool,
    pub comment_count_changed: bool,
    pub new_head_sha: Option<String>,
    pub new_comment_count: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReviewRequestItem {
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub title: String,
    pub html_url: String,
    pub author: String,
    pub created_at: String,
    pub updated_at: String,
    pub draft: bool,
    pub direct_request: bool,
    pub my_review_status: String,
    pub unresolved_thread_count: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepoAnalysis {
    pub repo_url: String,
    pub repo_name: String,
    pub stack: Vec<String>,
    pub entry_points: Vec<String>,
    pub symbols: Vec<SymbolEntry>,
    pub call_trace: Vec<CallTraceNode>,
    #[serde(default)]
    pub architecture: ArchitectureGraph,
    pub overview_md: String,
    pub truncated: bool,
    pub warnings: Vec<String>,
    pub onboarding_pack: OnboardingPack,
    pub monorepo: Option<MonorepoInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ArchitectureGraph {
    pub nodes: Vec<ArchitectureNode>,
    pub edges: Vec<ArchitectureEdge>,
    pub generated_from: String,
}

impl Default for ArchitectureGraph {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            generated_from: "none".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ArchitectureNode {
    pub id: String,
    pub name: String,
    pub node_type: String,
    pub description: String,
    pub confidence: String,
    pub source_refs: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ArchitectureEdge {
    pub source: String,
    pub target: String,
    pub relationship: String,
    pub confidence: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SymbolEntry {
    pub name: String,
    pub kind: String,
    pub file: String,
    pub line: u64,
    pub signature: String,
    pub calls: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CallTraceNode {
    pub name: String,
    pub file: String,
    pub line: u64,
    pub is_entry_point: bool,
    pub resolution: CallTraceResolution,
    pub callees: Vec<CallTraceNode>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum CallTraceResolution {
    Resolved,
    Ambiguous,
    Cycle,
    DepthCapped,
    Unresolved,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AnalysisProgress {
    pub step: u8,
    pub total_steps: u8,
    pub label: String,
    pub status: FetchStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items_done: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items_total: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct OnboardingPack {
    pub overview_md: String,
    pub start_here_md: String,
    pub symbols_json: String,
    pub call_trace_json: String,
    pub repo_analysis_json: String,
    pub warnings_json: String,
    pub claude_snippet_md: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepoFile {
    pub path: String,
    pub file_type: String,
    pub size: Option<u64>,
    pub sha: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileContent {
    pub path: String,
    pub content: String,
    pub truncated: bool,
    pub warning: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MonorepoInfo {
    pub detected: bool,
    pub indicators: Vec<String>,
    pub workspace_candidates: Vec<WorkspaceCandidate>,
    pub selected_scope: Option<String>,
    pub scope_reason: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkspaceCandidate {
    pub path: String,
    pub name: String,
    pub stack: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExportResult {
    pub path: Option<String>,
    pub files_written: Vec<String>,
    pub fallback_required: bool,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedAnalysisInfo {
    pub repo_url: String,
    pub repo_name: String,
    pub analyzed_file_count: usize,
    pub cached_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuditResult {
    pub findings: Vec<AuditFinding>,
    pub summary: AuditSummary,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuditFinding {
    pub pillar: String,
    pub priority: String,
    pub title: String,
    pub description: String,
    pub file: String,
    pub line: u64,
    pub evidence: String,
    pub impact: String,
    pub recommendation: String,
    pub confidence: String,
    pub source: String,
    pub rule_override: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AuditSummary {
    pub p0: u32,
    pub p1: u32,
    pub p2: u32,
    pub p3: u32,
    pub p4: u32,
    pub p5: u32,
    pub top_pillar: Option<String>,
}
