import { useMemo, useState } from "react";
import type { ArchitectureGraph, ArchitectureNode } from "../types";

interface Props {
  graph: ArchitectureGraph;
}

const NODE_WIDTH = 280;
const NODE_HEIGHT = 92;
const COLUMN_GAP = 220;
const ROW_GAP = 52;

export function ArchitectureView({ graph }: Props) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [viewRootId, setViewRootId] = useState<string | null>(null);
  const [filter, setFilter] = useState("all");
  const [focusMode, setFocusMode] = useState<"neighbors" | "trace" | null>(null);
  const [deterministicOnly, setDeterministicOnly] = useState(true);
  const root = graph.nodes.find((node) => node.id === viewRootId) ?? null;
  const childIds = new Set(graph.nodes.filter((node) => node.parent_id === viewRootId).map((node) => node.id));

  const scopedNodes = useMemo(() => {
    if (!viewRootId) return graph.nodes;
    const connectedExternalIds = new Set(
      graph.edges.flatMap((edge) => childIds.has(edge.source) && edge.target.startsWith("external:") ? [edge.target] : []),
    );
    return graph.nodes.filter((node) => childIds.has(node.id) || connectedExternalIds.has(node.id));
  }, [graph.nodes, graph.edges, viewRootId, childIds]);
  const baseNodes = scopedNodes.filter((node) =>
    (filter === "all" || node.c4_level === filter) &&
    (!deterministicOnly || node.deterministic),
  );
  const baseIds = new Set(baseNodes.map((node) => node.id));
  const baseEdges = graph.edges.filter((edge) => baseIds.has(edge.source) && baseIds.has(edge.target));
  const focusIds = new Set(selectedId ? [selectedId] : []);
  if (selectedId && focusMode === "neighbors") {
    for (const edge of baseEdges) {
      if (edge.source === selectedId) focusIds.add(edge.target);
      if (edge.target === selectedId) focusIds.add(edge.source);
    }
  }
  if (selectedId && focusMode === "trace") {
    const frontier = [selectedId];
    for (let depth = 0; depth < 5 && frontier.length > 0; depth += 1) {
      const next: string[] = [];
      for (const edge of baseEdges) {
        if (frontier.includes(edge.source) && !focusIds.has(edge.target)) {
          focusIds.add(edge.target);
          next.push(edge.target);
        }
      }
      frontier.splice(0, frontier.length, ...next);
    }
  }
  const visibleNodes = selectedId && focusMode ? baseNodes.filter((node) => focusIds.has(node.id)) : baseNodes;
  const visibleIds = new Set(visibleNodes.map((node) => node.id));
  const visibleEdges = baseEdges.filter((edge) => visibleIds.has(edge.source) && visibleIds.has(edge.target));
  const positions = layoutNodes(visibleNodes);
  const selected = graph.nodes.find((node) => node.id === selectedId) ?? null;
  const width = Math.max(760, ...Object.values(positions).map((position) => position.x + NODE_WIDTH + 48));
  const height = Math.max(360, ...Object.values(positions).map((position) => position.y + NODE_HEIGHT + 48));
  const levels = ["context", "container", "component", "code"].filter((level) => graph.nodes.some((node) => node.c4_level === level));

  async function copyGraph() {
    await navigator.clipboard?.writeText(JSON.stringify(graph, null, 2));
  }

  return (
    <div className="panel-content architecture-view">
      <div className="view-toolbar architecture-toolbar">
        <div>
          <div className="architecture-breadcrumbs">
            <button className="link-button" onClick={() => { setViewRootId(null); setSelectedId(null); }}>Repository</button>
            {root && <><span className="muted">›</span><strong>{root.name}</strong></>}
          </div>
          <h2>{root ? `${root.name} components` : "Architecture Map"}</h2>
          <p className="muted">Click a node to focus its relationships. Double-click a container to explore what is inside it.</p>
        </div>
        <div className="architecture-actions">
          <select className="filter-input" value={filter} onChange={(event) => setFilter(event.target.value)} aria-label="Filter architecture nodes">
            <option value="all">All C4 levels</option>
            {levels.map((level) => <option key={level} value={level}>{level}</option>)}
          </select>
          <label className="architecture-toggle">
            <input type="checkbox" checked={deterministicOnly} onChange={(event) => setDeterministicOnly(event.target.checked)} />
            Deterministic only
          </label>
          {focusMode && <button className="secondary-button" onClick={() => { setFocusMode(null); setSelectedId(null); }}>Clear focus</button>}
          <button className="secondary-button" onClick={copyGraph}>Copy graph JSON</button>
        </div>
      </div>

      <div className="architecture-summary">
        <span className="badge">{visibleNodes.length} nodes</span>
        <span className="badge">{visibleEdges.length} relationships</span>
        <span className="muted">Generated from {graph.generated_from.toLowerCase()}.</span>
      </div>
      <div className="confidence-legend" aria-label="Confidence rubric">
        <strong>Confidence rubric</strong>
        <span><i className="legend-dot high" /> High ≥ 0.80: uniquely resolved AST/import evidence</span>
        <span><i className="legend-dot medium" /> Medium 0.60–0.79: deterministic but indirect evidence</span>
        <span><i className="legend-dot low" /> Low &lt; 0.60: ambiguous resolution or heuristic inference</span>
      </div>

      {visibleNodes.length === 0 ? (
        <p className="empty-state">No architecture nodes match this filter.</p>
      ) : (
        <div className="architecture-canvas" role="img" aria-label="Repository architecture graph">
          <svg viewBox={`0 0 ${width} ${height}`} width="100%" height={height}>
            <defs>
              <marker id="architecture-arrow" markerWidth="8" markerHeight="8" refX="7" refY="4" orient="auto">
                <path d="M0,0 L8,4 L0,8 z" fill="#64748b" />
              </marker>
            </defs>
            {visibleEdges.map((edge, index) => {
              const source = positions[edge.source];
              const target = positions[edge.target];
              if (!source || !target) return null;
              return (
                <g key={`${edge.source}:${edge.target}:${edge.relationship}:${index}`}>
                  <line
                    className={`architecture-edge edge-${edge.confidence} ${!selectedId || edge.source === selectedId || edge.target === selectedId ? "connected" : "dimmed"}`}
                    x1={source.x + NODE_WIDTH}
                    y1={source.y + NODE_HEIGHT / 2}
                    x2={target.x}
                    y2={target.y + NODE_HEIGHT / 2}
                    markerEnd="url(#architecture-arrow)"
                  />
                  <text className="architecture-edge-label" x={(source.x + NODE_WIDTH + target.x) / 2} y={(source.y + target.y) / 2 + 4}>
                    {edge.relationship}
                  </text>
                </g>
              );
            })}
            {visibleNodes.map((node) => {
              const position = positions[node.id];
              const isSelected = selectedId === node.id;
              const isConnected = !selectedId || isSelected || focusIds.has(node.id);
              const hasChildren = graph.nodes.some((child) => child.parent_id === node.id);
              return (
                <g
                  key={node.id}
                  className={`architecture-node node-${node.confidence} ${isSelected ? "selected" : ""} ${isConnected ? "connected" : "dimmed"}`}
                  transform={`translate(${position.x}, ${position.y})`}
                  onClick={() => { setSelectedId(node.id); setFocusMode("neighbors"); }}
                  onDoubleClick={() => { if (hasChildren) { setViewRootId(node.id); setSelectedId(null); } }}
                  role="button"
                  tabIndex={0}
                  aria-label={`Select ${node.name}`}
                  onKeyDown={(event) => { if (event.key === "Enter" || event.key === " ") setSelectedId(node.id); }}
                >
                  <rect width={NODE_WIDTH} height={NODE_HEIGHT} rx="10" />
                  <text className="architecture-node-type" x="14" y="22">{node.node_type} · {node.confidence} · {node.confidence_score.toFixed(2)}{node.is_entry_point ? " · ENTRY" : ""}</text>
                  <text className="architecture-node-name" x="14" y="49">{truncate(node.name, 34)}</text>
                  <text className="architecture-node-ref" x="14" y="71">{truncate(node.source_refs[0] ?? "inferred", 38)}</text>
                </g>
              );
            })}
          </svg>
        </div>
      )}

      {selected && <NodeDetails node={selected} graph={graph} onFocus={(mode) => setFocusMode(mode)} onOpen={(id) => { setViewRootId(id); setSelectedId(null); setFocusMode(null); }} />}
    </div>
  );
}

