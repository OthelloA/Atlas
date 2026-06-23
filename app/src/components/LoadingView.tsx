import type { AnalysisProgress } from "../types";

interface LoadingViewProps {
  subject: string;
  progress: AnalysisProgress | null;
  onCancel: () => void;
}

export function LoadingView({ subject, progress, onCancel }: LoadingViewProps) {
  const total = progress?.total_steps ?? 7;
  const currentStep = progress?.step ?? 1;
  const currentStatus = progress?.status ?? "running";
  const completedSteps = currentStatus === "done" ? currentStep : currentStep - 1;
  const progressPercent = Math.max(0, Math.min(100, (completedSteps / total) * 100));

  return (
    <div className="loading-view">
      <div className="loading-view-title">Analyzing {subject}</div>
      <div className="loading-view-body">
        <div className="loading-view-step loading-view-step-active">
          <span className="loading-view-spinner" />
          <span className="loading-view-step-label">
            {progress?.label ?? "Starting analysis"}
            {progress?.detail && <span className="muted"> · {progress.detail}</span>}
            {progress?.items_total != null && (
              <span className="loading-view-file-count"> {progress.items_done ?? 0}/{progress.items_total}</span>
            )}
          </span>
        </div>
        <div className="loading-view-progress">
          <div className="loading-view-progress-fill" style={{ width: `${progressPercent}%` }} />
        </div>
        <button className="secondary-button" onClick={onCancel} style={{ marginTop: 16 }}>
          Cancel
        </button>
      </div>
    </div>
  );
}
