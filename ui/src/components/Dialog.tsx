import { useEffect, type ReactNode } from "react";
import { X } from "lucide-react";
import { set } from "../store";

export const closeDialog = () => set({ dialog: null });

export function Dialog({ title, children, footer, wide }: { title: string; children: ReactNode; footer?: ReactNode; wide?: boolean }) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && closeDialog();
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
  return (
    <div className="dialog-backdrop" onMouseDown={(e) => e.target === e.currentTarget && closeDialog()}>
      <div className={`dialog ${wide ? "wide" : ""}`} role="dialog" aria-label={title}>
        <header>
          <h2>{title}</h2>
          <button className="icon-btn" title="Close" onClick={closeDialog}>
            <X size={14} />
          </button>
        </header>
        <div className="dialog-body">{children}</div>
        {footer && <footer>{footer}</footer>}
      </div>
    </div>
  );
}
