//! Theoretical mass spectra of peptides.
//!
//! A mass spectrometer breaks copies of a peptide at every position and weighs
//! the pieces. Sequencing runs that backwards: given the weights, work out the
//! peptide. Everything here is in integer daltons, which is the convention the
//! sequencing algorithms use — real instrument readings carry decimals, and
//! rounding them is what makes the combinatorial search finite.
//!
//! Peptides are lists of masses rather than letters throughout. Leucine and
//! isoleucine both weigh 113 and glutamine and lysine both weigh 128, so a
//! spectrum cannot distinguish them; carrying letters would promise a
//! resolution the data does not have.

/// The integer mass of each amino acid, in the usual one-letter order.
pub const AMINO_ACID_MASSES: [(char, u32); 20] = [
    ('G', 57),
    ('A', 71),
    ('S', 87),
    ('P', 97),
    ('V', 99),
    ('T', 101),
    ('C', 103),
    ('I', 113),
    ('L', 113),
    ('N', 114),
    ('D', 115),
    ('K', 128),
    ('Q', 128),
    ('E', 129),
    ('M', 131),
    ('H', 137),
    ('F', 147),
    ('R', 156),
    ('Y', 163),
    ('W', 186),
];

/// The 18 distinct amino acid masses.
///
/// Eighteen, not twenty: I/L and K/Q collide. Search algorithms extend by mass,
/// so using twenty would explore each colliding pair twice for no gain.
pub fn distinct_masses() -> Vec<u32> {
    let mut masses: Vec<u32> = AMINO_ACID_MASSES.iter().map(|(_, mass)| *mass).collect();
    masses.sort_unstable();
    masses.dedup();
    masses
}

/// The mass of a single amino acid, by letter.
pub fn mass_of(residue: char) -> Option<u32> {
    AMINO_ACID_MASSES
        .iter()
        .find(|(letter, _)| *letter == residue.to_ascii_uppercase())
        .map(|(_, mass)| *mass)
}

/// Turn a peptide written as letters into the masses a spectrometer would see.
pub fn masses_of(peptide: &str) -> Option<Vec<u32>> {
    peptide.chars().map(mass_of).collect()
}

/// Running totals, so any subpeptide's mass is one subtraction.
fn prefix_masses(peptide: &[u32]) -> Vec<u32> {
    let mut prefix = Vec::with_capacity(peptide.len() + 1);
    prefix.push(0);
    for mass in peptide {
        prefix.push(prefix[prefix.len() - 1] + mass);
    }
    prefix
}

/// Every contiguous subpeptide's mass, sorted, including 0 and the whole.
pub fn linear_spectrum(peptide: &[u32]) -> Vec<u32> {
    let prefix = prefix_masses(peptide);
    let mut spectrum = vec![0];
    for start in 0..peptide.len() {
        for end in start + 1..=peptide.len() {
            spectrum.push(prefix[end] - prefix[start]);
        }
    }
    spectrum.sort_unstable();
    spectrum
}

/// The same for a peptide joined end to end.
///
/// A cyclic peptide has subpeptides that wrap past the end, and their masses are
/// what a spectrum of a cyclic peptide contains beyond the linear ones. Each
/// wrapping piece is the complement of a non-wrapping one, so it costs a
/// subtraction rather than another scan.
pub fn cyclic_spectrum(peptide: &[u32]) -> Vec<u32> {
    let prefix = prefix_masses(peptide);
    let total = *prefix.last().unwrap_or(&0);
    let mut spectrum = vec![0];
    for start in 0..peptide.len() {
        for end in start + 1..=peptide.len() {
            let piece = prefix[end] - prefix[start];
            spectrum.push(piece);
            // The wrap-around piece, which exists whenever this one is a proper
            // middle section.
            if start > 0 && end < peptide.len() {
                spectrum.push(total - piece);
            }
        }
    }
    spectrum.sort_unstable();
    spectrum
}

