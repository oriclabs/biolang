//! Hidden Markov models: decoding, likelihood, and posterior probabilities.
//!
//! An HMM is the tool behind gene finding, profile search, segmentation of a
//! genome into states, and CNV calling — and until now nothing in the tree
//! could run one, which made Rosalind's eleven BA10 problems unreachable and
//! left a conspicuous hole for a language aimed at genomics.
//!
//! States and symbols are indices here; the mapping to and from names belongs
//! at the language boundary, so this module never allocates a string in a loop.
//!
//! The initial distribution is explicit rather than assumed. Rosalind's
//! formulations start uniform over the states, but a profile HMM does not, and
//! baking the uniform case in would make the general one unreachable.

/// A model over a fixed set of states and emitted symbols.
///
/// `transition[from][to]` and `emission[state][symbol]` are the usual row-wise
/// conventions: each row sums to one.
#[derive(Debug, Clone, PartialEq)]
pub struct Hmm {
    pub states: Vec<String>,
    pub symbols: Vec<String>,
    /// Probability of starting in each state.
    pub initial: Vec<f64>,
    /// `transition[from][to]`, `states × states`.
    pub transition: Vec<Vec<f64>>,
    /// `emission[state][symbol]`, `states × symbols`.
    pub emission: Vec<Vec<f64>>,
}

/// `ln` that treats zero as negative infinity instead of returning it as a NaN
/// source. A zero probability is a legitimate model entry — it means "never" —
/// and it has to compare as worse than every alternative rather than poison the
/// arithmetic.
fn ln(x: f64) -> f64 {
    if x > 0.0 {
        x.ln()
    } else {
        f64::NEG_INFINITY
    }
}

impl Hmm {
    /// A model whose states are equally likely to start.
    pub fn with_uniform_start(
        states: Vec<String>,
        symbols: Vec<String>,
        transition: Vec<Vec<f64>>,
        emission: Vec<Vec<f64>>,
    ) -> Self {
        let count = states.len();
        let share = if count == 0 { 0.0 } else { 1.0 / count as f64 };
        Self {
            states,
            symbols,
            initial: vec![share; count],
            transition,
            emission,
        }
    }

    /// The single most likely sequence of states, as state indices.
    ///
    /// Viterbi, in log space. Multiplying a few hundred probabilities underflows
    /// f64 to zero long before a chromosome-length observation ends, at which
    /// point every path ties at zero and the answer is whichever one the
    /// comparison happened to see first. Adding logs has the same argmax and
    /// cannot underflow.
    ///
    /// Note this is *not* the same as taking the most likely state at each
    /// position independently — see [`Hmm::posterior`]. Viterbi returns a path
    /// the model can actually produce; the position-wise maxima need not be
    /// connected by a transition of non-zero probability.
    pub fn viterbi(&self, observations: &[usize]) -> Vec<usize> {
        let count = self.states.len();
        let length = observations.len();
        if count == 0 || length == 0 {
            return Vec::new();
        }

        let mut score: Vec<f64> = (0..count)
            .map(|state| ln(self.initial[state]) + ln(self.emission[state][observations[0]]))
            .collect();
        // `came_from[t][s]` is the predecessor of state `s` on the best path
        // reaching it at step `t`.
        let mut came_from = vec![vec![0usize; count]; length];

        for step in 1..length {
            let mut next = vec![f64::NEG_INFINITY; count];
            for state in 0..count {
                let mut best = f64::NEG_INFINITY;
                let mut best_previous = 0usize;
                for previous in 0..count {
                    let candidate = score[previous] + ln(self.transition[previous][state]);
                    if candidate > best {
                        best = candidate;
                        best_previous = previous;
                    }
                }
                came_from[step][state] = best_previous;
                next[state] = best + ln(self.emission[state][observations[step]]);
            }
            score = next;
        }

        let mut end = 0usize;
        for state in 1..count {
            if score[state] > score[end] {
                end = state;
            }
        }

        let mut path = vec![0usize; length];
        path[length - 1] = end;
        for step in (1..length).rev() {
            path[step - 1] = came_from[step][path[step]];
        }
        path
    }

