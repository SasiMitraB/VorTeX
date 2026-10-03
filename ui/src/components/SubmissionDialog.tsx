// "Prepare submission": a cleaned, verified source tarball for arXiv or a journal.

import { useEffect, useState } from "react";
import { CheckCircle2, Loader2, Tag, TriangleAlert } from "lucide-react";
import { commands, type SubmissionInfo, type SubmissionReport, type SubmissionTarget } from "../bindings";
import { activeTab, get } from "../store";
import { openPdf, saveAllFiles } from "../actions";
import { Dialog, closeDialog } from "./Dialog";

const activePath = () => {
  const t = activeTab(get());
  return t?.kind === "text" ? t.path : null;
};

const TARGETS: { id: SubmissionTarget; title: string; detail: string }[] = [
  { id: "arxiv", title: "arXiv", detail: "Sources and .bbl; arXiv doesn't run BibTeX" },
  { id: "journal", title: "Journal", detail: "Sources, .bbl, .bib and .bst" },
];

export function SubmissionDialog() {
  const [info, setInfo] = useState<SubmissionInfo | null>(null);
  const [target, setTarget] = useState<SubmissionTarget>("arxiv");
  const [fromHead, setFromHead] = useState(false);
  const [stripComments, setStripComments] = useState(true);
  const [pruneBib, setPruneBib] = useState(true);
  const [tag, setTag] = useState(true);
  const [running, setRunning] = useState(false);
  const [report, setReport] = useState<SubmissionReport | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    commands
      .submissionInfo(activePath())
      .then(setInfo)
      .catch((e) => setError(String(e)));
  }, []);

  async function prepare() {
    setRunning(true);
    setError(null);
    try {
      // The files on disk are packaged.
      if (!fromHead) await saveAllFiles();
      setReport(await commands.prepareSubmission(activePath(), {
          target,
          fromHead,
          stripComments,
          pruneBib,
          tag: tag && !!info?.inGit,
        }));
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  }

  if (report) {
    const folderName = report.folder.split("/").pop();
    return (
      <Dialog
        title="Submission Ready"
        wide
        footer={
          <>
            <span className="muted small ellipsis" title={report.folder}>
              submitted_versions/{folderName}
            </span>
            <span className="spacer" />
            <button onClick={() => void commands.openExternal(report.folder)}>Show in Finder</button>
            {report.pdf && (
              <button
                onClick={() => {
                  closeDialog();
                  void openPdf(report.pdf!, "right");
                }}
              >
                Open PDF
              </button>
            )}
            <button className="primary" onClick={closeDialog}>
              Done
            </button>
          </>
        }
      >
        <div className="submission-report">
          {report.verification.map((line) => (
            <div key={line} className="report-line ok">
              <CheckCircle2 size={14} /> <span>{line}</span>
            </div>
          ))}
          {report.tag && (
            <div className="report-line ok">
              <Tag size={14} />{" "}
              <span>
                Tagged the packaged commit as <span className="mono">{report.tag}</span>. Tags stay local until you
                push them (git push origin {report.tag}).
              </span>
            </div>
          )}
          {report.warnings.map((line) => (
            <div key={line} className="report-line warn">
              <TriangleAlert size={14} /> <span>{line}</span>
            </div>
          ))}
          {report.steps.length > 0 && (
            <>
              <h3>Clean-up</h3>
              <ul>
                {report.steps.map((s) => (
                  <li key={s}>{s}</li>
                ))}
              </ul>
            </>
          )}
          <details>
            <summary>
              {report.files.length} files · {(report.sizeBytes / 1e6).toFixed(1)} MB
            </summary>
            <ul className="mono small">
              {report.files.map((f) => (
                <li key={f}>{f}</li>
              ))}
            </ul>
          </details>
        </div>
      </Dialog>
    );
  }

  return (
    <Dialog
      title="Prepare Submission"
      footer={
        <>
          <span className="muted small ellipsis">{info?.mainRel ? `Main document: ${info.mainRel}` : ""}</span>
          <span className="spacer" />
          <button onClick={closeDialog}>Cancel</button>
          <button className="primary" disabled={running || !info?.mainRel} onClick={prepare}>
            {running && <Loader2 size={13} className="spin" />} {running ? "Building and checking…" : "Prepare"}
          </button>
        </>
      }
    >
      {info && !info.mainRel && <div className="empty-state">No main document found in this project.</div>}
      {info?.mainRel && (
        <div className="submission-form">
          <h3>Target</h3>
          <div className="choice-row">
            {TARGETS.map((t) => (
              <div key={t.id} className={`commit-pick ${target === t.id ? "on" : ""}`} onClick={() => setTarget(t.id)}>
                <div>{t.title}</div>
                <div className="muted small">{t.detail}</div>
              </div>
            ))}
          </div>
          {info.inGit && (
            <>
              <h3>Source</h3>
              <div className="choice-row">
                <div className={`commit-pick ${!fromHead ? "on" : ""}`} onClick={() => setFromHead(false)}>
                  <div>Files on disk</div>
                  <div className="muted small">Including unsaved edits once saved</div>
                </div>
                <div className={`commit-pick ${fromHead ? "on" : ""}`} onClick={() => setFromHead(true)}>
                  <div>Last commit</div>
                  <div className="muted small">Only what is committed</div>
                </div>
              </div>
            </>
          )}
          <h3>Clean-up</h3>
          <label className="check">
            <input type="checkbox" checked={stripComments} onChange={(e) => setStripComments(e.target.checked)} />
            Strip comments from .tex files
          </label>
          <label className="check">
            <input type="checkbox" checked={pruneBib} onChange={(e) => setPruneBib(e.target.checked)} />
            Keep only cited .bib entries
          </label>
          {info.inGit && (
            <label className="check">
              <input type="checkbox" checked={tag} onChange={(e) => setTag(e.target.checked)} />
              Tag the packaged commit in git (submitted/…)
            </label>
          )}
          <p className="muted small">
            Only files the build actually reads are packaged. The tarball is then compiled on its own and its PDF is
            compared with the original build. Output goes to submitted_versions/.
          </p>
        </div>
      )}
      {error && <pre className="error-box">{error}</pre>}
    </Dialog>
  );
}
