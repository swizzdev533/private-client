import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles/base.css";
import "./styles/layout.css";
import "./styles/components.css";
import "./styles/play.css";
import "./styles/mods.css";

if (typeof window !== "undefined") {
  document.addEventListener("selectstart", (e) => {
    const target = e.target as HTMLElement | null;
    if (
      target &&
      (target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable ||
        target.closest("input, textarea, [contenteditable='true']"))
    ) {
      return;
    }
    e.preventDefault();
  });

  document.addEventListener("dragstart", (e) => {
    const target = e.target as HTMLElement | null;
    if (
      target &&
      (target.tagName === "INPUT" ||
        target.tagName === "TEXTAREA" ||
        target.isContentEditable ||
        target.closest("input, textarea, [contenteditable='true']"))
    ) {
      return;
    }
    e.preventDefault();
  });
}

const root = document.getElementById("root");

if (!root) {
  throw new Error("Nie znaleziono głównego elementu aplikacji.");
}

createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
