import type { ReactNode } from "react";

export function StatCard({
  label,
  value,
  icon,
}: {
  label: string;
  value: ReactNode;
  icon?: ReactNode;
}) {
  return (
    <div className="veyra-surface veyra-surface-hover rounded-xl border border-border bg-white/78 p-4">
      <div className="flex items-center justify-between gap-3">
        <p className="text-xs font-medium uppercase tracking-[0.08em] text-muted-foreground">{label}</p>
        {icon ? (
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-accent text-primary">
            {icon}
          </div>
        ) : null}
      </div>
      <div className="mt-3 text-2xl font-semibold tracking-tight text-foreground">{value}</div>
    </div>
  );
}
