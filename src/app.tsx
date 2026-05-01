import type { PropsWithChildren } from "react";

export function App({ children }: PropsWithChildren) {
  return (
    <div className="min-h-screen bg-bg text-fg font-sans">{children}</div>
  );
}
