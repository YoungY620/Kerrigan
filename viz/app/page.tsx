'use client';

import { useEffect, useState, useCallback } from 'react';
import dynamic from 'next/dynamic';

import type { Task, DagResponse } from '@/lib/api';
import { fetchTasks, fetchDag } from '@/lib/api';
import TaskList from './components/TaskList';
import Legend from './components/Legend';

// DagCard uses ReactFlow which requires browser APIs → load client-only
const DagCard = dynamic(() => import('./components/DagCard'), { ssr: false });

export default function Home() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [search, setSearch] = useState('');
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [dagData, setDagData] = useState<Map<string, DagResponse>>(new Map());
  const [loadingDags, setLoadingDags] = useState<Set<string>>(new Set());
  const [hiddenStatuses, setHiddenStatuses] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string | null>(null);

  // Load task list on mount
  useEffect(() => {
    fetchTasks()
      .then(setTasks)
      .catch((e) => setError(String(e)));
  }, []);

  // Load DAG data when a task is selected
  const handleToggle = useCallback(
    async (id: string) => {
      setSelectedIds((prev) => {
        const next = new Set(prev);
        if (next.has(id)) {
          next.delete(id);
        } else {
          next.add(id);
        }
        return next;
      });

      // Fetch dag if not already cached
      if (!dagData.has(id)) {
        setLoadingDags((prev) => new Set(prev).add(id));
        try {
          const data = await fetchDag(id);
          setDagData((prev) => new Map(prev).set(id, data));
        } catch (e) {
          console.error('Failed to fetch DAG:', e);
        } finally {
          setLoadingDags((prev) => {
            const next = new Set(prev);
            next.delete(id);
            return next;
          });
        }
      }
    },
    [dagData],
  );

  const handleClose = useCallback((id: string) => {
    setSelectedIds((prev) => {
      const next = new Set(prev);
      next.delete(id);
      return next;
    });
  }, []);

  const handleToggleStatus = useCallback((status: string) => {
    setHiddenStatuses((prev) => {
      const next = new Set(prev);
      if (next.has(status)) next.delete(status);
      else next.add(status);
      return next;
    });
  }, []);

  // Ordered list of selected tasks (preserving order from tasks array)
  const selectedTasks = tasks.filter((t) => selectedIds.has(t.id));

  return (
    <div className="flex h-screen overflow-hidden bg-slate-100">
      {/* Left sidebar */}
      <aside className="w-72 shrink-0 flex flex-col bg-white border-r border-slate-200 overflow-hidden">
        <div className="px-4 py-3 border-b border-slate-200 bg-slate-800">
          <h1 className="text-sm font-bold text-white tracking-wide">Kerrigan</h1>
          <p className="text-[10px] text-slate-400">DAG Visualizer</p>
        </div>
        {error ? (
          <div className="p-4 text-xs text-red-600">{error}</div>
        ) : (
          <TaskList
            tasks={tasks}
            search={search}
            onSearchChange={setSearch}
            selectedIds={selectedIds}
            onToggle={handleToggle}
          />
        )}
      </aside>

      {/* Main content */}
      <main className="flex-1 flex flex-col overflow-hidden">
        <Legend hiddenStatuses={hiddenStatuses} onToggle={handleToggleStatus} />

        <div className="flex-1 overflow-y-auto p-4">
          {selectedIds.size === 0 ? (
            <div className="flex items-center justify-center h-full text-slate-400 text-sm">
              Select tasks from the sidebar to view their DAGs
            </div>
          ) : (
            <div className="grid gap-4" style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(680px, 1fr))' }}>
              {selectedTasks.map((task) => {
                const data = dagData.get(task.id);
                const loading = loadingDags.has(task.id);
                return (
                  <div key={task.id}>
                    {loading || !data ? (
                      <div className="rounded-xl border border-slate-200 bg-white p-6 text-center text-sm text-slate-400 shadow-sm">
                        {loading ? 'Loading DAG…' : 'Failed to load'}
                      </div>
                    ) : (
                      <DagCard
                        data={data}
                        hiddenStatuses={hiddenStatuses}
                        onClose={() => handleClose(task.id)}
                      />
                    )}
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </main>
    </div>
  );
}
