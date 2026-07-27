export type NotebookDirective = "hide" | "skip" | "echo" | "hide-output" | "chat";

export interface NotebookBlock {
  type: "markdown" | "code" | "metadata";
  content: string;
  start: number;
  end: number;
  directives: NotebookDirective[];
  directiveStart: number;
  directiveEnd: number;
  syntax?: "fenced" | "dash";
}

interface SourceLine {
  start: number;
  end: number;
  text: string;
  value: string;
}

const directiveByLine = new Map<string, NotebookDirective>([
  ["# @hide", "hide"],
  ["# @hide-code", "hide"],
  ["# @skip", "skip"],
  ["# @echo", "echo"],
  ["# @hide-output", "hide-output"],
  ["# @chat", "chat"],
]);

function sourceLines(source: string, start = 0) {
  const lines: SourceLine[] = [];
  let cursor = start;
  while (cursor < source.length) {
    const newline = source.indexOf("\n", cursor);
    const end = newline === -1 ? source.length : newline + 1;
    const text = source.slice(cursor, end);
    lines.push({
      start: cursor,
      end,
      text,
      value: text.replace(/\r?\n$/, ""),
    });
    cursor = end;
  }
  return lines;
}

function trimmedSpan(source: string, start: number, end: number) {
  const raw = source.slice(start, end);
  const leading = raw.match(/^\s*/)?.[0].length ?? 0;
  const trailing = raw.match(/\s*$/)?.[0].length ?? 0;
  return {
    start: start + leading,
    end: Math.max(start + leading, end - trailing),
  };
}

function parseDirectiveHeader(source: string, start: number, end: number) {
  const directives: NotebookDirective[] = [];
  let cursor = start;
  while (cursor < end) {
    const newline = source.indexOf("\n", cursor);
    const lineEnd = newline === -1 || newline >= end ? end : newline + 1;
    const line = source.slice(cursor, lineEnd).replace(/\r?\n$/, "").trim();
    const directive = directiveByLine.get(line);
    if (!directive) break;
    if (!directives.includes(directive)) directives.push(directive);
    cursor = lineEnd;
  }
  return { directives, bodyStart: cursor };
}

function frontMatterSpan(source: string) {
  const bomOffset = source.startsWith("\uFEFF") ? 1 : 0;
  const lines = sourceLines(source, bomOffset);
  if (lines[0]?.value.trim() !== "---") return undefined;
  const closeIndex = lines.findIndex((line, index) => index > 0 && line.value.trim() === "---");
  if (closeIndex < 0) return undefined;

  let foundField = false;
  for (const line of lines.slice(1, closeIndex)) {
    const value = line.value.trim();
    if (!value) continue;
    const separator = value.indexOf(":");
    if (separator < 1) return undefined;
    const key = value.slice(0, separator).trim();
    if (!/^[A-Za-z][A-Za-z0-9_-]*$/.test(key)) return undefined;
    foundField = true;
  }
  if (!foundField) return undefined;

  return {
    contentStart: lines[0].end,
    contentEnd: lines[closeIndex].start,
    bodyStart: lines[closeIndex].end,
  };
}

function isBioLangFence(value: string) {
  const trimmed = value.trim().toLowerCase();
  return trimmed === "```" || trimmed === "```biolang" || trimmed === "```bl";
}

function isOtherFence(value: string) {
  const trimmed = value.trim().toLowerCase();
  return trimmed.startsWith("```") && !isBioLangFence(value);
}

