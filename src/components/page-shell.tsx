import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

export function PageShell({
  title,
  description,
  eyebrow,
  action,
  children,
  className,
}: {
  title: ReactNode;
  description?: ReactNode;
  /** Mono caption rendered above the title (e.g. "Workspace · Home"). */
  eyebrow?: ReactNode;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={cn("veyra-page", className)}>
      <header className="shrink-0 flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div className="min-w-0">
          {eyebrow ? (
            <div className="veyra-eyebrow mb-2">{eyebrow}</div>
          ) : null}
          <h1 className="text-[1.65rem] font-medium leading-[1.08] tracking-[-0.028em] text-foreground">
            {title}
          </h1>
          {description ? (
            <p className="mt-1.5 text-[0.9rem] leading-5 text-muted-foreground">{description}</p>
          ) : null}
        </div>
        {action ? <div className="flex shrink-0 items-center gap-2">{action}</div> : null}
      </header>
      {children}
    </section>
  );
}

export function Panel({
  title,
  description,
  eyebrow,
  action,
  children,
  className,
}: {
  title?: ReactNode;
  description?: ReactNode;
  /** Mono caption rendered above the title (e.g. "01 · Capture · STT"). */
  eyebrow?: ReactNode;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section className={cn("veyra-panel", className)}>
      {(title || description || action || eyebrow) && (
        <div className="mb-4 flex items-start justify-between gap-4">
          <div className="min-w-0">
            {eyebrow ? <div className="veyra-eyebrow mb-1.5">{eyebrow}</div> : null}
            {title ? (
              <h2 className="text-[1.05rem] font-semibold tracking-[-0.018em] text-foreground">
                {title}
              </h2>
            ) : null}
            {description ? (
              <p className="mt-1 text-xs leading-5 text-muted-foreground">{description}</p>
            ) : null}
          </div>
          {action ? <div className="shrink-0">{action}</div> : null}
        </div>
      )}
      {children}
    </section>
  );
}

export function Toolbar({
  children,
  className,
}: {
  children: ReactNode;
  className?: string;
}) {
  return <div className={cn("veyra-toolbar", className)}>{children}</div>;
}
