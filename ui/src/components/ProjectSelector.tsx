import { convertFileSrc } from "@tauri-apps/api/core";
import { FolderOpen, FolderSearch, Loader2 } from "lucide-react";
import { useApp } from "../store";
import { chooseProjectsFolder, openFolderDialog, openProject } from "../actions";

export function ProjectSelector() {
  const folder = useApp((s) => s.settings?.projectsFolder);
  const projects = useApp((s) => s.projects);
  const loading = useApp((s) => s.projectsLoading);
  const current = useApp((s) => s.project?.path);
  const error = useApp((s) => s.projectsError);

  return (
    <div className="projects">
      <header className="toolbar" data-tauri-drag-region>
        <div className="toolbar-left" />
        <div className="toolbar-title" data-tauri-drag-region>
          VorTeX
        </div>
        <div className="toolbar-right">
          <button className="tool-btn" onClick={() => void openFolderDialog()}>
            <FolderOpen size={14} /> Open Folder…
          </button>
          <button className="tool-btn" onClick={() => void chooseProjectsFolder()}>
            <FolderSearch size={14} /> {folder ? "Change Folder…" : "Choose Projects Folder…"}
          </button>
        </div>
      </header>
      <main className="projects-body">
        <div className="projects-heading">
          <h1>Projects</h1>
          {folder && <span className="muted mono small ellipsis">{folder}</span>}
          {loading && <Loader2 size={14} className="spin muted" />}
        </div>
        {!folder && (
          <div className="empty-state big">
            <p>Choose the folder that holds your LaTeX projects. Each subfolder becomes a project.</p>
            <button className="primary" onClick={() => void chooseProjectsFolder()}>
              Choose Projects Folder…
            </button>
          </div>
        )}
        {folder && !loading && projects.length === 0 && <div className="empty-state">No project folders in here yet.</div>}
        <div className="project-grid">
          {projects.map((p) => (
            <button key={p.path} className={`project-card ${p.path === current ? "current" : ""}`} onClick={() => void openProject(p.path)}>
              <div className="thumb">
                {p.previewImage ? (
                  <img src={convertFileSrc(p.previewImage)} alt="" draggable={false} />
                ) : (
                  <span className="thumb-badge">LaTeX Project</span>
                )}
              </div>
              <div className="project-name ellipsis">{p.name}</div>
            </button>
          ))}
        </div>
        {error && <p className="err small">{error}</p>}
      </main>
    </div>
  );
}
