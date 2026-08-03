/**
 * The same analysis in Python, R, and BioLang.
 *
 * The workbench never made its own case. Someone arriving from pandas or dplyr
 * has no reason to believe a new language is worth the switching cost, and a
 * feature list does not answer that — a task they recognise, written three
 * ways, does.
 *
 * The comparison is deliberately fair. The Python and R versions are what a
 * competent person would actually write, not padded strawmen, and dplyr in
 * particular is close on line count. The real difference is the dependency
 * column: the bio types and readers are in the language, so there is nothing
 * to install and nothing to keep in sync.
 *
 * The BioLang source is the source the Run button executes, and a test asserts
 * it produces the table this claims. A comparison that does not run would be
 * worse than none.
 */

export type ComparisonVariant = {
  id: "biolang" | "python" | "r";
  label: string;
  /** What you need before the code will run at all. */
  dependencies: string;
  source: string;
};

export const comparisonTask =
  "Read a FASTA, compute length and GC for each record, keep the long ones, and rank by GC.";

export const comparisonVariants: ComparisonVariant[] = [
  {
    id: "biolang",
    label: "BioLang",
    dependencies: "Nothing to install",
    source: `read_fasta("data/sequences.fasta")
    |> map(|r| {id: r.id, length: seq_len(r.seq), gc: gc_content(r.seq)})
    |> filter(|r| r.length >= 30)
    |> table()
    |> arrange("-gc")
`,
  },
  {
    id: "python",
    label: "Python",
    dependencies: "biopython, pandas",
    source: `from Bio import SeqIO
from Bio.SeqUtils import gc_fraction
import pandas as pd

records = []
for record in SeqIO.parse("data/sequences.fasta", "fasta"):
    records.append({
        "id": record.id,
        "length": len(record.seq),
        "gc": gc_fraction(record.seq),
    })

df = pd.DataFrame(records)
df = df[df["length"] >= 30]
df = df.sort_values("gc", ascending=False)
`,
  },
  {
    id: "r",
    label: "R",
    dependencies: "Biostrings, dplyr",
    source: `library(Biostrings)
library(dplyr)

seqs <- readDNAStringSet("data/sequences.fasta")

tibble(
  id = names(seqs),
  length = width(seqs),
  gc = letterFrequency(seqs, "GC", as.prob = TRUE)[, 1]
) |>
  filter(length >= 30) |>
  arrange(desc(gc))
`,
  },
];

/** The runnable variant, which is the one the Run button executes. */
export function biolangVariant(): ComparisonVariant {
  return comparisonVariants.find((variant) => variant.id === "biolang")!;
}

/** Non-blank lines, for the line-count note under each variant. */
export function lineCount(source: string): number {
  return source.split("\n").filter((line) => line.trim().length > 0).length;
}