    /// `ln P(observations)` — the likelihood summed over every hidden path.
    ///
    /// The forward algorithm, rescaled at each step. The probability itself
    /// underflows for anything longer than a few hundred symbols, so the sum is
    /// normalised at every position and the log of each normaliser accumulated;
    /// the total is exact in log space regardless of the observation's length.
    pub fn log_likelihood(&self, observations: &[usize]) -> f64 {
        let count = self.states.len();
        if count == 0 || observations.is_empty() {
            // The empty observation has probability one: ln 1 = 0.
            return 0.0;
        }

        let mut alpha: Vec<f64> = (0..count)
            .map(|state| self.initial[state] * self.emission[state][observations[0]])
            .collect();
        let mut total: f64 = alpha.iter().sum();
        if total <= 0.0 {
            return f64::NEG_INFINITY;
        }
        for value in &mut alpha {
            *value /= total;
        }
        let mut accumulated = total.ln();

        for &symbol in &observations[1..] {
            let mut next = vec![0.0; count];
            for state in 0..count {
                let reached: f64 = (0..count)
                    .map(|previous| alpha[previous] * self.transition[previous][state])
                    .sum();
                next[state] = reached * self.emission[state][symbol];
            }
            total = next.iter().sum();
            if total <= 0.0 {
                // No path explains the observation.
                return f64::NEG_INFINITY;
            }
            for value in &mut next {
                *value /= total;
            }
            accumulated += total.ln();
            alpha = next;
        }

        accumulated
    }

    /// `P(observations)`.
    ///
    /// Convenience over [`Hmm::log_likelihood`]. Underflows to zero for a long
    /// observation, which is a property of the answer and not of the algorithm —
    /// reach for the log form when that matters.
    pub fn likelihood(&self, observations: &[usize]) -> f64 {
        self.log_likelihood(observations).exp()
    }

    /// `P(state at each position | the whole observation)`.
    ///
    /// Forward-backward, or "soft decoding": unlike Viterbi this conditions on
    /// the entire observation, including what comes *after* each position, so a
    /// later symbol can revise an earlier call. The result is a distribution per
    /// position, each row summing to one.
    ///
    /// Both passes are rescaled, and each row is renormalised at the end, so the
    /// scaling convention cancels and long observations behave.
    pub fn posterior(&self, observations: &[usize]) -> Vec<Vec<f64>> {
        let count = self.states.len();
        let length = observations.len();
        if count == 0 || length == 0 {
            return Vec::new();
        }

        let mut alpha = vec![vec![0.0f64; count]; length];
        let mut scale = vec![1.0f64; length];

        for state in 0..count {
            alpha[0][state] = self.initial[state] * self.emission[state][observations[0]];
        }
        let total: f64 = alpha[0].iter().sum();
        if total > 0.0 {
            scale[0] = total;
            for value in &mut alpha[0] {
                *value /= total;
            }
        }

        for step in 1..length {
            for state in 0..count {
                let reached: f64 = (0..count)
                    .map(|previous| alpha[step - 1][previous] * self.transition[previous][state])
                    .sum();
                alpha[step][state] = reached * self.emission[state][observations[step]];
            }
            let total: f64 = alpha[step].iter().sum();
            if total > 0.0 {
                scale[step] = total;
                for value in &mut alpha[step] {
                    *value /= total;
                }
            }
        }

        // Backward, sharing the forward pass's scale factors so the two stay in
        // step and the product below is the posterior up to one constant.
        let mut beta = vec![vec![0.0f64; count]; length];
        for value in &mut beta[length - 1] {
            *value = 1.0;
        }
        for step in (0..length - 1).rev() {
            for state in 0..count {
                let onward: f64 = (0..count)
                    .map(|next| {
                        self.transition[state][next]
                            * self.emission[next][observations[step + 1]]
                            * beta[step + 1][next]
                    })
                    .sum();
                beta[step][state] = onward / scale[step + 1];
            }
        }

        (0..length)
            .map(|step| {
                let combined: Vec<f64> = (0..count)
                    .map(|state| alpha[step][state] * beta[step][state])
                    .collect();
                let total: f64 = combined.iter().sum();
                if total > 0.0 {
                    combined.iter().map(|value| value / total).collect()
                } else {
                    vec![0.0; count]
                }
            })
            .collect()
    }

