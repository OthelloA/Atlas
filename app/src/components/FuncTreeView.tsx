import { useMemo, useState } from "react";
import type { SymbolEntry } from "../types";

export function FuncTreeView({ symbols }: { symbols: SymbolEntry[] }) {
  const [query, setQuery] = useState("");
  const grouped = useMemo(() => {
    const map = new Map<string, SymbolEntry[]>();
    for (const symbol of symbols) {
      const haystack = `${symbol.name} ${symbol.file} ${symbol.signature} ${describeSymbol(symbol)}`.toLowerCase();
      if (query && !haystack.includes(query.toLowerCase())) continue;
      const list = map.get(symbol.file) ?? [];
      list.push(symbol);
      map.set(symbol.file, list);
    }
    return [...map.entries()].sort(([a], [b]) => a.localeCompare(b));
  }, [symbols, query]);

  async function copyRef(symbol: SymbolEntry) {
    await navigator.clipboard?.writeText(`${symbol.file}:${symbol.line}`);
  }

  return (
    <div className="panel-content">
      <div className="view-toolbar">
        <input className="filter-input" placeholder="Filter symbols" value={query} onChange={(e) => setQuery(e.target.value)} />
        <span className="muted">Best-effort symbols · {symbols.length}</span>
      </div>
      {grouped.length === 0 ? <p className="empty-state">No symbols detected.</p> : grouped.map(([file, items]) => (
        <details className="tree-group" key={file} open>
          <summary>{file} <span className="muted">({items.length})</span></summary>
          <div className="symbol-list">
            {items.map((symbol) => (
              <div className="symbol-row" key={`${symbol.file}:${symbol.line}:${symbol.name}`}>
                <div className="symbol-main">
                  <div className="symbol-title">
                    <span className="badge">{symbol.kind}</span>
                    <code>{symbol.name}</code>
                    <button className="link-button" onClick={() => copyRef(symbol)}>{symbol.file}:{symbol.line}</button>
                  </div>
                  <div className="symbol-description">{describeSymbol(symbol)}</div>
                  <div className="muted">{symbol.signature}</div>
                </div>
              </div>
            ))}
          </div>
        </details>
      ))}
    </div>
  );
}

function describeSymbol(symbol: SymbolEntry): string {
  const location = `${symbol.file}:${symbol.line}`;
  const calls = symbol.calls.length > 0
    ? ` Calls ${symbol.calls.slice(0, 5).join(", ")}${symbol.calls.length > 5 ? ", …" : ""}.`
    : " No direct calls detected.";
  if (symbol.kind === "class") return `Class exported from ${location}.${calls}`;
  if (symbol.kind === "function") return `Function exported from ${location}.${calls}`;
  return `Exported symbol from ${location}.${calls}`;
}
