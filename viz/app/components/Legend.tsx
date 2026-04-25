'use client';

export const STATUS_CONFIG = {
  pending:   { label: 'Pending',   color: '#9ca3af', dot: 'bg-gray-400' },
  ready:     { label: 'Ready',     color: '#3b82f6', dot: 'bg-blue-500' },
  running:   { label: 'Running',   color: '#f59e0b', dot: 'bg-amber-400' },
  succeeded: { label: 'Succeeded', color: '#22c55e', dot: 'bg-green-500' },
  failed:    { label: 'Failed',    color: '#ef4444', dot: 'bg-red-500'   },
} as const;

export type StatusKey = keyof typeof STATUS_CONFIG;

interface Props {
  hiddenStatuses: Set<string>;
  onToggle: (status: string) => void;
}

export default function Legend({ hiddenStatuses, onToggle }: Props) {
  return (
    <div className="flex flex-wrap gap-3 px-4 py-3 bg-white border-b border-slate-200 sticky top-0 z-10">
      <span className="text-xs font-semibold text-slate-500 self-center mr-1">Show:</span>
      {(Object.entries(STATUS_CONFIG) as [StatusKey, typeof STATUS_CONFIG[StatusKey]][]).map(
        ([key, cfg]) => {
          const hidden = hiddenStatuses.has(key);
          return (
            <label
              key={key}
              className="flex items-center gap-1.5 cursor-pointer select-none text-xs"
            >
              <input
                type="checkbox"
                checked={!hidden}
                onChange={() => onToggle(key)}
                className="w-3.5 h-3.5 cursor-pointer"
              />
              <span
                className="w-2.5 h-2.5 rounded-full inline-block"
                style={{ background: cfg.color }}
              />
              <span className={hidden ? 'text-slate-400 line-through' : 'text-slate-700'}>
                {cfg.label}
              </span>
            </label>
          );
        },
      )}
    </div>
  );
}
