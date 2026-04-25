export interface Task {
  id: string;
  summary: string;
  workdir: string;
  created_at: string; // Unix timestamp string
}

export interface VizNode {
  id: string;
  summary: string;
  status: 'pending' | 'ready' | 'running' | 'succeeded' | 'failed';
  result_summary?: string;
  failure_reason?: string;
}

export interface VizEdge {
  predecessor_id: string;
  successor_id: string;
}

export interface DagResponse {
  task: Task;
  nodes: VizNode[];
  edges: VizEdge[];
}

export async function fetchTasks(): Promise<Task[]> {
  const res = await fetch('/api/tasks');
  if (!res.ok) throw new Error('Failed to fetch tasks');
  return res.json();
}

export async function fetchDag(taskId: string): Promise<DagResponse> {
  const res = await fetch(`/api/tasks/${taskId}`);
  if (!res.ok) throw new Error(`Failed to fetch DAG for ${taskId}`);
  return res.json();
}

export function formatDate(unixStr: string): string {
  const ts = parseInt(unixStr, 10);
  if (isNaN(ts)) return unixStr;
  const d = new Date(ts * 1000);
  const mm = String(d.getMonth() + 1).padStart(2, '0');
  const dd = String(d.getDate()).padStart(2, '0');
  const hh = String(d.getHours()).padStart(2, '0');
  const min = String(d.getMinutes()).padStart(2, '0');
  return `${mm}-${dd} ${hh}:${min}`;
}
