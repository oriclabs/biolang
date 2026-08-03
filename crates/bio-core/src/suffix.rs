//! Suffix arrays and the LCP array beside them.
//!
//! These are the foundation the string problems stand on: repeats, longest
//! common substrings, the Burrows-Wheeler transform, and pattern matching that
//! does not rescan the text. Nothing in the tree could build any of them, which
//! is what an external problem set is good at showing — Rosalind's Textbook
//! Track named suffix structures as one of three gaps, and four of its
//! Stronghold problems wait on the same thing.
//!
//! Positions are 0-based, and no sentinel is added. Rosalind's formulations put
//! the `$` in the input themselves, so inventing one here would shift every
//! answer by one and silently disagree with the published output.

/// Sort the suffixes of `text`, returning their starting positions.
///
/// Prefix doubling: sort by the first character, then repeatedly sort by twice
/// as much, reusing the previous ranks so each round is a sort of pairs rather
/// than of strings. That is O(n log² n), against O(n² log n) for sorting the
/// suffixes as strings — which is fine for a Rosalind input of a kilobase and
/// not fine for a chromosome, and this is a language for chromosomes.
///
/// SA-IS would make it linear. It is also ten times the code and far easier to
/// get subtly wrong, so it belongs in a later pass with these tests to hold it.
pub fn suffix_array(text: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    let n = bytes.len();
    if n == 0 {
        return Vec::new();
    }

    let mut sa: Vec<usize> = (0..n).collect();
    // Rank by first character to begin with. i64 so the "past the end" rank can
    // be -1 and sort before every real character.
    let mut rank: Vec<i64> = bytes.iter().map(|&b| b as i64).collect();
    let mut next = vec![0i64; n];

    let mut span = 1usize;
    while span < n {
        // The key for a suffix is its own rank paired with the rank of the
        // suffix `span` further on — which is what makes each round double the
        // length being compared.
        let key = |rank: &[i64], i: usize| -> (i64, i64) {
            (rank[i], if i + span < n { rank[i + span] } else { -1 })
        };

        sa.sort_by(|&a, &b| key(&rank, a).cmp(&key(&rank, b)));

        next[sa[0]] = 0;
        for position in 1..n {
            let previous = key(&rank, sa[position - 1]);
            let current = key(&rank, sa[position]);
            next[sa[position]] = next[sa[position - 1]] + i64::from(previous != current);
        }
        rank.copy_from_slice(&next);

        // Every suffix has its own rank, so no further round can reorder them.
        if rank[sa[n - 1]] == (n - 1) as i64 {
            break;
        }
        span *= 2;
    }

    sa
}

/// Longest common prefix between each suffix and the one before it in `sa`.
///
/// `lcp[i]` compares `sa[i]` with `sa[i - 1]`; `lcp[0]` is 0, having nothing
/// before it. Kasai's algorithm, which is linear because the match length can
/// only fall by one between consecutive text positions, so the total it walks
/// back is bounded by the total it walks forward.
pub fn lcp_array(text: &str, sa: &[usize]) -> Vec<usize> {
    let bytes = text.as_bytes();
    let n = bytes.len();
    if n == 0 || sa.len() != n {
        return vec![0; sa.len()];
    }

    let mut rank = vec![0usize; n];
    for (position, &start) in sa.iter().enumerate() {
        rank[start] = position;
    }

    let mut lcp = vec![0usize; n];
    let mut matched = 0usize;
    for start in 0..n {
        if rank[start] == 0 {
            matched = 0;
            continue;
        }
        let previous = sa[rank[start] - 1];
        while start + matched < n
            && previous + matched < n
            && bytes[start + matched] == bytes[previous + matched]
        {
            matched += 1;
        }
        lcp[rank[start]] = matched;
        matched = matched.saturating_sub(1);
    }

    lcp
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The answer Rosalind publishes for BA9G's sample.
    #[test]
    fn ba9g_sample() {
        let sa = suffix_array("AACGATAGCGGTAGA$");
        assert_eq!(
            sa,
            vec![15, 14, 0, 1, 12, 6, 4, 2, 8, 13, 3, 7, 9, 10, 11, 5]
        );
    }

    #[test]
    fn banana() {
        // Suffixes of "banana$", sorted: $, a$, ana$, anana$, banana$, na$, nana$
        assert_eq!(suffix_array("banana$"), vec![6, 5, 3, 1, 0, 4, 2]);
    }

    #[test]
    fn empty_and_single() {
        assert_eq!(suffix_array(""), Vec::<usize>::new());
        assert_eq!(suffix_array("A"), vec![0]);
    }

    #[test]
    fn every_suffix_appears_once_and_in_order() {
        let text = "MISSISSIPPI$";
        let sa = suffix_array(text);
        let mut seen = sa.clone();
        seen.sort_unstable();
        assert_eq!(seen, (0..text.len()).collect::<Vec<_>>());
        for pair in sa.windows(2) {
            assert!(
                text[pair[0]..] < text[pair[1]..],
                "suffixes out of order at {pair:?}"
            );
        }
    }

    #[test]
    fn lcp_matches_the_suffixes_it_describes() {
        let text = "banana$";
        let sa = suffix_array(text);
        let lcp = lcp_array(text, &sa);
        assert_eq!(lcp[0], 0, "nothing precedes the first suffix");
        for i in 1..sa.len() {
            let a = &text[sa[i - 1]..];
            let b = &text[sa[i]..];
            let shared = a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count();
            assert_eq!(lcp[i], shared, "lcp[{i}] disagrees with the suffixes");
        }
    }

    #[test]
    fn a_run_of_one_character_is_handled() {
        // Worst case for prefix doubling: every suffix shares a prefix with the
        // next, so the ranks only separate on the last round.
        let sa = suffix_array("AAAAAAAA");
        assert_eq!(sa, vec![7, 6, 5, 4, 3, 2, 1, 0]);
        let lcp = lcp_array("AAAAAAAA", &sa);
        assert_eq!(lcp, vec![0, 1, 2, 3, 4, 5, 6, 7]);
    }
}
