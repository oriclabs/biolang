/**
 * Names from R and Python that map onto BioLang builtins.
 *
 * The table vocabulary was already chosen to match dplyr and pandas — `mutate`,
 * `select`, `arrange`, `pivot_longer`, `left_join` are all present — but a
 * newcomer types the spelling they know and gets nothing, because completion
 * only matches BioLang names. These let the editor answer `%>%`, `read.csv`,
 * and `value_counts` with the BioLang equivalent, which turns an existing
 * asset into a migration path.
 *
 * Only genuine equivalents belong here. Suggesting a near-miss is worse than
 * suggesting nothing: it produces code that runs and is wrong.
 */

export type Alias = {
  /** What the newcomer types. */
  foreign: string;
  /** Where it comes from, shown so the suggestion explains itself. */
  origin: "dplyr" | "tidyr" | "pandas" | "base R" | "Bioconductor" | "Biopython";
  /** The BioLang builtin to insert. */
  biolang: string;
  /** Anything that does not carry over cleanly. */
  note?: string;
};

export const aliases: Alias[] = [
  // Pipes and assignment — the first things anyone types.
  { foreign: "%>%", origin: "dplyr", biolang: "|>" },
  { foreign: "%in%", origin: "base R", biolang: "contains" },
  { foreign: "<-", origin: "base R", biolang: "let" },

  // dplyr verbs that map one to one.
  { foreign: "summarise", origin: "dplyr", biolang: "summarize" },
  { foreign: "slice_head", origin: "dplyr", biolang: "head" },
  { foreign: "slice_tail", origin: "dplyr", biolang: "tail" },
  { foreign: "n_distinct", origin: "dplyr", biolang: "distinct" },
  { foreign: "bind_rows", origin: "dplyr", biolang: "concat", note: "concat row-binds tables; bind_cols joins them side by side" },

  // tidyr.
  { foreign: "gather", origin: "tidyr", biolang: "pivot_longer" },
  { foreign: "spread", origin: "tidyr", biolang: "pivot_wider" },
  { foreign: "melt", origin: "pandas", biolang: "pivot_longer" },
  { foreign: "pivot", origin: "pandas", biolang: "pivot_wider" },

  // pandas methods.
  { foreign: "read_table", origin: "pandas", biolang: "read_tsv" },
  { foreign: "to_csv", origin: "pandas", biolang: "write_csv" },
  { foreign: "drop_duplicates", origin: "pandas", biolang: "distinct" },
  { foreign: "sort_values", origin: "pandas", biolang: "arrange", note: "prefix a column with - for descending" },
  { foreign: "groupby", origin: "pandas", biolang: "group_by" },
  { foreign: "assign", origin: "pandas", biolang: "mutate" },
  { foreign: "merge", origin: "pandas", biolang: "left_join", note: "inner_join and anti_join are also available" },
  { foreign: "info", origin: "pandas", biolang: "glimpse" },
  { foreign: "str", origin: "base R", biolang: "glimpse", note: "BioLang str() converts a value to a string; glimpse() is the structure display" },
  { foreign: "shape", origin: "pandas", biolang: "nrow", note: "dim() is for matrices; tables use nrow() and ncol()" },
  { foreign: "columns", origin: "pandas", biolang: "colnames" },

  // base R I/O.
  { foreign: "read.csv", origin: "base R", biolang: "read_csv" },
  { foreign: "read.table", origin: "base R", biolang: "read_tsv" },
  { foreign: "write.csv", origin: "base R", biolang: "write_csv" },
  { foreign: "sapply", origin: "base R", biolang: "map" },
  { foreign: "lapply", origin: "base R", biolang: "map" },
  { foreign: "Reduce", origin: "base R", biolang: "reduce" },
  { foreign: "paste0", origin: "base R", biolang: "concat" },
  { foreign: "seq_along", origin: "base R", biolang: "range" },

  // Bioinformatics libraries, where the BioLang equivalent is a builtin.
  { foreign: "readDNAStringSet", origin: "Bioconductor", biolang: "read_fasta" },
  { foreign: "readFastq", origin: "Bioconductor", biolang: "read_fastq" },
  { foreign: "readVcf", origin: "Bioconductor", biolang: "read_vcf" },
  { foreign: "reverseComplement", origin: "Bioconductor", biolang: "reverse_complement" },
  { foreign: "alphabetFrequency", origin: "Bioconductor", biolang: "base_counts" },
  { foreign: "SeqIO.parse", origin: "Biopython", biolang: "read_fasta", note: "read_fastq for FASTQ" },
  { foreign: "gc_fraction", origin: "Biopython", biolang: "gc_content" },
];

/** Aliases whose foreign name starts with `prefix`, case-insensitively. */
export function matchingAliases(prefix: string): Alias[] {
  const needle = prefix.toLowerCase();
  if (needle.length < 2) return [];
  return aliases.filter((alias) => alias.foreign.toLowerCase().startsWith(needle));
}

/** Documentation shown on an alias suggestion. */
export function aliasDocumentation(alias: Alias): string {
  const lines = [`\`${alias.foreign}\` in ${alias.origin} is \`${alias.biolang}\` in BioLang.`];
  if (alias.note) lines.push(alias.note);
  return lines.join("\n\n");
}
