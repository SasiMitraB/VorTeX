import * as monaco from "./monaco";

export const LATEX = "latex";

const math = (close: RegExp) => [
  [/%.*$/, "comment"],
  [/\\[$%]/, "string.math"],
  [close, { token: "string.math.delimiter", next: "@pop" }],
  [/\\[a-zA-Z@]+/, "string.math.command"],
  [/[^$\\%\]\)]+/, "string.math"],
  [/./, "string.math"],
];

const tokens: monaco.languages.IMonarchLanguage = {
  defaultToken: "",
  tokenizer: {
    root: [
      [/%.*$/, "comment"],
      [/(\\(?:begin|end))(\s*)(\{)([^}]*)(\})/, ["keyword.env", "", "delimiter.curly", "type.env", "delimiter.curly"]],
      [/\\(?:part|chapter|(?:sub)*section|(?:sub)?paragraph)\*?(?![a-zA-Z])/, "keyword.section"],
      [
        /\\(?:label|[cC]?ref|eqref|autoref|pageref|nameref|cite[a-zA-Z]*|[a-z]*cite[a-z]*|input|include|includegraphics|bibliography|addbibresource|usepackage|documentclass)(?![a-zA-Z])/,
        "keyword.ref",
      ],
      [/\$\$/, { token: "string.math.delimiter", next: "@displaymath" }],
      [/\$/, { token: "string.math.delimiter", next: "@inlinemath" }],
      [/\\\[/, { token: "string.math.delimiter", next: "@bracketmath" }],
      [/\\\(/, { token: "string.math.delimiter", next: "@parenmath" }],
      [/\\[a-zA-Z@]+\*?/, "keyword"],
      [/\\./, "keyword.escape"],
      [/[{}]/, "delimiter.curly"],
      [/[[\]]/, "delimiter.square"],
      [/&/, "delimiter"],
    ],
    inlinemath: math(/\$/),
    displaymath: math(/\$\$/),
    bracketmath: math(/\\\]/),
    parenmath: math(/\\\)/),
  } as monaco.languages.IMonarchLanguage["tokenizer"],
};

const config: monaco.languages.LanguageConfiguration = {
  comments: { lineComment: "%" },
  brackets: [
    ["{", "}"],
    ["[", "]"],
    ["(", ")"],
  ],
  autoClosingPairs: [
    { open: "{", close: "}" },
    { open: "[", close: "]" },
    { open: "(", close: ")" },
    { open: "$", close: "$", notIn: ["comment"] },
  ],
  // Typing one of these with a selection wraps it.
  surroundingPairs: [
    { open: "{", close: "}" },
    { open: "[", close: "]" },
    { open: "(", close: ")" },
    { open: "$", close: "$" },
    { open: '"', close: '"' },
  ],
  autoCloseBefore: ";:.,=}])>$ \n\t",
  wordPattern: /(-?\d*\.\d\w*)|([^\s`~!@#$%^&*()=+[{\]}\\|;:'",.<>/?]+)/,
  folding: {
    markers: { start: /\\begin\{/, end: /\\end\{/ },
  },
};

const PALETTE = {
  dark: {
    base: "1e1e2e", mantle: "181825", crust: "11111b", surface0: "313244", surface1: "45475a", surface2: "585b70",
    overlay0: "6c7086", overlay2: "9399b2", text: "cdd6f4", lavender: "b4befe", rosewater: "f5e0dc",
    mauve: "cba6f7", blue: "89b4fa", peach: "fab387", red: "f38ba8", teal: "94e2d5", green: "a6e3a1",
    yellow: "f9e2af", pink: "f5c2e7",
  },
  light: {
    base: "eff1f5", mantle: "e6e9ef", crust: "dce0e8", surface0: "ccd0da", surface1: "bcc0cc", surface2: "acb0be",
    overlay0: "9ca0b0", overlay2: "7c7f93", text: "4c4f69", lavender: "7287fd", rosewater: "dc8a78",
    mauve: "8839ef", blue: "1e66f5", peach: "fe640b", red: "d20f39", teal: "179299", green: "40a02b",
    yellow: "df8e1d", pink: "ea76cb",
  },
};

function theme(mode: "light" | "dark"): monaco.editor.IStandaloneThemeData {
  const c = PALETTE[mode];
  return {
    base: mode === "dark" ? "vs-dark" : "vs",
    inherit: true,
    rules: [
      { token: "", foreground: c.text },
      { token: "comment", foreground: c.overlay0, fontStyle: "italic" },
      { token: "keyword", foreground: c.mauve },
      { token: "keyword.escape", foreground: c.pink },
      { token: "keyword.env", foreground: c.blue },
      { token: "type.env", foreground: c.peach },
      { token: "keyword.section", foreground: c.red, fontStyle: "bold" },
      { token: "keyword.ref", foreground: c.teal },
      { token: "string.math", foreground: c.green },
      { token: "string.math.delimiter", foreground: c.green, fontStyle: "bold" },
      { token: "string.math.command", foreground: c.yellow },
      { token: "delimiter", foreground: c.overlay2 },
      { token: "delimiter.curly", foreground: c.overlay2 },
      { token: "delimiter.square", foreground: c.overlay2 },
    ],
    colors: {
      "editor.background": `#${c.base}`,
      "editor.foreground": `#${c.text}`,
      "editor.lineHighlightBackground": `#${c.surface0}66`,
      "editor.lineHighlightBorder": `#00000000`,
      "editor.selectionBackground": `#${c.surface2}88`,
      "editor.inactiveSelectionBackground": `#${c.surface1}66`,
      "editorCursor.foreground": `#${c.rosewater}`,
      "editorLineNumber.foreground": `#${c.overlay0}`,
      "editorLineNumber.activeForeground": `#${c.lavender}`,
      "editorIndentGuide.background1": `#${c.surface0}`,
      "editorWidget.background": `#${c.mantle}`,
      "editorWidget.border": `#${c.surface1}`,
      "editorHoverWidget.background": `#${c.mantle}`,
      "editorHoverWidget.border": `#${c.surface1}`,
      "editorSuggestWidget.background": `#${c.mantle}`,
      "editorSuggestWidget.border": `#${c.surface1}`,
      "editorSuggestWidget.selectedBackground": `#${c.surface0}`,
      "editorWarning.foreground": `#${c.yellow}`,
      "editorError.foreground": `#${c.red}`,
      "editorInfo.foreground": `#${c.blue}`,
      "scrollbarSlider.background": `#${c.surface1}66`,
      "scrollbarSlider.hoverBackground": `#${c.surface2}88`,
      "editorGutter.background": `#${c.base}`,
    },
  };
}

/** Text color of the given theme, for rendering math previews to match. */
export const themeTextHex = (mode: "light" | "dark") => PALETTE[mode].text;

export function registerLatex() {
  monaco.languages.register({ id: LATEX, extensions: [".tex", ".sty", ".cls", ".bib", ".ltx"] });
  monaco.languages.setMonarchTokensProvider(LATEX, tokens);
  monaco.languages.setLanguageConfiguration(LATEX, config);
  monaco.editor.defineTheme("vortex-dark", theme("dark"));
  monaco.editor.defineTheme("vortex-light", theme("light"));
}
