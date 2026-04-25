'use client';

import { Handle, Position } from '@xyflow/react';
import type { NodeProps } from '@xyflow/react';

interface DagNodeData {
  label: string;
  status: string;
  result_summary?: string;
  failure_reason?: string;
  dimmed: boolean;
  [key: string]: unknown;
}

export default function DagNode({ data }: NodeProps) {
  const d = data as DagNodeData;
  const tooltip = d.result_summary ?? d.failure_reason ?? null;

  return (
    <div className={`dag-node dag-node-${d.status}${d.dimmed ? ' dimmed' : ''}`}>
      <Handle type="target" position={Position.Top} style={{ background: '#94a3b8' }} />
      <span style={{ fontSize: 11, lineHeight: 1.4, display: 'block' }}>
        {d.label.length > 90 ? d.label.slice(0, 87) + '…' : d.label}
      </span>
      {tooltip && (
        <div className="dag-node-tooltip">{tooltip}</div>
      )}
      <Handle type="source" position={Position.Bottom} style={{ background: '#94a3b8' }} />
    </div>
  );
}
