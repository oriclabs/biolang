/**
 * Facts about a sequence literal, for the editor hover.
 *
 * These mirror `bio-core::seq_ops` so the numbers on hover are the numbers the
 * program will compute. Where the Rust side has a choice — GC over the whole
 * length including ambiguity codes, complement passing unknown bases through —
 * this follows it rather than doing something more defensible in isolation.
 */

export type Molecule = "dna" | "rna" | "protein";

/** Matches `dna"ACGT"`, the only sequence spelling the lexer recognises. */
const LITERAL = /\b(dna|rna|protein)"([^"\\]*)"/g;

export type SequenceLiteral = {
  molecule: Molecule;
  sequence: string;
  /** Column of the opening `d`/`r`/`p`, 1-based, as Monaco counts. */
  startColumn: number;
  /** Column just past the closing quote, 1-based. */
  endColumn: number;
};

/** Find the sequence literal covering `column` on `line`, if there is one. */
export function literalAt(line: string, column: number): SequenceLiteral | undefined {
  LITERAL.lastIndex = 0;
  for (let match = LITERAL.exec(line); match; match = LITERAL.exec(line)) {
    const startColumn = match.index + 1;
    const endColumn = startColumn + match[0].length;
    if (column >= startColumn && column <= endColumn) {
      return {
        molecule: match[1] as Molecule,
        sequence: match[2],
        startColumn,
        endColumn,
      };
    }
  }
  return undefined;
}

const DNA_COMPLEMENT: Record<string, string> = {
  A: "T", T: "A", G: "C", C: "G",
  a: "t", t: "a", g: "c", c: "g",
};

const RNA_COMPLEMENT: Record<string, string> = {
  A: "U", U: "A", G: "C", C: "G",
  a: "u", u: "a", g: "c", c: "g",
};

/** Unknown bases pass through unchanged, matching `complement_dna`. */
export function reverseComplement(sequence: string, molecule: Molecule): string {
  const table = molecule === "rna" ? RNA_COMPLEMENT : DNA_COMPLEMENT;
  return [...sequence].reverse().map((base) => table[base] ?? base).join("");
}

/** Fraction of the full length that is G or C, matching `gc_content`. */
export function gcContent(sequence: string): number {
  if (!sequence.length) return 0;
  const gc = [...sequence].filter((base) => "GCgc".includes(base)).length;
  return gc / sequence.length;
}

const CODONS: Record<string, string> = {
  UUU: "F", UUC: "F",
  UUA: "L", UUG: "L", CUU: "L", CUC: "L", CUA: "L", CUG: "L",
  AUU: "I", AUC: "I", AUA: "I",
  AUG: "M",
  GUU: "V", GUC: "V", GUA: "V", GUG: "V",
  UCU: "S", UCC: "S", UCA: "S", UCG: "S", AGU: "S", AGC: "S",
  CCU: "P", CCC: "P", CCA: "P", CCG: "P",
  ACU: "T", ACC: "T", ACA: "T", ACG: "T",
  GCU: "A", GCC: "A", GCA: "A", GCG: "A",
  UAU: "Y", UAC: "Y",
  UAA: "*", UAG: "*", UGA: "*",
  CAU: "H", CAC: "H",
  CAA: "Q", CAG: "Q",
  AAU: "N", AAC: "N",
  AAA: "K", AAG: "K",
  GAU: "D", GAC: "D",
  GAA: "E", GAG: "E",
  UGU: "C", UGC: "C",
  UGG: "W",
  CGU: "R", CGC: "R", CGA: "R", CGG: "R", AGA: "R", AGG: "R",
  GGU: "G", GGC: "G", GGA: "G", GGG: "G",
};

/**
 * Translate every whole codon, `X` for unknown and `*` for stops.
 *
 * This is `bio_core::translate`, not the `translate` builtin — the builtin stops
 * at the first stop codon. Showing the stop is the more useful thing on hover,
 * since spotting a premature one is half of why you look.
 */
export function translate(sequence: string): string {
  const rna = sequence.toUpperCase().replaceAll("T", "U");
  let protein = "";
  for (let index = 0; index + 3 <= rna.length; index += 3) {
    protein += CODONS[rna.slice(index, index + 3)] ?? "X";
  }
  return protein;
}

/** Counts for every distinct residue, ordered by descending frequency. */
export function composition(sequence: string): Array<[string, number]> {
  const counts = new Map<string, number>();
  for (const residue of sequence.toUpperCase()) {
    counts.set(residue, (counts.get(residue) ?? 0) + 1);
  }
  return [...counts.entries()].sort((left, right) => right[1] - left[1]);
}

/** Keep long sequences from turning the hover card into a wall of bases. */
function elide(text: string, limit = 60): string {
  return text.length <= limit ? text : `${text.slice(0, limit)}… (${text.length})`;
}

/**
 * Markdown for the hover card.
 *
 * Nucleotides get GC, reverse complement, and the frame-1 translation, which
 * are the three things you actually squint at a literal to work out. Proteins
 * get composition instead — the others are meaningless for them.
 */
export function describeLiteral(literal: SequenceLiteral): string[] {
  const { molecule, sequence } = literal;
  const label = molecule === "dna" ? "DNA" : molecule === "rna" ? "RNA" : "Protein";
  const lines = [`**${label} literal** — ${sequence.length} ${molecule === "protein" ? "residues" : "bases"}`];

  if (molecule === "protein") {
    const top = composition(sequence)
      .slice(0, 6)
      .map(([residue, count]) => `${residue}×${count}`)
      .join("  ");
    if (top) lines.push(`Composition: ${top}`);
    return lines;
  }

  lines.push(`GC content: ${(gcContent(sequence) * 100).toFixed(1)}%`);
  lines.push("");
  lines.push("Reverse complement");
  lines.push(`\`\`\`biolang\n${elide(reverseComplement(sequence, molecule))}\n\`\`\``);

  const protein = translate(sequence);
  if (protein) {
    const remainder = sequence.length % 3;
    const frame = remainder
      ? `Translation (frame 1, ${remainder} trailing base${remainder === 1 ? "" : "s"} ignored)`
      : "Translation (frame 1)";
    lines.push(frame);
    lines.push(`\`\`\`biolang\n${elide(protein)}\n\`\`\``);
  }
  return lines;
}
