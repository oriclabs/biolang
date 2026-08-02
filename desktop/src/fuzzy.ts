/**
 * Subsequence matching for the command palette and quick open.
 *
 * The palette used to filter with `label.includes(query)`, so "raf" could not
 * find "Run Active File" and nothing could rank one hit above another. This
 * scores a candidate by where its matched characters land — start of a word,
 * after a separator, in a run — which is what makes short abbreviations feel
 * like they hit the thing you meant.
 */

export type FuzzyMatch = {
  /** Higher is better. Only meaningful when comparing against the same query. */
  score: number;
  /** Indices into the target that the query matched, ascending. */
  positions: number[];
};

const SEPARATORS = new Set(["/", "\\", "_", "-", " ", ".", ":", ">", "@"]);

/** Points for a character that starts a word rather than sitting inside one. */
function boundaryBonus(target: string, index: number): number {
  if (index === 0) return 9;
  const previous = target[index - 1];
  if (SEPARATORS.has(previous)) return 8;
  const current = target[index];
  const camel = current >= "A" && current <= "Z" && previous >= "a" && previous <= "z";
  return camel ? 7 : 0;
}

function scorePositions(target: string, positions: number[]): number {
  let score = 0;
  let previous = -2;
  for (const index of positions) {
    score += 10 + boundaryBonus(target, index);
    // A run of adjacent characters is far stronger evidence than the same
    // characters scattered across the string.
    if (index === previous + 1) score += 14;
    previous = index;
  }
  // Prefer matches that start early and targets that are not padded with
  // unrelated text, so "kmer" ranks kmer.bl above deep/nested/kmer_table.bl.
  score -= Math.min(positions[0] ?? 0, 24);
  score -= Math.min(Math.floor(target.length / 8), 14);
  return score;
}

/** Earliest match: scan forward, taking the first candidate for each character. */
function matchForward(needle: string, haystack: string): number[] | undefined {
  const positions: number[] = [];
  let cursor = 0;
  for (const character of needle) {
    const found = haystack.indexOf(character, cursor);
    if (found < 0) return undefined;
    positions.push(found);
    cursor = found + 1;
  }
  return positions;
}

/** Latest match: scan backward, which finds the tightest run near the end. */
function matchBackward(needle: string, haystack: string): number[] | undefined {
  const positions: number[] = [];
  let cursor = haystack.length - 1;
  for (let index = needle.length - 1; index >= 0; index -= 1) {
    const found = haystack.lastIndexOf(needle[index], cursor);
    if (found < 0) return undefined;
    positions.unshift(found);
    cursor = found - 1;
  }
  return positions;
}

/**
 * Score `query` against `target`, or return undefined when the query is not a
 * subsequence of it.
 *
 * Both a forward and a backward pass run because they fail in opposite
 * directions: forward finds "Run Active File" from "raf" but drags a path query
 * to the first directory that happens to share a letter, while backward lands
 * on the file name. Taking the better of the two costs one extra linear scan.
 */
export function fuzzyMatch(query: string, target: string): FuzzyMatch | undefined {
  const needle = query.toLowerCase().replace(/\s+/g, "");
  if (!needle) return { score: 0, positions: [] };
  if (needle.length > target.length) return undefined;

  const haystack = target.toLowerCase();
  const forward = matchForward(needle, haystack);
  if (!forward) return undefined;
  const backward = matchBackward(needle, haystack);

  const forwardScore = scorePositions(target, forward);
  const backwardScore = backward ? scorePositions(target, backward) : -Infinity;
  return backward && backwardScore > forwardScore
    ? { score: backwardScore, positions: backward }
    : { score: forwardScore, positions: forward };
}

/**
 * Split `target` into alternating unmatched and matched runs so the palette can
 * show which characters the query is responsible for.
 */
export function highlightSegments(
  target: string,
  positions: number[],
): Array<{ text: string; matched: boolean }> {
  if (!positions.length) return [{ text: target, matched: false }];
  const matched = new Set(positions);
  const segments: Array<{ text: string; matched: boolean }> = [];
  let start = 0;
  let current = matched.has(0);
  for (let index = 1; index <= target.length; index += 1) {
    const flag = matched.has(index);
    if (index === target.length || flag !== current) {
      segments.push({ text: target.slice(start, index), matched: current });
      start = index;
      current = flag;
    }
  }
  return segments;
}
