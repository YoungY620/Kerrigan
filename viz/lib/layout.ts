import dagre from '@dagrejs/dagre';
import type { Node, Edge } from '@xyflow/react';
import type { VizNode, VizEdge } from './api';

const NODE_WIDTH = 220;
const NODE_HEIGHT = 48;

export function buildFlowGraph(
  vizNodes: VizNode[],
  vizEdges: VizEdge[],
  hiddenStatuses: Set<string>,
): { nodes: Node[]; edges: Edge[] } {
  // Build dagre graph for layout
  const g = new dagre.graphlib.Graph();
  g.setDefaultEdgeLabel(() => ({}));
  g.setGraph({ rankdir: 'TB', ranksep: 55, nodesep: 35, marginx: 20, marginy: 20 });

  vizNodes.forEach((n) => {
    g.setNode(n.id, { width: NODE_WIDTH, height: NODE_HEIGHT });
  });

  vizEdges.forEach((e) => {
    g.setEdge(e.predecessor_id, e.successor_id);
  });

  dagre.layout(g);

  const nodes: Node[] = vizNodes.map((n) => {
    const pos = g.node(n.id);
    const dimmed = hiddenStatuses.has(n.status);
    return {
      id: n.id,
      type: 'dagNode',
      position: {
        x: (pos?.x ?? 0) - NODE_WIDTH / 2,
        y: (pos?.y ?? 0) - NODE_HEIGHT / 2,
      },
      data: {
        label: n.summary,
        status: n.status,
        result_summary: n.result_summary,
        failure_reason: n.failure_reason,
        dimmed,
      },
    };
  });

  const edges: Edge[] = vizEdges.map((e, i) => ({
    id: `e${i}-${e.predecessor_id}-${e.successor_id}`,
    source: e.predecessor_id,
    target: e.successor_id,
    animated: false,
    style: { stroke: '#94a3b8', strokeWidth: 1.5 },
  }));

  return { nodes, edges };
}
