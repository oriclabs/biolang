import type { DataPreview } from "./types";

export type ViewerKind = "editor" | "data" | "notebook" | "workflow";

export interface ViewerRegistration {
  id: string;
  kind: ViewerKind;
  extensions: string[];
}

export const viewerRegistry: ViewerRegistration[] = [
  {
    id: "notebook",
    kind: "notebook",
    extensions: ["bln", "bl.md"],
  },
  {
    id: "workflow",
    kind: "workflow",
    extensions: ["blflow"],
  },
  {
    id: "biological-data",
    kind: "data",
    extensions: [
      "fasta", "fa", "fna", "faa", "fastq", "fq", "vcf", "bed", "gff", "gff3",
      "gtf", "sam", "nwk", "newick", "tree", "pdb", "ent", "cif", "mmcif",
      "csv", "tsv", "png", "jpg", "jpeg", "gif", "webp", "svg", "pdf",
    ],
  },
];

function matchesExtension(path: string, extension: string) {
  return path.toLowerCase().endsWith(`.${extension}`);
}

export function viewerForPath(path: string, size = 0): ViewerKind {
  for (const registration of viewerRegistry) {
    if (registration.extensions.some((extension) => matchesExtension(path, extension))) {
      return registration.kind;
    }
  }
  if (path.toLowerCase().endsWith(".json") && size > 512_000) return "data";
  return "editor";
}

export function previewExportFormats(preview: DataPreview) {
  if (preview.kind === "fasta") return ["fasta", "json"];
  if (preview.kind === "newick") return ["newick", "json"];
  if (preview.columns.length) return ["csv", "tsv", "json"];
  return ["json"];
}
