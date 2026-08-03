//! Sorting a permutation by reversals.
//!
//! Chromosomes rearrange by reversal — a segment is excised, flipped and
//! reinserted — so the fewest reversals separating two gene orders is a measure
//! of how far apart two genomes have drifted. Unlike the 2-break distance, which
//! has a closed form, this one has no useful formula at small sizes and is
//! computed by search.
//!
//! The search is what makes this belong in Rust rather than in BioLang. A
//! permutation of ten elements has 45 possible reversals and 3.6 million
//! reachable orders; a breadth-first search over that finishes in milliseconds
//! compiled and not at all interpreted.

use std::collections::HashMap;

/// A reversal of the closed interval `[from, to]`, zero-based.
pub type Reversal = (usize, usize);

fn apply(permutation: &[u8], (from, to): Reversal) -> Vec<u8> {
    let mut next = permutation.to_vec();
    next[from..=to].reverse();
    next
}

/// Every reversal of a permutation of this length.
///
/// Single-element intervals are excluded: reversing one element changes
/// nothing, and including them would inflate the branching factor by `n` for no
/// new states.
fn reversals(length: usize) -> Vec<Reversal> {
    (0..length)
        .flat_map(|from| (from + 1..length).map(move |to| (from, to)))
        .collect()
}

/// Relabel `source` so that `target` becomes the identity.
///
/// Sorting into an arbitrary permutation and sorting into the identity are the
/// same problem, and doing this once means the search only ever has one goal to
/// recognise.
fn relative_to(source: &[u8], target: &[u8]) -> Option<Vec<u8>> {
    if source.len() != target.len() {
        return None;
    }
    let mut rank = vec![0u8; target.len() + 1];
    for (position, &value) in target.iter().enumerate() {
        let slot = usize::from(value);
        if slot >= rank.len() {
            return None;
        }
        rank[slot] = position as u8;
    }
    source
        .iter()
        .map(|&value| {
            let slot = usize::from(value);
            (slot < rank.len()).then(|| rank[slot])
        })
        .collect()
}

/// The fewest reversals taking `source` to `target`, and one such sequence.
///
/// Bidirectional: the distance for length-10 permutations reaches 9, and a
/// one-sided search to depth 9 with a branching factor of 45 is hopeless.
/// Searching from both ends meets in the middle, so each side only reaches
/// depth 4 or 5 — the difference between millions of states and thousands.
///
/// Reversals are their own inverse, so the graph is undirected and the same
/// expansion serves both directions.
///
/// The intervals returned are zero-based and inclusive, and applying them to
/// `source` in order yields `target`.
pub fn sorting_reversals(source: &[u8], target: &[u8]) -> Option<Vec<Reversal>> {
    let start = relative_to(source, target)?;
    let length = start.len();
    let goal: Vec<u8> = (0..length as u8).collect();
    if start == goal {
        return Some(Vec::new());
    }

    let moves = reversals(length);

    // Each side maps a permutation to how it was reached: the previous
    // permutation and the reversal that produced it.
    let mut from_start: HashMap<Vec<u8>, Option<(Vec<u8>, Reversal)>> = HashMap::new();
    let mut from_goal: HashMap<Vec<u8>, Option<(Vec<u8>, Reversal)>> = HashMap::new();
    from_start.insert(start.clone(), None);
    from_goal.insert(goal.clone(), None);

    let mut start_edge = vec![start.clone()];
    let mut goal_edge = vec![goal.clone()];

    loop {
        if start_edge.is_empty() || goal_edge.is_empty() {
            return None;
        }
        // Always expand the smaller frontier, which is what keeps the two sides
        // balanced when the distance is odd.
        let expanding_start = start_edge.len() <= goal_edge.len();
        let (edge, seen, other) = if expanding_start {
            (&mut start_edge, &mut from_start, &from_goal)
        } else {
            (&mut goal_edge, &mut from_goal, &from_start)
        };

        let mut next_edge = Vec::new();
        let mut meeting: Option<Vec<u8>> = None;
        for permutation in edge.iter() {
            for &reversal in &moves {
                let neighbour = apply(permutation, reversal);
                if seen.contains_key(&neighbour) {
                    continue;
                }
                seen.insert(neighbour.clone(), Some((permutation.clone(), reversal)));
                if other.contains_key(&neighbour) {
                    meeting = Some(neighbour.clone());
                    break;
                }
                next_edge.push(neighbour);
            }
            if meeting.is_some() {
                break;
            }
        }

        if let Some(middle) = meeting {
            return Some(rebuild(&middle, &from_start, &from_goal));
        }
        *edge = next_edge;
    }
}

