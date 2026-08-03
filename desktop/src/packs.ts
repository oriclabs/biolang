import type { FileEntry } from "./types";

/**
 * Example packs in the browser workbench.
 *
 * A pack is published as a single JSON bundle (see `scripts/build-packs.mjs`)
 * so the browser needs no archive decoder: the bundle's `files` map drops
 * straight into the demo workspace that browser mode already uses.
 *
 * Everything here is pure apart from the two fetches, so the tree building and
 * link parsing can be exercised without a network or a DOM.
 */

/** One problem, as published in `index.json` and in a bundle. */
export interface PackProblem {
  id: string;
  title: string;
  file: string;
  url: string;
  status: "solved" | "partial" | "blocked";
  asserted: boolean;
  network: boolean;
  blockedOn?: string;
}

export interface PackCounts {
  solved: number;
  partial: number;
  blocked: number;
  asserted: number;
  network: number;
}

export interface PackIndexEntry {
  id: string;
  name: string;
  version: string;
  description: string;
  track?: string;
  listUrl?: string;
  license?: string;
  requires?: string;
  problems: number;
  counts: PackCounts;
  index: PackProblem[];
  bundle: { file: string; url: string; bytes: number; sha256: string };
}

export interface PackBundle {
  schemaVersion: number;
  id: string;
  version: string;
  pack: Record<string, unknown>;
  problems: PackProblem[];
  files: Record<string, string>;
}

/** Where the catalog lives. `pages.yml` publishes it alongside the site. */
export const PACK_INDEX_URL = "/packs/index.json";

/**
 * A deep link into a pack: `?pack=rosalind-armory&problem=SUBO`.
 *
 * The problem id is upper-cased because Rosalind writes them that way in URLs
 * and people copy them by hand; matching case-sensitively would turn a
 * reasonable link into a silent no-op.
 */
export function parsePackLink(search: string): { pack?: string; problem?: string } {
  const params = new URLSearchParams(search);
  const pack = params.get("pack")?.trim();
  const problem = params.get("problem")?.trim();
  return {
    pack: pack || undefined,
    problem: problem ? problem.toUpperCase() : undefined,
  };
}

function resolve(url: string, base: string) {
  if (/^https?:\/\//.test(url)) return url;
  const trimmed = base.replace(/\/+$/, "");
  return url.startsWith("/") ? `${trimmed}${url}` : `${trimmed}/${url}`;
}

export async function fetchPackIndex(
  base = "",
  fetcher: typeof fetch = fetch,
): Promise<PackIndexEntry[]> {
  const response = await fetcher(resolve(PACK_INDEX_URL, base));
  if (!response.ok) throw new Error(`Pack catalog unavailable (HTTP ${response.status})`);
  const catalog = (await response.json()) as { packs?: PackIndexEntry[] };
  return catalog.packs ?? [];
}

/**
 * Download a bundle and check it against the digest the catalog advertises.
 *
 * The catalog and the bundle are separate files: without this a truncated or
 * stale bundle would be installed as if it were fine. Verification is skipped
 * only where `crypto.subtle` is unavailable (non-secure contexts), which is
 * reported rather than hidden.
 */
export async function fetchPackBundle(
  entry: PackIndexEntry,
  base = "",
  fetcher: typeof fetch = fetch,
): Promise<{ bundle: PackBundle; verified: boolean }> {
  const response = await fetcher(resolve(entry.bundle.url, base));
  if (!response.ok) throw new Error(`Could not download ${entry.name} (HTTP ${response.status})`);
  const raw = await response.text();

  let verified = false;
  if (globalThis.crypto?.subtle) {
    const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(raw));
    const hex = [...new Uint8Array(digest)]
      .map((byte) => byte.toString(16).padStart(2, "0"))
      .join("");
    if (hex !== entry.bundle.sha256) {
      throw new Error(`${entry.name} failed its checksum — the download is not what the catalog describes`);
    }
    verified = true;
  }

  return { bundle: JSON.parse(raw) as PackBundle, verified };
}

/** Workspace path for a file inside a pack, namespaced by the pack id. */
/**
 * Find a problem's file in an installed pack's manifest.
 *
 * The bundle is the usual source of this, but a pack already in the workspace
 * should not be downloaded again just to learn where one of its files sits.
 */
export function problemPathFromManifest(
  packId: string,
  manifest: string,
  problemId: string,
): string | undefined {
  const wanted = problemId.toUpperCase();
  for (const block of manifest.split(/^\[\[problem\]\]$/m).slice(1)) {
    const id = block.match(/^\s*id\s*=\s*"([^"]+)"/m)?.[1];
    const file = block.match(/^\s*file\s*=\s*"([^"]+)"/m)?.[1];
    if (id && file && id.toUpperCase() === wanted) return packFilePath(packId, file);
  }
  return undefined;
}

export function packFilePath(packId: string, relative: string) {
  return `${packId}/${relative}`;
}

/** The bundle's files, keyed by the path they take in the workspace. */
export function packWorkspaceFiles(bundle: PackBundle): Record<string, string> {
  const files: Record<string, string> = {};
  for (const [relative, content] of Object.entries(bundle.files)) {
    files[packFilePath(bundle.id, relative)] = content;
  }
  return files;
}

/**
 * Build the explorer tree for a pack from its flat file map.
 *
 * Directories are synthesised from the path segments, so a bundle never has to
 * describe its own folder structure and cannot disagree with it.
 */
export function packFileEntries(bundle: PackBundle): FileEntry {
  const root: FileEntry = {
    name: bundle.id,
    path: bundle.id,
    kind: "directory",
    size: 0,
    children: [],
  };

  for (const [relative, content] of Object.entries(bundle.files)) {
    const segments = relative.split("/").filter(Boolean);
    let parent = root;
    segments.forEach((segment, position) => {
      const isFile = position === segments.length - 1;
      const path = packFilePath(bundle.id, segments.slice(0, position + 1).join("/"));
      let node = parent.children.find((child) => child.name === segment);
      if (!node) {
        node = {
          name: segment,
          path,
          kind: isFile ? "file" : "directory",
          size: isFile ? content.length : 0,
          children: [],
        };
        parent.children.push(node);
      }
      parent = node;
    });
  }

  sortTree(root);
  return root;
}

/** Directories first, then files, each alphabetical — as the explorer expects. */
function sortTree(node: FileEntry) {
  node.children.sort((a, b) => {
    if (a.kind !== b.kind) return a.kind === "directory" ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
  for (const child of node.children) sortTree(child);
}

/** Workspace path of a problem, for deep links and "open next". */
export function problemPath(bundle: PackBundle, problemId: string): string | undefined {
  const wanted = problemId.toUpperCase();
  const problem = bundle.problems.find((candidate) => candidate.id.toUpperCase() === wanted);
  return problem ? packFilePath(bundle.id, problem.file) : undefined;
}

/** How far through a pack someone is, for the progress line. */
export function packSummary(entry: PackIndexEntry) {
  const { solved, partial, asserted, network } = entry.counts;
  return {
    solved,
    partial,
    asserted,
    network,
    // Only asserted problems can be checked locally; saying "15 verified" when
    // three need NCBI would overstate what pressing Run actually proves.
    checkable: asserted,
    label: `${entry.problems} problems — ${solved} solved, ${asserted} checkable offline`,
  };
}