    /// `P(path)` — the probability of a hidden path on its own, ignoring what
    /// was emitted.
    pub fn path_probability(&self, path: &[usize]) -> f64 {
        match path.split_first() {
            None => 1.0,
            Some((&first, rest)) => {
                let mut probability = self.initial[first];
                let mut previous = first;
                for &state in rest {
                    probability *= self.transition[previous][state];
                    previous = state;
                }
                probability
            }
        }
    }

    /// `P(observations | path)` — the probability the given path emitted the
    /// given symbols, which is just the emissions multiplied together.
    ///
    /// Returns `None` when the two differ in length, since there is no such
    /// quantity to report.
    pub fn emission_probability(&self, observations: &[usize], path: &[usize]) -> Option<f64> {
        if observations.len() != path.len() {
            return None;
        }
        Some(
            path.iter()
                .zip(observations)
                .map(|(&state, &symbol)| self.emission[state][symbol])
                .product(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two-state model Rosalind uses for BA10C.
    fn ba10c_model() -> Hmm {
        Hmm::with_uniform_start(
            vec!["A".into(), "B".into()],
            vec!["x".into(), "y".into(), "z".into()],
            vec![vec![0.641, 0.359], vec![0.729, 0.271]],
            vec![vec![0.117, 0.691, 0.192], vec![0.097, 0.42, 0.483]],
        )
    }

    fn encode(text: &str, alphabet: &[String]) -> Vec<usize> {
        text.chars()
            .map(|c| {
                alphabet
                    .iter()
                    .position(|s| s == &c.to_string())
                    .expect("symbol outside the alphabet")
            })
            .collect()
    }

    #[test]
    fn ba10c_sample() {
        let model = ba10c_model();
        let observations = encode("xyxzzxyxyy", &model.symbols);
        let path: String = model
            .viterbi(&observations)
            .iter()
            .map(|&s| model.states[s].as_str())
            .collect();
        assert_eq!(path, "AAABBAAAAA");
    }

    #[test]
    fn ba10d_sample() {
        let model = Hmm::with_uniform_start(
            vec!["A".into(), "B".into()],
            vec!["x".into(), "y".into(), "z".into()],
            vec![vec![0.303, 0.697], vec![0.831, 0.169]],
            vec![vec![0.533, 0.065, 0.402], vec![0.342, 0.334, 0.324]],
        );
        let observations = encode("xzyyzzyzyy", &model.symbols);
        let published = 1.1005510319694847e-06;
        let got = model.likelihood(&observations);
        assert!(
            (got - published).abs() < 1e-16,
            "likelihood {got} disagrees with {published}"
        );
    }

    #[test]
    fn ba10j_sample() {
        let model = Hmm::with_uniform_start(
            vec!["A".into(), "B".into()],
            vec!["x".into(), "y".into(), "z".into()],
            vec![vec![0.911, 0.089], vec![0.228, 0.772]],
            vec![vec![0.356, 0.191, 0.453], vec![0.04, 0.467, 0.493]],
        );
        let observations = encode("zyxxxxyxzz", &model.symbols);
        let published = [
            0.5438, 0.6492, 0.9647, 0.9936, 0.9957, 0.9891, 0.9154, 0.964, 0.8737, 0.8167,
        ];
        for (row, expected) in model.posterior(&observations).iter().zip(published) {
            assert!(
                (row[0] - expected).abs() < 5e-5,
                "P(A) = {} against {expected}",
                row[0]
            );
            assert!(
                (row[0] + row[1] - 1.0).abs() < 1e-12,
                "the row does not sum to one"
            );
        }
    }

    #[test]
    fn ba10a_sample() {
        let model = Hmm::with_uniform_start(
            vec!["A".into(), "B".into()],
            Vec::new(),
            vec![vec![0.194, 0.806], vec![0.273, 0.727]],
            Vec::new(),
        );
        let path: Vec<usize> = "AABBBAABABAAAABBBBAABBABABBBAABBAAAABABAABBABABBAB"
            .chars()
            .map(|c| usize::from(c == 'B'))
            .collect();
        let got = model.path_probability(&path);
        assert!(
            (got - 5.01732865318e-19).abs() < 1e-30,
            "path probability {got}"
        );
    }

    #[test]
    fn ba10b_sample() {
        let model = Hmm::with_uniform_start(
            vec!["A".into(), "B".into()],
            vec!["x".into(), "y".into(), "z".into()],
            Vec::new(),
            vec![vec![0.612, 0.314, 0.074], vec![0.346, 0.317, 0.336]],
        );
        let observations = encode(
            "xxyzyxzzxzxyxyyzxxzzxxyyxxyxyzzxxyzyzxzxxyxyyzxxzx",
            &model.symbols,
        );
        let path: Vec<usize> = "BBBAAABABABBBBBBAAAAAABAAAABABABBBBBABAABABABABBBB"
            .chars()
            .map(|c| usize::from(c == 'B'))
            .collect();
        assert_eq!(
            path.len(),
            observations.len(),
            "the sample's path and string are both 50"
        );
        let got = model
            .emission_probability(&observations, &path)
            .expect("same length");
        assert!((got - 1.93157070893e-28).abs() < 1e-38, "emission {got}");
    }

    #[test]
    fn viterbi_beats_every_path_it_could_have_chosen() {
        // On a short observation the best path can be found by brute force, and
        // it must agree with what Viterbi returns.
        let model = ba10c_model();
        let observations = encode("xyxz", &model.symbols);
        let best = model.viterbi(&observations);
        let joint = |path: &[usize]| {
            model.path_probability(path)
                * model
                    .emission_probability(&observations, path)
                    .expect("same length")
        };
        for candidate in 0..(1u32 << observations.len()) {
            let path: Vec<usize> = (0..observations.len())
                .map(|i| ((candidate >> i) & 1) as usize)
                .collect();
            assert!(
                joint(&path) <= joint(&best) + 1e-15,
                "{path:?} beats the Viterbi path {best:?}"
            );
        }
    }

    #[test]
    fn likelihood_is_the_sum_over_all_paths() {
        let model = ba10c_model();
        let observations = encode("xyxz", &model.symbols);
        let mut total = 0.0;
        for candidate in 0..(1u32 << observations.len()) {
            let path: Vec<usize> = (0..observations.len())
                .map(|i| ((candidate >> i) & 1) as usize)
                .collect();
            total += model.path_probability(&path)
                * model
                    .emission_probability(&observations, &path)
                    .expect("same length");
        }
        assert!(
            (model.likelihood(&observations) - total).abs() < 1e-15,
            "forward gives {} against {total} enumerated",
            model.likelihood(&observations)
        );
    }

    #[test]
    fn a_zero_probability_does_not_produce_a_nan() {
        // "Never" has to stay finite-or-negative-infinity, not NaN.
        let model = Hmm::with_uniform_start(
            vec!["A".into(), "B".into()],
            vec!["x".into(), "y".into()],
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        );
        let path = model.viterbi(&[0, 0, 0]);
        assert_eq!(path, vec![0, 0, 0], "only A can emit x");
        assert!(!model.log_likelihood(&[0, 1]).is_nan());
        // A -> y is impossible and so is A -> B, so nothing explains "xy".
        assert_eq!(model.log_likelihood(&[0, 1]), f64::NEG_INFINITY);
    }

    #[test]
    fn empty_and_single() {
        let model = ba10c_model();
        assert!(model.viterbi(&[]).is_empty());
        assert!(model.posterior(&[]).is_empty());
        assert_eq!(model.log_likelihood(&[]), 0.0);
        assert_eq!(model.viterbi(&[1]).len(), 1);
        // One position, conditioned on itself, is just the normalised joint.
        let single = model.posterior(&[1]);
        assert!((single[0].iter().sum::<f64>() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn posterior_rows_sum_to_one_over_a_long_observation() {
        // The scaled passes exist for this: unscaled, 2000 steps underflows and
        // every row comes back as zeros.
        let model = ba10c_model();
        let observations: Vec<usize> = (0..2000).map(|i| i % 3).collect();
        for row in model.posterior(&observations) {
            assert!(
                (row.iter().sum::<f64>() - 1.0).abs() < 1e-9,
                "a row stopped summing to one"
            );
        }
        assert!(
            model.log_likelihood(&observations) < -1000.0,
            "should be a very small probability"
        );
        assert!(model.log_likelihood(&observations).is_finite());
    }
}
