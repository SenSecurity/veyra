import { useEffect, useState } from "react";
import { useNavigate, useRouterState } from "@tanstack/react-router";
import { ipc } from "@/lib/tauri";

export function useWizardGate() {
  const navigate = useNavigate();
  const pathname = useRouterState({ select: (s) => s.location.pathname });
  const [completed, setCompleted] = useState<boolean | null>(null);

  useEffect(() => {
    const onComplete = () => setCompleted(true);
    window.addEventListener("veyra:wizard-complete", onComplete);
    return () => window.removeEventListener("veyra:wizard-complete", onComplete);
  }, []);

  useEffect(() => {
    void ipc
      .wizardStatus()
      .then((status) => {
        setCompleted(status.completed);
        if (!status.completed && pathname !== "/wizard") void navigate({ to: "/wizard" });
      })
      .catch(() => {
        setCompleted(false);
        if (pathname !== "/wizard") void navigate({ to: "/wizard" });
      });
  }, [navigate, pathname]);

  return { completed };
}