export function parseNotebook(source: string): NotebookBlock[] {
  const blocks: NotebookBlock[] = [];
  const frontMatter = frontMatterSpan(source);
  let bodyStart = 0;
  if (frontMatter) {
    const span = trimmedSpan(source, frontMatter.contentStart, frontMatter.contentEnd);
    blocks.push({
      type: "metadata",
      content: source.slice(span.start, span.end),
      start: span.start,
      end: span.end,
      directives: [],
      directiveStart: span.start,
      directiveEnd: span.start,
    });
    bodyStart = frontMatter.bodyStart;
  }

  let currentStart = bodyStart;
  let currentEnd = bodyStart;
  let inDashCode = false;
  let inFencedCode = false;
  let inOtherFence = false;
  let codeSyntax: NotebookBlock["syntax"];

  const flush = (isCode: boolean) => {
    const span = trimmedSpan(source, currentStart, currentEnd);
    if (span.start >= span.end) {
      currentStart = currentEnd;
      return;
    }
    if (!isCode) {
      blocks.push({
        type: "markdown",
        content: source.slice(currentStart, currentEnd),
        start: currentStart,
        end: currentEnd,
        directives: [],
        directiveStart: currentStart,
        directiveEnd: currentStart,
      });
    } else {
      const header = parseDirectiveHeader(source, span.start, span.end);
      blocks.push({
        type: "code",
        content: source.slice(header.bodyStart, span.end),
        start: header.bodyStart,
        end: span.end,
        directives: header.directives,
        directiveStart: span.start,
        directiveEnd: header.bodyStart,
        syntax: codeSyntax,
      });
    }
    currentStart = currentEnd;
  };

  for (const line of sourceLines(source, bodyStart)) {
    const trimmed = line.value.trim();

    if (inOtherFence) {
      currentEnd = line.end;
      if (trimmed === "```") inOtherFence = false;
      continue;
    }

    if (inFencedCode) {
      if (trimmed === "```") {
        flush(true);
        inFencedCode = false;
        codeSyntax = undefined;
        currentStart = line.end;
        currentEnd = line.end;
      } else {
        currentEnd = line.end;
      }
      continue;
    }

    if (!inDashCode && isBioLangFence(line.value)) {
      flush(false);
      inFencedCode = true;
      codeSyntax = "fenced";
      currentStart = line.end;
      currentEnd = line.end;
      continue;
    }

    if (!inDashCode && isOtherFence(line.value)) {
      inOtherFence = true;
      if (currentStart === currentEnd) currentStart = line.start;
      currentEnd = line.end;
      continue;
    }

    if (trimmed === "---") {
      flush(inDashCode);
      inDashCode = !inDashCode;
      codeSyntax = inDashCode ? "dash" : undefined;
      currentStart = line.end;
      currentEnd = line.end;
      continue;
    }

    if (currentStart === currentEnd) currentStart = line.start;
    currentEnd = line.end;
  }

  flush(inDashCode || inFencedCode);
  return blocks;
}

export function updateNotebookBlock(source: string, block: NotebookBlock, content: string) {
  return `${source.slice(0, block.start)}${content}${source.slice(block.end)}`;
}

export function setNotebookDirective(
  source: string,
  block: NotebookBlock,
  directive: NotebookDirective,
  enabled: boolean,
) {
  if (block.type !== "code") return source;
  const directives = enabled
    ? [...block.directives, directive].filter((value, index, values) => values.indexOf(value) === index)
    : block.directives.filter((value) => value !== directive);
  const newline = source.includes("\r\n") ? "\r\n" : "\n";
  const header = directives.map((value) => `# @${value}`).join(newline);
  const replacement = header ? `${header}${newline}` : "";
  return `${source.slice(0, block.directiveStart)}${replacement}${source.slice(block.start)}`;
}

export interface NotebookRunPlan {
  notebookSource: string;
  scriptSource: string;
  markerPrefix: string;
  cellIndexes: number[];
  reportedCellIndexes: number[];
  hiddenOutputCellIndexes: number[];
}

export interface NotebookOutputChunk {
  cellIndex: number;
  data: string;
}

function markerCell(markerPrefix: string, cellIndex: number) {
  return `println("${markerPrefix}${cellIndex}")`;
}

function directiveHeader(directives: NotebookDirective[]) {
  return directives.map((directive) => `# @${directive}`).join("\n");
}

function echoSource(block: NotebookBlock) {
  return block.directives.includes("echo") && !block.directives.includes("hide")
    ? block.content.split(/\r?\n/).map((line) => `println(${JSON.stringify(`  ${line}`)})`).join("\n")
    : "";
}

function remoteCellSource(block: NotebookBlock) {
  const echo = echoSource(block);
  const code = block.directives.includes("chat")
    ? `println(chat(${JSON.stringify(block.content.trim())}))`
    : block.content;
  return [echo, code].filter(Boolean).join("\n");
}

