import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import type { MonorepoInfo } from "../types";

interface RepoOpenerProps {
  onAnalyze: (repoUrl: string, selectedScope?: string | null) => void;
  onSettingsClick: () => void;
  disabled?: boolean;
}

export function RepoOpener({ onAnalyze, onSettingsClick, disabled }: RepoOpenerProps) {
  const [repoUrl, setRepoUrl] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [detecting, setDetecting] = useState(false);
  const [monorepo, setMonorepo] = useState<MonorepoInfo | null>(null);
  const [selectedScope, setSelectedScope] = useState<string>("__global__");

  function validate(value: string): boolean {
    const trimmed = value.trim();
    if (!trimmed) {
      setError("Enter a GitHub repository URL.");
      return false;
    }
    if (trimmed.includes("/pull/")) {
      setError("Paste a repository URL, not a pull request URL.");
      return false;
    }
    if (!/^(https?:\/\/github\.com\/[^/]+\/[^/]+|github\.com\/[^/]+\/[^/]+|[^/\s]+\/[^/\s]+)$/.test(trimmed)) {
      setError("Expected https://github.com/owner/repo or owner/repo.");
      return false;
    }
    setError(null);
    return true;
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const trimmed = repoUrl.trim();
    if (!validate(trimmed)) return;

    if (monorepo?.workspace_candidates.length) {
      onAnalyze(trimmed, selectedScope === "__global__" ? null : selectedScope);
      return;
    }

    setDetecting(true);
    setError(null);
    try {
      const detected = await invoke<MonorepoInfo>("detect_repo_scopes", { repoUrl: trimmed });
      if (detected.detected && detected.workspace_candidates.length > 0) {
        setMonorepo(detected);
        setSelectedScope(detected.selected_scope ?? detected.workspace_candidates[0]?.path ?? "__global__");
      } else {
        onAnalyze(trimmed, detected.selected_scope ?? null);
      }
    } catch (err) {
      setError(`Scope detection failed. You can still analyze the whole repo. ${String(err)}`);
      setMonorepo({ detected: false, indicators: [], workspace_candidates: [], selected_scope: null, scope_reason: "Detection failed" });
    } finally {
      setDetecting(false);
    }
  }

  function resetScopeDetection(value: string) {
    setRepoUrl(value);
    setMonorepo(null);
    setSelectedScope("__global__");
  }

  return (
    <form className="repo-opener" onSubmit={handleSubmit}>
      <input
        className="repo-input"
        value={repoUrl}
        onChange={(e) => resetScopeDetection(e.target.value)}
        placeholder="https://github.com/owner/repo"
        disabled={disabled || detecting}
      />
      <button className="primary-button" type="submit" disabled={disabled || detecting || !repoUrl.trim()}>
        {detecting ? "Detecting apps…" : monorepo?.workspace_candidates.length ? "Analyze selected" : "Analyze"}
      </button>
      <button className="icon-button" type="button" onClick={onSettingsClick} title="Settings">
        ⚙
      </button>
      {error && <div className="inline-error">{error}</div>}

      {monorepo?.workspace_candidates.length ? (
        <div className="scope-selector">
          <div>
            <strong>Monorepo apps/workspaces detected</strong>
            <div className="muted">Choose the app/package to scan, or scan the whole repo with bounded limits.</div>
          </div>
          <label className="scope-option">
            <input
              type="radio"
              name="scope"
              value="__global__"
              checked={selectedScope === "__global__"}
              onChange={() => setSelectedScope("__global__")}
            />
            <span>Whole repo, bounded</span>
          </label>
          {monorepo.workspace_candidates.map((candidate) => (
            <label className="scope-option" key={candidate.path}>
              <input
                type="radio"
                name="scope"
                value={candidate.path}
                checked={selectedScope === candidate.path}
                onChange={() => setSelectedScope(candidate.path)}
              />
              <span>
                <code>{candidate.path}</code>
                <span className="muted"> — {candidate.reason}{candidate.stack.length ? ` · ${candidate.stack.join(", ")}` : ""}</span>
              </span>
            </label>
          ))}
        </div>
      ) : null}
    </form>
  );
}
