// Word counts for the status bar: the whole file, and the innermost section, environment or
// caption around the cursor. Counts prose only: commands, math, comments, the preamble and
// arguments such as \label{...} or \cite{...} are left out.

import type * as monaco from "./monaco";
import { LATEX } from "./latex";
import { pathOf } from "./models";
import { set } from "../store";

interface Region {
  label: string;
  start: number;
  end: number;
}

interface Analysis {
  /** Offsets where counted words start, ascending. */
  words: number[];
  bodyStart: number;
  bodyEnd: number;
  regions: Region[];
}

const SECTION_LEVELS: Record<string, number> = {
  part: -1,
  chapter: 0,
  section: 1,
  subsection: 2,
  subsubsection: 3,
  paragraph: 4,
  subparagraph: 5,
};

/** Commands whose (first) argument is a scope of its own. */
const SCOPE_COMMANDS: Record<string, string> = {
  caption: "Caption",
  footnote: "Footnote",
  title: "Title",
  thanks: "Thanks",
};

/** Environments that hold no prose: dropped entirely, never a scope. */
const SKIPPED_ENVS = new Set([
  "equation", "equation*", "align", "align*", "aligned", "alignat", "alignat*", "flalign",
  "flalign*", "gather", "gather*", "multline", "multline*", "eqnarray", "eqnarray*", "math",
  "displaymath", "split", "cases", "verbatim", "verbatim*", "lstlisting", "minted", "alltt",
  "Verbatim", "BVerbatim", "LVerbatim", "comment", "tikzpicture", "filecontents", "filecontents*",
]);

/** Leading brace arguments of an environment that are not prose (column specs, widths, ...). */
const ENV_ARGS: Record<string, number> = {
  tabular: 1, "tabular*": 2, tabularx: 2, tabulary: 2, array: 1, longtable: 1, minipage: 1,
  wrapfigure: 2, wraptable: 2, multicols: 1, subfigure: 1, thebibliography: 1,
};

/** Brace arguments of a command that are not prose. */
const SKIP_ARGS: Record<string, number> = {
  label: 1, ref: 1, eqref: 1, pageref: 1, autoref: 1, cref: 1, Cref: 1, nameref: 1, vref: 1,
  cite: 1, citep: 1, citet: 1, citealp: 1, citeauthor: 1, citeyear: 1, parencite: 1,
  textcite: 1, autocite: 1, nocite: 1, bibitem: 1,
  includegraphics: 1, input: 1, include: 1, subfile: 1, usepackage: 1, RequirePackage: 1,
  documentclass: 1, bibliography: 1, bibliographystyle: 1, addbibresource: 1, graphicspath: 1,
  url: 1, href: 1, hypersetup: 1,
  vspace: 1, "vspace*": 1, hspace: 1, "hspace*": 1, setlength: 2, addtolength: 2,
  setcounter: 2, addtocounter: 2, color: 1, textcolor: 1, colorbox: 1, pagestyle: 1,
  thispagestyle: 1, pagenumbering: 1, newcommand: 2, renewcommand: 2, providecommand: 2,
  newenvironment: 3, renewenvironment: 3, DeclareMathOperator: 2, newtheorem: 2,
};

