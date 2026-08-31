import type { BioLangSource } from "./session.js";

export interface SomerRunOptions {
  name?: string;
  entrypoint?: string;
  sourceFiles?: Array<{ path: string; content: string }>;
  inputs?: Array<{
    path: string;
    data: string | Blob | File | ArrayBuffer | ArrayBufferView | Uint8Array;
    sha256?: string;
  }>;
  chunkSize?: number;
  environment?: Record<string, string>;
  priority?: number;
  tags?: Record<string, string>;
  dependsOn?: string[];
  retryPolicy?: { maxAttempts?: number; backoffSeconds?: number };
  retentionDays?: number;
  runtimeVersion?: string;
  resources?: { profile?: string; cpu?: number; memoryGb?: number; gpu?: number };
}

export class SomerRun {
  readonly id: string;
  readonly status: string;
  refresh(): Promise<unknown>;
  wait(options?: { onUpdate?: (job: unknown) => void; signal?: AbortSignal; intervalMs?: number }): Promise<unknown>;
  events(onEvent: (event: unknown) => void, signal?: AbortSignal): Promise<void>;
  cancel(): Promise<unknown>;
  retry(): Promise<unknown>;
  artifacts(): Promise<unknown[]>;
  download(artifact: unknown): Promise<Uint8Array>;
}

export class SomerExecutor {
  constructor(client: unknown);
  serviceInfo(): Promise<unknown>;
  resourceProfiles(): Promise<unknown[]>;
  run(source: BioLangSource, options?: SomerRunOptions): Promise<SomerRun>;
}