function NodeDetails({ node, graph, onFocus, onOpen }: { node: ArchitectureNode; graph: ArchitectureGraph; onFocus: (mode: "neighbors" | "trace") => void; onOpen: (id: string) => void }) {
  const relationships = graph.edges.filter((edge) => edge.source === node.id || edge.target === node.id);
  const children = graph.nodes.filter((child) => child.parent_id === node.id);
  return (
    <aside className="architecture-details">
      <div className="architecture-details-header">
        <div>
          <span className="badge">{node.node_type}</span>
          <h3>{node.name}</h3>
        </div>
        <span className={`badge confidence-${node.confidence}`}>{node.confidence} · {node.confidence_score.toFixed(2)}</span>
      </div>
      <p>{node.description}</p>
      <div className="architecture-detail-actions">
        <button className="secondary-button" onClick={() => onFocus("neighbors")}>Focus connections</button>
        <button className="secondary-button" onClick={() => onFocus("trace")}>Trace outgoing</button>
        {children.length > 0 && <button className="secondary-button architecture-open-button" onClick={() => onOpen(node.id)}>Open {children.length} contained component{children.length === 1 ? "" : "s"}</button>}
      </div>
      <p className="muted">{node.deterministic ? "Deterministic result — produced before any LLM inference." : "Inferred result — treat as a suggestion."}</p>
      <strong>Evidence</strong>
      <ul>
        {node.evidence.map((item, index) => (
          <li key={`${item.rule}:${index}`}><strong>{item.rule}</strong> · {item.explanation} <code>{item.source_refs.join(", ")}</code></li>
        ))}
      </ul>
      {relationships.length > 0 && (
        <>
          <strong>Relationships</strong>
          <ul>
            {relationships.map((edge, index) => (
              <li key={`${edge.relationship}:${index}`}>
                {edge.relationship} · {edge.confidence} ({edge.confidence_score.toFixed(2)}) · {edge.evidence.map((item) => item.explanation).join(" ")} <code>{edge.evidence.flatMap((item) => item.source_refs).join(", ")}</code>
              </li>
            ))}
          </ul>
        </>
      )}
    </aside>
  );
}

function layoutNodes(nodes: ArchitectureNode[]): Record<string, { x: number; y: number }> {
  const groups = new Map<string, ArchitectureNode[]>();
  for (const node of nodes) groups.set(node.node_type, [...(groups.get(node.node_type) ?? []), node]);
  const order = ["external", "container", "component", "code"];
  const types = [...groups.keys()].sort((a, b) => {
    const aIndex = order.indexOf(a);
    const bIndex = order.indexOf(b);
    return (aIndex < 0 ? order.length : aIndex) - (bIndex < 0 ? order.length : bIndex) || a.localeCompare(b);
  });
  const positions: Record<string, { x: number; y: number }> = {};
  types.forEach((type, column) => {
    groups.get(type)?.forEach((node, row) => {
      positions[node.id] = { x: 32 + column * (NODE_WIDTH + COLUMN_GAP), y: 32 + row * (NODE_HEIGHT + ROW_GAP) };
    });
  });
  return positions;
}

function truncate(value: string, max: number): string {
  return value.length > max ? `${value.slice(0, max - 1)}…` : value;
}
