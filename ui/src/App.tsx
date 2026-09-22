import { useEffect } from "react";
import { effectiveTheme, useApp } from "./store";
import { init, report } from "./actions";
import { Workspace } from "./components/Workspace";
import { ProjectSelector } from "./components/ProjectSelector";

export function App() {
  const view = useApp((s) => s.view);
  const ready = useApp((s) => s.settings !== null);
  const theme = useApp((s) => effectiveTheme(s));

  useEffect(() => {
    init().catch(report);
  }, []);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  if (!ready) return <div className="boot" data-tauri-drag-region />;
  return view === "workspace" ? <Workspace /> : <ProjectSelector />;
}
