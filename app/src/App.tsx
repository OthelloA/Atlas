import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Header } from "./components/Header";
import { RepoOpener } from "./components/RepoOpener";
import { LoadingView } from "./components/LoadingView";
import { SettingsModal } from "./components/SettingsModal";
import { OverviewView } from "./components/OverviewView";
import { FuncTreeView } from "./components/FuncTreeView";
import { CallTraceView } from "./components/CallTraceView";
import { OnboardingPackExport } from "./components/OnboardingPackExport";
import { AuditView } from "./components/AuditView";
import type { AnalysisProgress, AuditResult, RepoAnalysis } from "./types";

type TabId = "overview" | "symbols" | "trace" | "audit";
type AuditStatus = "idle" | "running" | "done" | "error";

function App() {
  const [analysis, setAnalysis] = useState<RepoAnalysis | null>(null);
  const [activeTab, setActiveTab] = useState<TabId>("overview");
  const [status, setStatus] = useState<"idle" | "analyzing" | "analyzed" | "error">("idle");
  const [subject, setSubject] = useState("");
  const [progress, setProgress] = useState<AnalysisProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [auditStatus, setAuditStatus] = useState<AuditStatus>("idle");
  const [auditResult, setAuditResult] = useState<AuditResult | null>(null);
  const [auditError, setAuditError] = useState<string | null>(null);
  const unlistenRef = useRef<(() => void) | null>(null);

  useEffect(() => () => unlistenRef.current?.(), []);

  async function handleAnalyze(repoUrl: string, selectedScope?: string | null) {
    if (status === "analyzing") return;
    setStatus("analyzing");
    setSubject(repoUrl);
    setProgress(null);
    setError(null);
    setAnalysis(null);
    setActiveTab("overview");
    setAuditStatus("idle");
    setAuditResult(null);
    setAuditError(null);

    const unlisten = await listen<AnalysisProgress>("analysis-progress", (event) => {
      setProgress(event.payload);
    });
    unlistenRef.current = unlisten;

    try {
      const result = await invoke<RepoAnalysis>("analyze_repo", { repoUrl, selectedScope: selectedScope ?? null });
      setAnalysis(result);
      setStatus("analyzed");
    } catch (err) {
      setError(String(err));
      setStatus("error");
    } finally {
      unlisten();
      unlistenRef.current = null;
      setProgress(null);
    }
  }

  async function handleRunAudit(currentAnalysis: RepoAnalysis) {
    if (auditStatus === "running") return;
    setAuditStatus("running");
    setAuditError(null);
    try {
      const result = await invoke<AuditResult>("run_audit", { analysis: currentAnalysis });
      setAuditResult(result);
      setAuditStatus("done");
    } catch (err) {
      setAuditError(String(err));
      setAuditStatus("error");
    }
  }

  function reset() {
    unlistenRef.current?.();
    unlistenRef.current = null;
    setStatus("idle");
    setSubject("");
    setAnalysis(null);
    setError(null);
    setProgress(null);
    setAuditStatus("idle");
    setAuditResult(null);
    setAuditError(null);
  }

  return (
    <div className="app-shell">
      <Header analysis={analysis} onSettingsClick={() => setSettingsOpen(true)} onNewAnalysis={reset} />
      <main className="app-main">
        {status === "idle" && (
          <section className="hero-card">
            <h2>Understand any GitHub repo in minutes</h2>
            <p>Atlas builds a best-effort onboarding map from real repository structure: stack, symbols, approximate call trace, and exportable onboarding pack.</p>
            <RepoOpener onAnalyze={handleAnalyze} onSettingsClick={() => setSettingsOpen(true)} />
          </section>
        )}

        {status === "analyzing" && <LoadingView subject={subject} progress={progress} onCancel={reset} />}

        {status === "error" && (
          <section className="hero-card">
            <h2>Analysis failed</h2>
            <p className="error-text">{error}</p>
            <RepoOpener onAnalyze={handleAnalyze} onSettingsClick={() => setSettingsOpen(true)} />
          </section>
        )}

        {status === "analyzed" && analysis && (
          <section className="analysis-layout">
            <aside className="tab-sidebar">
              <button className={activeTab === "overview" ? "active" : ""} onClick={() => setActiveTab("overview")}>Overview</button>
              <button className={activeTab === "symbols" ? "active" : ""} onClick={() => setActiveTab("symbols")}>Func Tree</button>
              <button className={activeTab === "trace" ? "active" : ""} onClick={() => setActiveTab("trace")}>Call Trace</button>
              <button className={`tab-audit-btn ${activeTab === "audit" ? "active" : ""}`} onClick={() => setActiveTab("audit")}>
                <span>Audit</span>
                {auditStatus === "running" && <span className="tab-badge running"><span className="tab-spinner" /></span>}
                {auditStatus === "done" && auditResult && (
                  <span className="tab-badge done">
                    {(auditResult.summary.p0 + auditResult.summary.p1) > 0
                      ? `${auditResult.summary.p0 + auditResult.summary.p1} crit`
                      : `${auditResult.findings.length}`}
                  </span>
                )}
                {auditStatus === "error" && <span className="tab-badge error">!</span>}
              </button>
              <OnboardingPackExport repoName={analysis.repo_name} pack={analysis.onboarding_pack} />
            </aside>
            <section className="output-panel">
              {analysis.monorepo?.detected && (
                <div className="monorepo-banner">
                  <strong>Monorepo detected.</strong> {analysis.monorepo.scope_reason}
                  {analysis.monorepo.workspace_candidates.length > 0 && (
                    <div className="muted">Candidates: {analysis.monorepo.workspace_candidates.map((c) => c.path).join(", ")}</div>
                  )}
                </div>
              )}
              {activeTab === "overview" && <OverviewView analysis={analysis} />}
              {activeTab === "symbols" && <FuncTreeView symbols={analysis.symbols} />}
              {activeTab === "trace" && <CallTraceView nodes={analysis.call_trace} />}
              {activeTab === "audit" && (
                <AuditView
                  auditStatus={auditStatus}
                  auditResult={auditResult}
                  auditError={auditError}
                  onRunAudit={() => handleRunAudit(analysis)}
                />
              )}
            </section>
          </section>
        )}
      </main>
      <SettingsModal open={settingsOpen} onClose={() => setSettingsOpen(false)} />
    </div>
  );
}

export default App;
