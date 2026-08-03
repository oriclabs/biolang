import { bridge } from "./bridge";
import type { EnvironmentInfo, JobProvenance, OpenFile, PackageInfo } from "./types";

function hex(bytes: ArrayBuffer): string {
  return [...new Uint8Array(bytes)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

export async function sourceSha256(source: string): Promise<string> {
  if (globalThis.crypto?.subtle) {
    return hex(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(source)));
  }
  let hash = 2166136261;
  for (const character of source) {
    hash ^= character.charCodeAt(0);
    hash = Math.imul(hash, 16777619);
  }
  return `fnv1a-${(hash >>> 0).toString(16).padStart(8, "0")}`;
}

export async function createJobProvenance(
  file: OpenFile,
  environment: EnvironmentInfo | undefined,
  packages: PackageInfo[],
  backend: string,
  targetId: string,
): Promise<JobProvenance> {
  const referencedPaths = [...file.content.matchAll(
    /["']([^"'\r\n]+\.(?:fa|fasta|fna|fastq|fq|bam|sam|vcf|bcf|bed|gff3?|gtf|csv|tsv|json|zarr|h5ad|mtx|pdb|nwk))(?:\.gz)?["']/gi,
  )].map((match) => match[1]);
  const environmentPaths = ["biolang.toml", "biolang.lock", "Cargo.lock", "environment.yml", "requirements.txt"];
  const [inputs, environmentFiles] = await Promise.all([
    bridge.checksumWorkspaceFiles([...new Set(referencedPaths)]).catch(() => []),
    bridge.checksumWorkspaceFiles(environmentPaths).catch(() => []),
  ]);
  const randomSeed = file.content.match(/\bset_seed\s*\(\s*([^)]+?)\s*\)/)?.[1];
  const sourceSnapshot = file.content.length <= 512_000
    ? file.content
    : `${file.content.slice(0, 512_000)}\n# Snapshot truncated by BioLang Desktop\n`;
  return {
    biolangVersion: environment?.blVersion?.replace(/^bl\s*/i, ""),
    packages: Object.fromEntries(
      packages
        .filter((entry) => entry.installed)
        .map((entry) => [entry.name, entry.version ?? "installed"]),
    ),
    backend,
    targetId,
    sourceHash: await sourceSha256(file.content),
    sourceSnapshot,
    workspace: environment?.workspace,
    entrypoint: file.name,
    parameters: {
      cellIndex: file.viewer === "notebook" ? "notebook" : "full-file",
    },
    capturedAt: new Date().toISOString(),
    platform: environment?.platform,
    architecture: environment?.architecture,
    inputs,
    randomSeed,
    tools: [{
      name: "BioLang",
      version: environment?.blVersion?.replace(/^bl\s*/i, ""),
      path: environment?.blPath,
    }],
    runtime: {
      locale: Intl.DateTimeFormat().resolvedOptions().locale,
      timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
      logicalCpus: navigator.hardwareConcurrency,
      userAgent: navigator.userAgent,
    },
    environmentFiles,
  };
}
