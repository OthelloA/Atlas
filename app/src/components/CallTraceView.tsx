import type { CallTraceNode } from "../types";

export function CallTraceView({ nodes }: { nodes: CallTraceNode[] }) {
  return (
    <div className="panel-content">
      <p className="muted">Approximate call trace. Dynamic dispatch, re-exports, and same-name symbols may be ambiguous.</p>
      {nodes.length === 0 ? <p className="empty-state">No call trace roots detected.</p> : <div className="call-tree">{nodes.map(renderNode)}</div>}
    </div>
  );
}

function renderNode(node: CallTraceNode): React.ReactNode {
  return (
    <details className="call-node" key={`${node.file}:${node.line}:${node.name}`} open={node.is_entry_point}>
      <summary>
        <code>{node.name}</code>
        <span className={`badge resolution-${node.resolution}`}>{node.resolution}</span>
        <span className="muted">{node.file}{node.line ? `:${node.line}` : ""}</span>
      </summary>
      {node.callees.length > 0 && <div className="call-children">{node.callees.map(renderNode)}</div>}
    </details>
  );
}
