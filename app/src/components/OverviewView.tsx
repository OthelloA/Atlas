import type { RepoAnalysis } from "../types";

export function OverviewView({ analysis }: { analysis: RepoAnalysis }) {
  return (
    <div className="panel-content">
      {analysis.warnings.length > 0 && (
        <div className="warning-box">
          <strong>Warnings</strong>
          <ul>
            {analysis.warnings.map((warning, idx) => <li key={idx}>{warning}</li>)}
          </ul>
        </div>
      )}
      <pre className="markdown-output">{analysis.overview_md || "No overview generated."}</pre>
    </div>
  );
}
