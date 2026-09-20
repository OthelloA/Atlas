export type ProgressStatus = "running" | "done";

export interface SymbolEntry {
  name: string;
  kind: string;
  file: string;
  line: number;
  signature: string;
  calls: string[];
}

export type CallTraceResolution = "resolved" | "ambiguous" | "cycle" | "depth_capped" | "unresolved";

export interface CallTraceNode {
  name: string;
  file: string;
  line: number;
  is_entry_point: boolean;
  resolution: CallTraceResolution;
  callees: CallTraceNode[];
}

export interface AnalysisProgress {
  step: number;
  total_steps: number;
  label: string;
  status: ProgressStatus;
  detail?: string;
  items_done?: number;
  items_total?: number;
}

export interface OnboardingPack {
  overview_md: string;
  start_here_md: string;
  symbols_json: string;
  call_trace_json: string;
  repo_analysis_json: string;
  warnings_json: string;
  claude_snippet_md: string;
}

export interface WorkspaceCandidate {
  path: string;
  name: string;
  stack: string[];
  reason: string;
}

export interface MonorepoInfo {
  detected: boolean;
  indicators: string[];
  workspace_candidates: WorkspaceCandidate[];
  selected_scope: string | null;
  scope_reason: string;
}

export interface ArchitectureEvidence {
  rule: string;
  kind: string;
  source_refs: string[];
  explanation: string;
}

export interface ArchitectureNode {
  id: string;
  name: string;
  node_type: string;
  c4_level: string;
  parent_id: string | null;
  is_entry_point: boolean;
  description: string;
  confidence: string;
  confidence_score: number;
  deterministic: boolean;
  source_refs: string[];
  evidence: ArchitectureEvidence[];
}

export interface ArchitectureEdge {
  source: string;
  target: string;
  relationship: string;
  confidence: string;
  confidence_score: number;
  deterministic: boolean;
  evidence: ArchitectureEvidence[];
}

export interface ArchitectureGraph {
  nodes: ArchitectureNode[];
  edges: ArchitectureEdge[];
  generated_from: string;
}

export interface RepoAnalysis {
  repo_url: string;
  repo_name: string;
  stack: string[];
  entry_points: string[];
  symbols: SymbolEntry[];
  call_trace: CallTraceNode[];
  architecture: ArchitectureGraph;
  overview_md: string;
  truncated: boolean;
  warnings: string[];
  onboarding_pack: OnboardingPack;
  monorepo: MonorepoInfo | null;
}

export interface ExportResult {
  path: string | null;
  files_written: string[];
  fallback_required: boolean;
  message: string;
}

export interface AuditResult {
  findings: AuditFinding[];
  summary: AuditSummary;
  warnings: string[];
}

export interface AuditFinding {
  pillar: string;
  priority: "P0" | "P1" | "P2" | "P3" | "P4" | "P5" | string;
  title: string;
  description: string;
  file: string;
  line: number;
  evidence: string;
  impact: string;
  recommendation: string;
  confidence: string;
  source: string;
  rule_override: boolean;
}

export interface AuditSummary {
  p0: number;
  p1: number;
  p2: number;
  p3: number;
  p4: number;
  p5: number;
  top_pillar: string | null;
}

export interface Settings {
  model: string;
  github_token: string;
  aws_profile: string;
  filter_older: boolean;
  filter_team: boolean;
}

// Legacy PR-review types remain exported so old, currently unrendered components still type-check
// until they are deleted in a dedicated cleanup pass.
export type RiskLevel = "critical" | "high" | "medium" | "low";
export type HighlightSeverity = "critical" | "warning" | "info";
export interface Highlight { start_line: number; end_line: number; severity: HighlightSeverity; comment: string; }
export interface FileDiff {
  path: string; classification: string; reason: string; category: string; risk_level: RiskLevel;
  diff_type: "modified" | "added" | "removed"; base_content: string; head_content: string;
  unified_diff: string; additions: number; deletions: number; highlights: Highlight[];
  hunk_scores: string[]; diff_hash: string;
}
export interface ChangeGroup { label: string; description: string; file_paths: string[]; }
export interface ReviewManifest {
  pr_title: string; pr_url: string; pr_number: number; base_ref: string; head_ref: string;
  base_sha: string; head_sha: string; summary: string; change_groups: ChangeGroup[]; files: FileDiff[];
}
export interface CommentAuthor { login: string; avatar_url: string; }
export interface ReviewComment { id: string; body: string; author: CommentAuthor; created_at: string; updated_at: string; url: string; }
export interface ReviewThread {
  id: string; is_resolved: boolean; is_outdated: boolean; path: string; line: number | null;
  original_line: number | null; diff_hunk: string; comments: ReviewComment[];
}
export type SidebarView = "groups" | "comments" | "category" | "tree";
export type DiffViewMode = "split" | "unified";
export type HunkSignificanceFilter = "all" | "high" | "medium" | "low";
export interface SearchMatch { filePath: string; lineNumber: number; lineContent: string; matchStart: number; matchLength: number; }
export type ReviewStatus = "approved" | "changes_requested" | "commented" | "dismissed" | "pending";
export interface CachedPrInfo { owner: string; repo: string; pr_number: number; pr_title: string; pr_url: string; head_sha: string; file_count: number; cached_at: string; }
export interface ReviewRequestItem {
  owner: string; repo: string; number: number; title: string; html_url: string; author: string;
  created_at: string; updated_at: string; draft: boolean; direct_request: boolean;
  my_review_status: ReviewStatus; unresolved_thread_count: number;
}
