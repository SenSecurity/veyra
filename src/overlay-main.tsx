import React from "react";
import ReactDOM from "react-dom/client";
import { OverlayApp } from "@/overlay/overlay-app";
import "./styles/tailwind.css";
import "./styles/globals.css";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <OverlayApp />
  </React.StrictMode>,
);

