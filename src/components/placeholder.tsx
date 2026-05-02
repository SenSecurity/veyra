// Phase 3 placeholder component used while real route content is built in T4..T10.
export function Placeholder({ name }: { name: string }) {
  return (
    <div className="flex h-full min-h-[320px] items-center justify-center p-8">
      <div className="w-full max-w-md rounded-lg border border-border bg-card p-6 shadow-sm">
        <p className="text-xs font-medium uppercase text-muted-foreground">{name}</p>
        <h1 className="mt-2 text-xl font-semibold text-foreground">Workspace ready</h1>
        <div className="mt-5 space-y-2">
          <div className="h-2 rounded bg-muted" />
          <div className="h-2 w-5/6 rounded bg-muted" />
          <div className="h-2 w-2/3 rounded bg-muted" />
        </div>
      </div>
    </div>
  );
}
