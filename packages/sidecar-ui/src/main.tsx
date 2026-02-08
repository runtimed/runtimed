import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./index.css";

type SidecarGlobal = typeof globalThis & {
  onMessage?: (msg: unknown) => void;
  __sidecarPendingMessages?: unknown[];
};

const sidecarGlobal = globalThis as SidecarGlobal;

if (!sidecarGlobal.__sidecarPendingMessages) {
  sidecarGlobal.__sidecarPendingMessages = [];
}

if (typeof sidecarGlobal.onMessage !== "function") {
  sidecarGlobal.onMessage = (msg: unknown) => {
    sidecarGlobal.__sidecarPendingMessages?.push(msg);
  };
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>
);