export function createNotebookRunPlan(
  source: string,
  runId: string,
  selectedCellIndex?: number,
): NotebookRunPlan {
  const markerPrefix = `__BIOLANG_DESKTOP_CELL_${runId}_`;
  const codeBlocks = parseNotebook(source).filter((block) => block.type === "code");
  const indexed = codeBlocks
    .map((block, cellIndex) => ({ block, cellIndex }))
    .filter(({ block }) => !block.directives.includes("skip"));
  const requestedCell = selectedCellIndex == null
    ? undefined
    : codeBlocks[selectedCellIndex];
  const selected = selectedCellIndex == null
    ? indexed
    : requestedCell?.directives.includes("skip")
      ? []
      : indexed.filter(({ cellIndex }) => cellIndex <= selectedCellIndex);
  const reportedCellIndexes = selectedCellIndex == null
    ? selected.map(({ cellIndex }) => cellIndex)
    : selected.filter(({ cellIndex }) => cellIndex === selectedCellIndex).map(({ cellIndex }) => cellIndex);
  const replayedCellIndexes = selectedCellIndex == null
    ? []
    : selected.filter(({ cellIndex }) => cellIndex < selectedCellIndex).map(({ cellIndex }) => cellIndex);

  return {
    notebookSource: selected.map(({ block, cellIndex }) => {
      const echo = echoSource(block);
      const directives = block.directives.filter((directive) => directive !== "echo");
      if (replayedCellIndexes.includes(cellIndex) && !directives.includes("hide-output")) {
        directives.push("hide-output");
      }
      const header = directiveHeader(directives);
      const cell = [header, block.content].filter(Boolean).join("\n");
      const echoCell = echo ? `\`\`\`biolang\n${echo}\n\`\`\`\n\n` : "";
      return `${echoCell}\`\`\`biolang\n${cell}\n\`\`\`\n\n\`\`\`biolang\n${markerCell(markerPrefix, cellIndex)}\n\`\`\``;
    }).join("\n\n"),
    scriptSource: selected.map(({ block, cellIndex }) =>
      `${remoteCellSource(block)}\n${markerCell(markerPrefix, cellIndex)}`,
    ).join("\n\n"),
    markerPrefix,
    cellIndexes: selected.map(({ cellIndex }) => cellIndex),
    reportedCellIndexes,
    hiddenOutputCellIndexes: selected
      .filter(({ block, cellIndex }) =>
        replayedCellIndexes.includes(cellIndex) || block.directives.includes("hide-output"))
      .map(({ cellIndex }) => cellIndex),
  };
}

export class NotebookOutputRouter {
  private currentOffset = 0;
  private pendingStdout = "";
  private readonly hiddenOutputCellIndexes: Set<number>;

  constructor(
    private readonly markerPrefix: string,
    private readonly cellIndexes: number[],
    hiddenOutputCellIndexes: number[] = [],
  ) {
    this.hiddenOutputCellIndexes = new Set(hiddenOutputCellIndexes);
  }

  stdout(data: string): { visible: string; chunks: NotebookOutputChunk[] } {
    const combined = this.pendingStdout + data;
    const lines = combined.split(/(?<=\n)/);
    this.pendingStdout = lines.at(-1)?.endsWith("\n") ? "" : lines.pop() ?? "";
    return this.routeLines(lines);
  }

  stderr(data: string): { visible: string; chunks: NotebookOutputChunk[] } {
    const cellIndex = this.cellIndexes[this.currentOffset];
    if (cellIndex == null || this.hiddenOutputCellIndexes.has(cellIndex)) {
      return { visible: "", chunks: [] };
    }
    return {
      visible: data,
      chunks: !data ? [] : [{ cellIndex, data }],
    };
  }

  flush(): { visible: string; chunks: NotebookOutputChunk[] } {
    const pending = this.pendingStdout;
    this.pendingStdout = "";
    return this.routeLines(pending ? [pending] : []);
  }

  private routeLines(lines: string[]): { visible: string; chunks: NotebookOutputChunk[] } {
    let visible = "";
    const chunks: NotebookOutputChunk[] = [];
    for (const line of lines) {
      const marker = line.trim();
      const expectedCell = this.cellIndexes[this.currentOffset];
      if (expectedCell != null && marker === `${this.markerPrefix}${expectedCell}`) {
        this.currentOffset += 1;
        continue;
      }
      if (expectedCell == null || this.hiddenOutputCellIndexes.has(expectedCell)) continue;
      visible += line;
      if (line) chunks.push({ cellIndex: expectedCell, data: line });
    }
    return { visible, chunks };
  }
}
