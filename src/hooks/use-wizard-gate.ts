import { useEffect } from "react";
import { useNavigate, useRouterState } from "@tanstack/react-router";
import { ipc } from "@/lib/tauri";

export function useWizardGate() {
  const navigate = useNavigate();
  const pathname = useRouterState({ select: (s) => s.location.pathname });

  useEffect(() => {
    if (pathname === "/wizard") return;
    void ipc
      .wizardStatus()
      .then((status) => {
        if (!status.completed) void navigate({ to: "/wizard" });
      })
      .catch(() => {});
  }, [navigate, pathname]);
}