/// How many masses the peptide's spectrum and the observed one share.
///
/// Multiset intersection: a mass appearing twice in both counts twice. Treating
/// it as a set intersection instead would score a peptide that explains a
/// repeated mass once as well as one that explains it fully.
pub fn score(theoretical: &[u32], observed: &[u32]) -> usize {
    let mut mine = theoretical.to_vec();
    mine.sort_unstable();
    let mut theirs = observed.to_vec();
    theirs.sort_unstable();

    let mut shared = 0;
    let (mut i, mut j) = (0, 0);
    while i < mine.len() && j < theirs.len() {
        match mine[i].cmp(&theirs[j]) {
            std::cmp::Ordering::Equal => {
                shared += 1;
                i += 1;
                j += 1;
            }
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
        }
    }
    shared
}

/// Every positive pairwise difference in a spectrum, largest multiplicity first.
///
/// The differences between fragment masses are themselves amino acid masses, so
/// the ones occurring most often are the residues the peptide is most likely
/// built from — which is how sequencing an unknown peptide gets started without
/// assuming the standard twenty.
pub fn convolution(spectrum: &[u32]) -> Vec<u32> {
    let mut differences = Vec::new();
    for (i, &a) in spectrum.iter().enumerate() {
        for &b in &spectrum[..i] {
            if a != b {
                differences.push(a.abs_diff(b));
            }
        }
    }
    differences.sort_unstable();
    differences
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ba4c_sample() {
        let peptide = masses_of("LEQN").expect("known residues");
        assert_eq!(peptide, vec![113, 129, 128, 114]);
        assert_eq!(
            cyclic_spectrum(&peptide),
            vec![0, 113, 114, 128, 129, 227, 242, 242, 257, 355, 356, 370, 371, 484]
        );
    }

    #[test]
    fn ba4j_sample() {
        // The linear spectrum of the same peptide is a subset: it lacks the
        // pieces that wrap past the end.
        let peptide = masses_of("NQEL").expect("known residues");
        assert_eq!(
            linear_spectrum(&peptide),
            vec![0, 113, 114, 128, 129, 242, 242, 257, 370, 371, 484]
        );
    }

    #[test]
    fn cyclic_contains_every_linear_mass() {
        let peptide = masses_of("LEQN").expect("known residues");
        let linear = linear_spectrum(&peptide);
        let cyclic = cyclic_spectrum(&peptide);
        assert_eq!(
            score(&linear, &cyclic),
            linear.len(),
            "every linear fragment is also a cyclic one"
        );
        assert!(cyclic.len() > linear.len(), "and the wraps are extra");
    }

    #[test]
    fn score_counts_multiplicity() {
        // 242 appears twice in LEQN's cyclic spectrum. A spectrum listing it
        // once should score one for it, not two.
        let peptide = masses_of("LEQN").expect("known residues");
        let theoretical = cyclic_spectrum(&peptide);
        assert_eq!(score(&theoretical, &[242]), 1);
        assert_eq!(score(&theoretical, &[242, 242]), 2);
        assert_eq!(score(&theoretical, &[242, 242, 242]), 2, "only two exist");
    }

    #[test]
    fn there_are_eighteen_distinct_masses() {
        // I/L and K/Q collide, so a spectrum cannot tell them apart.
        assert_eq!(distinct_masses().len(), 18);
        assert_eq!(mass_of('I'), mass_of('L'));
        assert_eq!(mass_of('K'), mass_of('Q'));
        assert_eq!(mass_of('X'), None);
    }

    #[test]
    fn ba4h_sample() {
        // The published sample: the convolution of {0, 137, 186, 323}.
        let mut got = convolution(&[0, 137, 186, 323]);
        got.sort_unstable();
        assert_eq!(got, vec![49, 137, 137, 186, 186, 323]);
    }

    #[test]
    fn an_empty_peptide_has_only_the_zero_mass() {
        assert_eq!(linear_spectrum(&[]), vec![0]);
        assert_eq!(cyclic_spectrum(&[]), vec![0]);
    }

    #[test]
    fn a_single_residue_is_the_same_either_way() {
        // With one residue there is nothing to wrap around.
        assert_eq!(linear_spectrum(&[113]), vec![0, 113]);
        assert_eq!(cyclic_spectrum(&[113]), vec![0, 113]);
    }
}
