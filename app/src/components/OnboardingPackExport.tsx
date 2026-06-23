import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import type { ExportResult, OnboardingPack } from "../types";

interface Props {
  repoName: string;
  pack: OnboardingPack;
}

const PACK_FILES: Array<[keyof OnboardingPack, string, string]> = [
  ["overview_md", "overview.md", "Markdown overview"],
  ["start_here_md", "start-here.md", "Reading path and commands"],
  ["symbols_json", "symbols.json", "Symbol index"],
  ["call_trace_json", "call-trace.json", "Approximate call trace"],
  ["repo_analysis_json", "repo-analysis.json", "Full structured analysis"],
  ["warnings_json", "warnings.json", "Warnings and limits"],
  ["claude_snippet_md", "claude-snippet.md", "CLAUDE.md navigation snippet"],
];

export function OnboardingPackExport({ repoName, pack }: Props) {
  const [open, setOpen] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  async function copyFile(key: keyof OnboardingPack, name: string) {
    await navigator.clipboard?.writeText(pack[key]);
    setMessage(`${name} copied.`);
  }

  function downloadFile(key: keyof OnboardingPack, name: string) {
    const content = pack[key];
    const type = name.endsWith(".json") ? "application/json" : "text/markdown";
    const blob = new Blob([content], { type });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = name;
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  }

  function downloadAll() {
    for (const [key, name] of PACK_FILES) downloadFile(key, name);
    setMessage("Downloaded all pack files through the browser.");
  }

  async function copyStartHere() {
    await copyFile("start_here_md", "start-here.md");
  }

  async function saveToDownloads() {
    try {
      const result = await invoke<ExportResult>("export_onboarding_pack", { repoName, pack, targetDir: null });
      setMessage(result.path ? `${result.message}: ${result.path}` : result.message);
    } catch (err) {
      setMessage(`Save to Downloads failed. Use browser download instead. ${String(err)}`);
    }
  }

  return (
    <div className="sidebar-export">
      <button className="primary-button sidebar-export-main" onClick={() => setOpen((v) => !v)}>
        Export Pack
      </button>
      {open && (
        <div className="sidebar-export-menu">
          <button className="secondary-button" onClick={saveToDownloads}>Save all to Downloads</button>
          <button className="secondary-button" onClick={downloadAll}>Browser download all</button>
          <button className="secondary-button" onClick={copyStartHere}>Copy start-here.md</button>
          <details>
            <summary className="muted">Individual files</summary>
            <div className="artifact-list compact">
              {PACK_FILES.map(([key, name, description]) => (
                <div className="artifact-row compact" key={name}>
                  <div>
                    <code>{name}</code>
                    <div className="muted">{description}</div>
                  </div>
                  <div className="artifact-actions">
                    <button className="link-button" onClick={() => copyFile(key, name)}>Copy</button>
                    <button className="link-button" onClick={() => downloadFile(key, name)}>Download</button>
                  </div>
                </div>
              ))}
            </div>
          </details>
          {message && <div className="muted export-message">{message}</div>}
        </div>
      )}
    </div>
  );
}
