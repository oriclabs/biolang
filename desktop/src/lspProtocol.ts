export interface LspRange {
  start: { line: number; character: number };
  end: { line: number; character: number };
}

export async function startLspListening(
  subscribe: () => Promise<() => void>,
  start: () => Promise<boolean>,
) {
  const dispose = await subscribe();
  try {
    if (await start()) return { started: true as const, dispose };
    dispose();
    return { started: false as const };
  } catch (error) {
    dispose();
    throw error;
  }
}

export function replaceProblemsForPath<T extends { path: string }>(
  current: T[],
  path: string,
  next: T[],
) {
  return [...current.filter((problem) => problem.path !== path), ...next];
}

export function pathToFileUri(path: string) {
  const normalized = path.replaceAll("\\", "/");
  const windowsPath = normalized.replace(/^\/+([A-Za-z]:\/)/, "$1");
  if (/^[A-Za-z]:\//.test(windowsPath)) {
    const [drive, ...segments] = windowsPath.split("/");
    return `file:///${drive}/${segments.map(encodeURIComponent).join("/")}`;
  }
  if (normalized.startsWith("//")) {
    const [host, ...segments] = normalized.slice(2).split("/");
    return `file://${host}/${segments.map(encodeURIComponent).join("/")}`;
  }
  const absolute = normalized.startsWith("/") ? normalized : `/${normalized}`;
  return `file://${absolute.split("/").map(encodeURIComponent).join("/")}`;
}

export function completionReplacementRange(
  lineNumber: number,
  column: number,
  wordStartColumn: number,
) {
  return {
    startLineNumber: lineNumber,
    endLineNumber: lineNumber,
    startColumn: wordStartColumn,
    endColumn: column,
  };
}

export function diagnosticMarkerRange(range: LspRange) {
  const sameLine = range.start.line === range.end.line;
  return {
    startLineNumber: range.start.line + 1,
    startColumn: range.start.character + 1,
    endLineNumber: range.end.line + 1,
    endColumn: sameLine
      ? Math.max(range.end.character + 1, range.start.character + 2)
      : Math.max(range.end.character + 1, 1),
  };
}
