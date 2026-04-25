'use client';

import { ReactFlow, Background, Controls, MiniMap } from '@xyflow/react';
import '@xyflow/react/dist/style.css';
import { useMemo } from 'react';

import type { DagResponse } from '@/lib/api';
import { formatDate } from '@/lib/api';
import { buildFlowGraph } from '@/lib/layout';
import DagNode from './DagNode';

const nodeTypes = { dagNode: DagNode };

interface Props {
  data: DagResponse;
  hiddenStatuses: Set<string>;
  onClose: () => void;
}

export default function DagCard({ data, hiddenStatuses, onClose }: Props) {
  const { task, nodes: vizNodes, edges: vizEdges } = data;

  const { nodes, edges } = useMemo(
    () => buildFlowGraph(vizNodes, vizEdges, hiddenStatuses),
    [vizNodes, vizEdges, hiddenStatuses],
  );

  // Count statuses
  const counts = vizNodes.reduce<Record<string, number>>((acc, n) => {
    acc[n.status] = (acc[n.status] ?? 0) + 1;
    return acc;
  }, {});

  const statusBadges: Record<string, { bg: string; text: string }> = {
    succeeded: { bg: '#dcfce7', text: '#166534' },
    failed:    { bg: '#fee2e2', text: '#991b1b' },
    running:   { bg: '#fef3c7', text: '#92400e' },
    ready:     { bg: '#dbeafe', text: '#1e40af' },
    pending:   { bg: '#f1f5f9', text: '#475569' },
  };

  return (
    <div className="flex flex-col rounded-xl border border-slate-200 bg-white shadow-sm overflow-hidden">
      {/* Card header */}
      <div className="flex items-start justify-between gap-2 px-4 py-3 border-b border-slate-100 bg-slate-50">
        <div className="min-w-0 flex-1">
          <p
            className="text-sm font-semibold text-slate-800 leading-snug break-words"
            title={task.summary}
          >
            {task.summary.length > 80 ? task.summary.slice(0, 77) + '…' : task.summary}
          </p>
          <p className="text-xs text-slate-400 mt-0.5 font-mono">
            {formatDate(task.created_at)} · {task.id.slice(0, 8)}
          </p>
          <div className="flex flex-wrap gap-1 mt-1.5">
            {Object.entries(counts).map(([st, cnt]) => {
              const style = statusBadges[st] ?? { bg: '#f1f5f9', text: '#475569' };
              return (
                <span
                  key={st}
                  className="text-[10px] px-1.5 py-0.5 rounded font-medium"
                  style={{ background: style.bg, color: style.text }}
                >
                  {cnt} {st}
                </span>
              );
            })}
          </div>
        </div>
        <button
          onClick={onClose}
          className="text-slate-400 hover:text-slate-700 text-lg leading-none mt-0.5 shrink-0"
          title="Close"
        >
          ×
        </button>
      </div>

      {/* React Flow canvas */}
      <div style={{ height: 480 }}>
        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          fitView
          fitViewOptions={{ padding: 0.15 }}
          minZoom={0.2}
          maxZoom={2}
          proOptions={{ hideAttribution: true }}
        >
          <Background color="#e2e8f0" gap={20} />
          <Controls />
          <MiniMap
            nodeColor={(n) => {
              const status = (n.data as { status: string }).status;
              const colors: Record<string, string> = {
                pending: '#9ca3af',
                ready: '#3b82f6',
                running: '#f59e0b',
                succeeded: '#22c55e',
                failed: '#ef4444',
              };
              return colors[status] ?? '#94a3b8';
            }}
            style={{ height: 80 }}
          />
        </ReactFlow>
      </div>
    </div>
  );
}
