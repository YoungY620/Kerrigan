'use client';

import type { Task } from '@/lib/api';
import { formatDate } from '@/lib/api';

interface Props {
  tasks: Task[];
  search: string;
  onSearchChange: (s: string) => void;
  selectedIds: Set<string>;
  onToggle: (id: string) => void;
}

export default function TaskList({ tasks, search, onSearchChange, selectedIds, onToggle }: Props) {
  const q = search.toLowerCase();
  const filtered = tasks.filter(
    (t) => t.id.toLowerCase().includes(q) || t.summary.toLowerCase().includes(q),
  );

  return (
    <div className="flex flex-col h-full">
      <div className="p-3 border-b border-slate-200">
        <input
          type="text"
          value={search}
          onChange={(e) => onSearchChange(e.target.value)}
          placeholder="Search tasks…"
          className="w-full text-sm px-3 py-1.5 border border-slate-300 rounded-md outline-none focus:border-blue-400 focus:ring-1 focus:ring-blue-200"
        />
        <p className="text-xs text-slate-400 mt-1.5">
          {filtered.length} / {tasks.length} tasks
          {selectedIds.size > 0 && ` · ${selectedIds.size} selected`}
        </p>
      </div>

      <div className="overflow-y-auto flex-1">
        {filtered.length === 0 ? (
          <p className="text-xs text-slate-400 px-4 py-6 text-center">No tasks match</p>
        ) : (
          filtered.map((task) => {
            const selected = selectedIds.has(task.id);
            return (
              <div
                key={task.id}
                onClick={() => onToggle(task.id)}
                className={`px-3 py-2.5 cursor-pointer border-b border-slate-100 hover:bg-slate-50 transition-colors ${
                  selected ? 'bg-blue-50 border-l-2 border-l-blue-400' : 'border-l-2 border-l-transparent'
                }`}
              >
                <div className="flex items-start gap-2">
                  <input
                    type="checkbox"
                    checked={selected}
                    onChange={() => onToggle(task.id)}
                    onClick={(e) => e.stopPropagation()}
                    className="mt-0.5 cursor-pointer shrink-0"
                  />
                  <div className="min-w-0">
                    <p
                      className="text-xs text-slate-700 leading-snug font-medium break-words"
                      style={{ wordBreak: 'break-word' }}
                    >
                      {task.summary}
                    </p>
                    <p className="text-[10px] text-slate-400 mt-0.5 font-mono">
                      {formatDate(task.created_at)} · {task.id.slice(0, 8)}
                    </p>
                  </div>
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}