const WORD = /[\p{L}\p{N}]+(?:['’\-][\p{L}\p{N}]+)*/gu;

/** Index after the group opened at `open` (`{` or `[`), skipping escaped characters. */
function groupEnd(text: string, open: number): number {
  const opener = text[open];
  const closer = opener === "{" ? "}" : "]";
  let depth = 0;
  for (let k = open; k < text.length; k++) {
    const c = text[k];
    if (c === "\\") k++;
    else if (c === opener) depth++;
    else if (c === closer && --depth === 0) return k + 1;
    else if (opener === "[" && c === "{") k = groupEnd(text, k) - 1;
  }
  return text.length;
}

function skipSpace(text: string, k: number): number {
  while (k < text.length && /\s/.test(text[k])) k++;
  return k;
}

/** First index at or after `from` where `needle` appears unescaped. */
function findUnescaped(text: string, needle: string, from: number): number {
  for (let k = from; k < text.length; k++) {
    if (text.startsWith(needle, k)) return k;
    if (text[k] === "\\") k++;
  }
  return text.length;
}

/** A short, readable form of a heading's source. */
function plainTitle(raw: string): string {
  const t = raw
    .replace(/\$[^$]*\$/g, "…")
    .replace(/\\[A-Za-z@]+\*?/g, "")
    .replace(/[{}\\~]/g, " ")
    .replace(/\s+/g, " ")
    .trim();
  return t.length > 28 ? `${t.slice(0, 27)}…` : t;
}

export function analyze(text: string): Analysis {
  const n = text.length;
  const keep = new Uint8Array(n).fill(1);
  const blank = (a: number, b: number) => keep.fill(0, a, Math.min(b, n));
  const regions: Region[] = [];
  const headings: { level: number; label: string; start: number }[] = [];
  const envs: { name: string; start: number }[] = [];
  let bodyStart = 0;
  let bodyEnd = n;

  // Blanks the brace and bracket arguments after `k`, up to `count` brace groups.
  const blankArgs = (k: number, count: number): number => {
    for (;;) {
      const s = skipSpace(text, k);
      if (text[s] === "[") {
        k = groupEnd(text, s);
        blank(s, k);
      } else if (text[s] === "{" && count > 0) {
        k = groupEnd(text, s);
        blank(s, k);
        count--;
      } else return k;
    }
  };

  let i = 0;
  while (i < n) {
    const c = text[i];
    if (c === "%") {
      const e = text.indexOf("\n", i);
      const end = e < 0 ? n : e;
      blank(i, end);
      i = end;
    } else if (c === "$") {
      const display = text[i + 1] === "$";
      const close = findUnescaped(text, display ? "$$" : "$", i + (display ? 2 : 1));
      const end = Math.min(n, close + (display ? 2 : 1));
      blank(i, end);
      i = end;
    } else if (c === "{" || c === "}" || c === "~") {
      blank(i, i + 1);
      i++;
    } else if (c === "\\") {
      const start = i;
      const m = /^[A-Za-z@]+\*?/.exec(text.slice(i + 1, i + 40));
      if (!m) {
        // Control symbol: \\, \%, \&, \(, \[, ...
        const next = text[i + 1];
        const close = next === "(" ? "\\)" : next === "[" ? "\\]" : null;
        const end = close ? Math.min(n, findUnescaped(text, close, i + 2) + 2) : i + 2;
        blank(i, end);
        i = end;
        continue;
      }
      const name = m[0];
      let j = i + 1 + name.length;
      blank(i, j);

      if (name === "begin" || name === "end") {
        const s = skipSpace(text, j);
        if (text[s] !== "{") {
          i = j;
          continue;
        }
        const e = groupEnd(text, s);
        const env = text.slice(s + 1, e - 1).trim();
        blank(j, e);
        j = e;
        if (name === "begin") {
          if (env === "document") bodyStart = j;
          else if (SKIPPED_ENVS.has(env)) {
            const close = text.indexOf(`\\end{${env}}`, j);
            j = close < 0 ? n : close + `\\end{${env}}`.length;
            blank(start, j);
          } else {
            envs.push({ name: env, start });
            j = blankArgs(j, ENV_ARGS[env] ?? 0);
          }
        } else if (env === "document") {
          bodyEnd = start;
        } else {
          const at = envs.map((x) => x.name).lastIndexOf(env);
          if (at >= 0) {
            regions.push({ label: env, start: envs[at].start, end: j });
            envs.length = at;
          }
        }
      } else if (name === "verb" || name === "verb*") {
        const close = text.indexOf(text[j], j + 1);
        j = close < 0 ? n : close + 1;
        blank(start, j);
      } else if (name.replace("*", "") in SECTION_LEVELS) {
        const base = name.replace("*", "");
        j = blankArgs(j, 0);
        const s = skipSpace(text, j);
        const title = text[s] === "{" ? plainTitle(text.slice(s + 1, groupEnd(text, s) - 1)) : "";
        const kind = base[0].toUpperCase() + base.slice(1);
        headings.push({ level: SECTION_LEVELS[base], label: title ? `${kind} “${title}”` : kind, start });
      } else if (name in SCOPE_COMMANDS) {
        j = blankArgs(j, 0);
        const s = skipSpace(text, j);
        if (text[s] === "{") regions.push({ label: SCOPE_COMMANDS[name], start, end: groupEnd(text, s) });
      } else if (name in SKIP_ARGS) {
        j = blankArgs(j, SKIP_ARGS[name]);
      }
      i = j;
    } else i++;
  }

  if (bodyStart > 0) blank(0, bodyStart);
  if (bodyEnd < n) blank(bodyEnd, n);

  headings.forEach((h, k) => {
    const next = headings.slice(k + 1).find((o) => o.level <= h.level);
    regions.push({ label: h.label, start: h.start, end: next ? next.start : bodyEnd });
  });

  const chars = text.split("");
  for (let k = 0; k < n; k++) if (!keep[k]) chars[k] = " ";
  const cleaned = chars.join("");
  const words: number[] = [];
  for (const w of cleaned.matchAll(WORD)) words.push(w.index!);

  return { words, bodyStart, bodyEnd, regions };
}

/** Index of the first word starting at or after `offset`. */
function lowerBound(words: number[], offset: number): number {
  let lo = 0;
  let hi = words.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (words[mid] < offset) lo = mid + 1;
    else hi = mid;
  }
  return lo;
}

const count = (a: Analysis, start: number, end: number) =>
  lowerBound(a.words, end) - lowerBound(a.words, start);

export interface WordCount {
  total: number;
  scope: { label: string; words: number } | null;
}

export function wordCount(a: Analysis, offset: number): WordCount {
  let best: Region | null = null;
  for (const r of a.regions)
    if (r.start <= offset && offset < r.end && (!best || r.end - r.start < best.end - best.start)) best = r;
  return {
    total: count(a, a.bodyStart, a.bodyEnd),
    scope: best && { label: best.label, words: count(a, best.start, best.end) },
  };
}

const cache = new WeakMap<monaco.editor.ITextModel, { version: number; analysis: Analysis }>();
let timer: number | undefined;

/** Updates the status bar's word counts for `editor`, shortly. */
export function scheduleWordCount(editor: monaco.editor.IStandaloneCodeEditor) {
  window.clearTimeout(timer);
  timer = window.setTimeout(() => {
    const model = editor.getModel();
    const pos = editor.getPosition();
    if (!model || !pos || model.getLanguageId() !== LATEX || pathOf(model)?.endsWith(".bib")) {
      set({ words: null });
      return;
    }
    let entry = cache.get(model);
    if (entry?.version !== model.getVersionId()) {
      entry = { version: model.getVersionId(), analysis: analyze(model.getValue()) };
      cache.set(model, entry);
    }
    set({ words: wordCount(entry.analysis, model.getOffsetAt(pos)) });
  }, 120);
}
