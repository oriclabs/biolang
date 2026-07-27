import type { JobLogChunk } from "../types";

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
    <pre className={className}>
      {chunks.map((chunk, index) => (
        <span className={`job-log-chunk ${chunk.stream}`} key={index}>{chunk.text}</span>
      ))}
    </pre>
  );
}
