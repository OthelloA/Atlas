import type { RepoAnalysis } from "../types";

interface HeaderProps {
  analysis: RepoAnalysis | null;
  onSettingsClick: () => void;
  onNewAnalysis: () => void;
}

export function Header({ analysis, onSettingsClick, onNewAnalysis }: HeaderProps) {
  return (
    <header className="app-header">
      <div>
        <h1>Atlas</h1>
        {analysis ? (
          <div className="header-meta">
            <strong>{analysis.repo_name}</strong>
            {analysis.stack.length > 0 && <span>{analysis.stack.join(" · ")}</span>}
            {analysis.truncated && <span className="warning-text">truncated</span>}
            {analysis.warnings.length > 0 && <span>{analysis.warnings.length} warnings</span>}
          </div>
        ) : <div className="header-meta">Understand a repository in minutes</div>}
      </div>
      <div className="header-actions">
        {analysis && <button className="secondary-button" onClick={onNewAnalysis}>New analysis</button>}
        <button className="secondary-button" onClick={onSettingsClick}>Settings</button>
      </div>
    </header>
  );
}
