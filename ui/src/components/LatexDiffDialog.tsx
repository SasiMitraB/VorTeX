// "Compare versions": pick two versions, get a change-tracked PDF (latexdiff).

import { useEffect, useState } from "react";
import { Loader2 } from "lucide-react";
import { commands, type LatexDiffOptions } from "../bindings";
import { activeTab, get } from "../store";
import { openPdf, saveAllFiles } from "../actions";
import { Dialog, closeDialog } from "./Dialog";

const activePath = () => {
  const t = activeTab(get());
  return t?.kind === "text" ? t.path : null;
};

export function LatexDiffDialog() {
  const [opts, setOpts] = useState<LatexDiffOptions | null>(null);
  const [oldRev, setOld] = useState<string | null>(null);
  const [newRev, setNew] = useState<string | null>(null); // null = working copy
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    commands
      .latexdiffOptions(activePath())
      .then((o) => {
        setOpts(o);
        setOld(o.commits[0]?.hash ?? null);
      })
      .catch((e) => setError(String(e)));
  }, []);

  async function generate() {
    if (!oldRev) return;
    setRunning(true);
    setError(null);
    try {
      // The working copy is read from disk.
      if (newRev === null) await saveAllFiles();
      const pdf = await commands.latexdiff(activePath(), oldRev, newRev);
      closeDialog();
      await openPdf(pdf, "right");
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  }

  const list = (value: string | null, onPick: (h: string | null) => void, withWorking: boolean) => (
    <div className="commit-list">
      {withWorking && (
        <div className={`commit-pick ${value === null ? "on" : ""}`} onClick={() => onPick(null)}>
          <div>Working copy</div>
          <div className="muted small">Your files on disk, including unsaved edits once saved</div>
        </div>
      )}
      {opts?.commits.map((c) => (
        <div key={c.hash} className={`commit-pick ${value === c.hash ? "on" : ""}`} onClick={() => onPick(c.hash)}>
          <div className="ellipsis">{c.subject}</div>
          <div className="muted small">
            <span className="mono">{c.shortHash}</span> · {c.author} · {c.relativeDate}
          </div>
        </div>
      ))}
    </div>
  );

  return (
    <Dialog
      title="Compare Versions"
      wide
      footer={
        <>
          <span className="muted small ellipsis">{opts?.mainRel ? `Main document: ${opts.mainRel}` : ""}</span>
          <span className="spacer" />
          <button onClick={closeDialog}>Cancel</button>
          <button className="primary" disabled={!oldRev || running || !opts?.mainRel} onClick={generate}>
            {running && <Loader2 size={13} className="spin" />} {running ? "Generating…" : "Generate PDF"}
          </button>
        </>
      }
    >
      {!opts && !error && <div className="empty-state">Loading history…</div>}
      {opts && opts.commits.length === 0 && <div className="empty-state">This project has no commits yet.</div>}
      {opts && opts.commits.length > 0 && (
        <div className="two-columns">
          <div>
            <h3>Old version</h3>
            {list(oldRev, (h) => setOld(h), false)}
          </div>
          <div>
            <h3>New version</h3>
            {list(newRev, setNew, true)}
          </div>
        </div>
      )}
      {error && <pre className="error-box">{error}</pre>}
    </Dialog>
  );
}
