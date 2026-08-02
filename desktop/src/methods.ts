import type { Job, JobProvenance } from "./types";

/**
 * A methods paragraph and citation, generated from a run's provenance.
 *
 * Provenance has recorded the BioLang version, every package version, the
 * random seed, input checksums, and the platform since it was introduced — and
 * only ever stored them. This is the third thing that record can pay for, after
 * run comparison and environment restore, and the one aimed squarely at what
 * researchers are optimising for: getting the paper out.
 *
 * The text is deliberately conservative. It states what was actually recorded
 * and nothing else: no invented sample sizes, no claims about what the analysis
 * showed. A methods section that overstates is a retraction risk, and a
 * generator that guesses would be worse than no generator.
 */

/** Fixed citation metadata, mirroring CITATION.cff at the repository root. */
export const citation = {
  title: "BioLang: A Pipe-First Domain-Specific Language for Bioinformatics",
  author: "Bandi, Raj",
  affiliation: "ORIC Labs",
  year: 2026,
  url: "https://lang.bio",
  repository: "https://github.com/oriclabs/biolang",
  license: "MIT",
};

function sentenceList(items: string[]): string {
  if (items.length === 0) return "";
  if (items.length === 1) return items[0];
  if (items.length === 2) return `${items[0]} and ${items[1]}`;
  return `${items.slice(0, -1).join(", ")}, and ${items.at(-1)}`;
}

/**
 * The methods paragraph.
 *
 * Version numbers are stated only when recorded; an unknown version is omitted
 * rather than written as "unknown", because a methods section is copied into a
 * manuscript verbatim and a placeholder would survive into submission.
 */
export function methodsParagraph(provenance: JobProvenance): string {
  const sentences: string[] = [];

  const version = provenance.biolangVersion;
  sentences.push(
    version
      ? `Analyses were performed using BioLang ${version}.`
      : "Analyses were performed using BioLang.",
  );

  const packages = Object.entries(provenance.packages ?? {})
    .filter(([, packageVersion]) => packageVersion && packageVersion !== "installed")
    .map(([name, packageVersion]) => `${name} (v${packageVersion})`);
  if (packages.length) {
    sentences.push(`The following BioLang packages were used: ${sentenceList(packages)}.`);
  }

  if (provenance.randomSeed) {
    sentences.push(
      `Stochastic steps were seeded with ${provenance.randomSeed} to make the results reproducible.`,
    );
  }

  const inputs = provenance.inputs ?? [];
  const checksummed = inputs.filter((input) => input.sha256);
  if (inputs.length) {
    sentences.push(
      checksummed.length === inputs.length
        ? `The analysis read ${inputs.length} input file${inputs.length === 1 ? "" : "s"}; SHA-256 checksums for each are given in the supplementary material.`
        : `The analysis read ${inputs.length} input file${inputs.length === 1 ? "" : "s"}.`,
    );
  }

  const platform = [provenance.platform, provenance.architecture].filter(Boolean).join(" / ");
  if (platform) {
    sentences.push(`Analyses were run on ${platform}.`);
  }

  sentences.push(
    `The analysis script and a complete provenance record are available as a run bundle exported from BioLang Studio.`,
  );

  return sentences.join(" ");
}

/** Input checksums as a supplementary table, in Markdown. */
export function checksumTable(provenance: JobProvenance): string {
  const inputs = (provenance.inputs ?? []).filter((input) => input.sha256);
  if (!inputs.length) return "";
  const rows = inputs.map(
    (input) => `| ${input.path} | ${input.size} | ${input.sha256} |`,
  );
  return ["| File | Bytes | SHA-256 |", "| --- | --- | --- |", ...rows].join("\n");
}

export function bibtex(): string {
  return [
    "@software{biolang,",
    `  title    = {${citation.title}},`,
    `  author   = {${citation.author}},`,
    `  year     = {${citation.year}},`,
    `  url      = {${citation.url}},`,
    `  note     = {${citation.repository}}`,
    "}",
  ].join("\n");
}

/** APA-style reference, for journals that do not take BibTeX. */
export function apaReference(): string {
  return `${citation.author} (${citation.year}). ${citation.title} [Computer software]. ${citation.url}`;
}

/** The full block offered for copying: paragraph, checksums, and citation. */
export function methodsDocument(job: Job): string {
  const provenance = job.provenance;
  if (!provenance) return "";
  const sections = [
    "## Methods",
    "",
    methodsParagraph(provenance),
    "",
    "## Citation",
    "",
    apaReference(),
    "",
    "```bibtex",
    bibtex(),
    "```",
  ];
  const checksums = checksumTable(provenance);
  if (checksums) {
    sections.push("", "## Supplementary: input checksums", "", checksums);
  }
  return sections.join("\n");
}
