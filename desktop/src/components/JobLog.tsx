import type { JobLogChunk } from "../types";

type LogPart =
  | { kind: "text"; value: string }
  | { kind: "svg"; value: string };

function richLogParts(text: string): LogPart[] {
  const parts: LogPart[] = [];
  const expression = /<svg\b[\s\S]*?<\/svg>/gi;
  let offset = 0;
  for (const match of text.matchAll(expression)) {
    const index = match.index ?? 0;
    if (index > offset) parts.push({ kind: "text", value: text.slice(offset, index) });
    parts.push({ kind: "svg", value: match[0] });
    offset = index + match[0].length;
  }
  if (offset < text.length) parts.push({ kind: "text", value: text.slice(offset) });
  return parts;
}

export function JobLog({
  chunks,
  emptyText = "No output was recorded for this job.",
  className,
}: {
  chunks: JobLogChunk[] | undefined;
  emptyText?: string;
  className?: string;
}) {
  if (!chunks?.length) {
    return <pre className={className}>{emptyText}</pre>;
  }
  return (
    <div className={`job-log ${className ?? ""}`}>
      {chunks.flatMap((chunk, chunkIndex) =>
        richLogParts(chunk.text).map((part, partIndex) =>
          part.kind === "svg"
            ? <figure className="job-log-plot" key={`${chunkIndex}-${partIndex}`}>
                <img
                  alt="BioLang plot output"
                  src={`data:image/svg+xml;charset=utf-8,${encodeURIComponent(part.value)}`}
                />
              </figure>
            : <span className={`job-log-chunk ${chunk.stream}`} key={`${chunkIndex}-${partIndex}`}>{part.value}</span>,
        ))}
    </div>
  );
}
