import { useMemo } from "react";
import type { AuditFinding, AuditResult } from "../types";

type AuditStatus = "idle" | "running" | "done" | "error";

interface AuditViewProps {
  auditStatus: AuditStatus;
  auditResult: AuditResult | null;
  auditError: string | null;
  onRunAudit: () => void;
}

export function AuditView({ auditStatus, auditResult, auditError, onRunAudit }: AuditViewProps) {
  const filtered = useMemo(() => auditResult?.findings ?? [], [auditResult]);

  const grouped = useMemo(() => {
    const map = new Map<string, AuditFinding[]>();
    for (const finding of filtered) {
      const list = map.get(finding.pillar) ?? [];
      list.push(finding);
      map.set(finding.pillar, list);
    }
    return [...map.entries()].sort(([a], [b]) => a.localeCompare(b));
  }, [filtered]);

  async function copyAuditJson() {
    if (!auditResult) return;
    await navigator.clipboard?.writeText(JSON.stringify(auditResult, null, 2));
  }

  return (
    <div className="panel-content">
      <div className="audit-header">
        <div>
          <h2>Well-Architected + UX/DX Audit</h2>
          <p className="muted">Deterministic rules first, optional LLM context. Findings sorted P0–P5.</p>
        </div>
        <div className="audit-actions">
          <button
            className="primary-button"
            onClick={onRunAudit}
            disabled={auditStatus === "running"}
          >
            {auditStatus === "running" ? (
              <span className="btn-running"><span className="loading-view-spinner" /> Auditing in background…</span>
            ) : auditStatus === "done" ? "Re-run Audit" : "Run Audit"}
          </button>
          {auditResult && <button className="secondary-button" onClick={copyAuditJson}>Copy audit.json</button>}
        </div>
      </div>

      {auditStatus === "running" && (
        <div className="audit-progress-banner">
          <span className="loading-view-spinner" />
          <span>Audit running in the background. Switch tabs freely — results will appear here when done.</span>
        </div>
      )}

      {auditError && <div className="warning-box error-text">{auditError}</div>}
      {auditResult?.warnings.map((warning, idx) => (
        <div className="warning-box" key={idx}>{warning}</div>
      ))}

      {auditResult && (
        <>
          <div className="audit-summary">
            {(["p0", "p1", "p2", "p3", "p4", "p5"] as const).map((key) => (
              <span key={key} className={`priority-pill static priority-pill-${key}`}>
                {key.toUpperCase()} · {auditResult.summary[key]}
              </span>
            ))}
            {auditResult.summary.top_pillar && (
              <span className="muted">Top pillar: {auditResult.summary.top_pillar}</span>
            )}
          </div>

          {grouped.length === 0
            ? <p className="empty-state">No audit findings detected.</p>
            : grouped.map(([pillar, findings]) => (
              <section className="audit-group" key={pillar}>
                <h3>{pillar}</h3>
                {findings.map((finding, idx) => (
                  <FindingCard finding={finding} key={`${finding.file}:${finding.line}:${finding.title}:${idx}`} />
                ))}
              </section>
            ))
          }
        </>
      )}

      {auditStatus === "idle" && (
        <p className="empty-state">Run Audit to generate P0–P5 findings against AWS Well-Architected pillars and UX/DX.</p>
      )}
    </div>
  );
}

function FindingCard({ finding }: { finding: AuditFinding }) {
  async function copyRef() {
    await navigator.clipboard?.writeText(`${finding.file}:${finding.line}`);
  }

  return (
    <details className="audit-card" open={finding.priority === "P0" || finding.priority === "P1"}>
      <summary>
        <span className={`priority-badge priority-${finding.priority.toLowerCase()}`}>{finding.priority}</span>
        <strong>{finding.title}</strong>
        <button className="link-button" onClick={(e) => { e.preventDefault(); copyRef(); }}>
          {finding.file}:{finding.line}
        </button>
      </summary>
      <div className="audit-card-body">
        <p>{finding.description}</p>
        <p><strong>Evidence:</strong> <code>{finding.evidence}</code></p>
        <p><strong>Impact:</strong> {finding.impact}</p>
        <p><strong>Recommendation:</strong> {finding.recommendation}</p>
        <div className="muted">
          Confidence: {finding.confidence} · Source: {finding.source}
          {finding.rule_override ? " · rule override" : ""}
        </div>
      </div>
    </details>
  );
}
