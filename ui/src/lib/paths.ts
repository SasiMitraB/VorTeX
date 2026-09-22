export const basename = (p: string) => p.slice(p.lastIndexOf("/") + 1);

export const dirname = (p: string) => p.slice(0, Math.max(p.lastIndexOf("/"), 1));

export function extname(p: string): string {
  const name = basename(p);
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(dot + 1).toLowerCase() : "";
}

/** `p` relative to `root`, or `p` itself when it is outside. */
export function relativeTo(root: string | undefined, p: string): string {
  return root && p.startsWith(root + "/") ? p.slice(root.length + 1) : p;
}

export const isPdf = (p: string) => extname(p) === "pdf";

export const isImage = (p: string) => ["png", "jpg", "jpeg", "gif", "svg"].includes(extname(p));