/// Walk both halves out from where the searches met.
fn rebuild(
    middle: &[u8],
    from_start: &HashMap<Vec<u8>, Option<(Vec<u8>, Reversal)>>,
    from_goal: &HashMap<Vec<u8>, Option<(Vec<u8>, Reversal)>>,
) -> Vec<Reversal> {
    let mut leading = Vec::new();
    let mut at = middle.to_vec();
    while let Some(Some((previous, reversal))) = from_start.get(&at) {
        leading.push(*reversal);
        at = previous.clone();
    }
    leading.reverse();

    // The goal side was built backwards, so its reversals are already in the
    // order they must be applied — reversals being self-inverse means no
    // adjustment beyond that.
    let mut trailing = Vec::new();
    let mut at = middle.to_vec();
    while let Some(Some((previous, reversal))) = from_goal.get(&at) {
        trailing.push(*reversal);
        at = previous.clone();
    }

    leading.extend(trailing);
    leading
}

/// The fewest reversals taking `source` to `target`.
pub fn reversal_distance(source: &[u8], target: &[u8]) -> Option<usize> {
    sorting_reversals(source, target).map(|steps| steps.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn distance(a: &[u8], b: &[u8]) -> usize {
        reversal_distance(a, b).expect("same length")
    }

    /// Applying the returned reversals to `source` really produces `target`.
    fn sorts(source: &[u8], target: &[u8]) -> bool {
        let steps = sorting_reversals(source, target).expect("same length");
        let mut current = source.to_vec();
        for step in steps {
            current = apply(&current, step);
        }
        current == target
    }

    #[test]
    fn rear_sample() {
        // The five pairs Rosalind publishes, with distances 9 4 5 7 0.
        let pairs: [(&[u8], &[u8], usize); 5] = [
            (
                &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
                &[3, 1, 5, 2, 7, 4, 9, 6, 10, 8],
                9,
            ),
            (
                &[3, 10, 8, 2, 5, 4, 7, 1, 6, 9],
                &[5, 2, 3, 1, 7, 4, 10, 8, 6, 9],
                4,
            ),
            (
                &[8, 6, 7, 9, 4, 1, 3, 10, 2, 5],
                &[8, 2, 7, 6, 9, 1, 5, 3, 10, 4],
                5,
            ),
            (
                &[3, 9, 10, 4, 1, 8, 6, 7, 5, 2],
                &[2, 9, 8, 5, 1, 7, 3, 4, 6, 10],
                7,
            ),
            (
                &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
                &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
                0,
            ),
        ];
        for (source, target, expected) in pairs {
            assert_eq!(
                distance(source, target),
                expected,
                "{source:?} -> {target:?}"
            );
            assert!(sorts(source, target), "the reversals must actually sort it");
        }
    }

    #[test]
    fn sort_sample() {
        let source = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let target = [1, 8, 9, 3, 2, 7, 6, 5, 4, 10];
        let steps = sorting_reversals(&source, &target).expect("same length");
        assert_eq!(steps.len(), 2);
        assert!(sorts(&source, &target));
    }

    #[test]
    fn a_permutation_is_no_distance_from_itself() {
        let same = [1, 2, 3, 4, 5];
        assert_eq!(distance(&same, &same), 0);
        assert!(sorting_reversals(&same, &same)
            .expect("same length")
            .is_empty());
    }

    #[test]
    fn one_reversal_is_distance_one() {
        // Reversing a single interval cannot be undone in fewer than one move,
        // and never needs more.
        assert_eq!(distance(&[1, 2, 3, 4, 5], &[1, 4, 3, 2, 5]), 1);
        assert_eq!(distance(&[1, 2, 3, 4, 5], &[5, 4, 3, 2, 1]), 1);
    }

    #[test]
    fn the_distance_is_symmetric() {
        // Reversals are self-inverse, so sorting either way costs the same.
        let a = [3, 1, 5, 2, 4];
        let b = [1, 2, 3, 4, 5];
        assert_eq!(distance(&a, &b), distance(&b, &a));
    }

    #[test]
    fn mismatched_lengths_have_no_distance() {
        assert!(reversal_distance(&[1, 2, 3], &[1, 2]).is_none());
    }

    #[test]
    fn sorting_into_a_non_identity_target_works() {
        // The relabelling step is what makes this the same problem as sorting
        // into the identity; if it were wrong this would disagree.
        let source = [4, 1, 3, 2, 5];
        let target = [2, 5, 1, 4, 3];
        assert!(sorts(&source, &target));
    }
}
