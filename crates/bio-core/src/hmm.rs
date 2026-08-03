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

/// Turn each row of counts into a distribution.
///
/// A row that saw nothing has no evidence to normalise, and dividing by its zero
/// total would give NaN. Those rows become uniform — the convention Rosalind's
/// parameter-estimation problems use, and the only choice that leaves every row
/// summing to one.
fn normalise_rows(rows: &mut [Vec<f64>]) {
    for row in rows.iter_mut() {
        let total: f64 = row.iter().sum();
        if total > 0.0 {
            for value in row.iter_mut() {
                *value /= total;
            }
        } else if !row.is_empty() {
            let share = 1.0 / row.len() as f64;
            row.fill(share);
        }
    }
}

/// Turn each row of counts into a distribution, leaving a row that saw nothing
/// at zero.
///
/// The other half of the convention in [`normalise_rows`]. Which one is right
/// depends on what the row means: an unvisited state in a *profile* has no
/// transitions to describe, whereas an unvisited state in parameter estimation
/// still needs a usable row.
fn normalise_rows_or_zero(rows: &mut [Vec<f64>]) {
    for row in rows.iter_mut() {
        let total: f64 = row.iter().sum();
        if total > 0.0 {
            for value in row.iter_mut() {
                *value /= total;
            }
        }
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
        let (alpha, beta, _) = self.scaled_passes(observations);

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

    /// The rescaled forward and backward tables, and the scale factor used at
    /// each position.
    ///
    /// Shared by [`Hmm::posterior`] and [`Hmm::baum_welch`], which need the same
    /// two passes and would otherwise drift apart — the scaling convention has
    /// to match between them or the products below are off by a factor that
    /// varies with position.
    fn scaled_passes(&self, observations: &[usize]) -> (Vec<Vec<f64>>, Vec<Vec<f64>>, Vec<f64>) {
        let count = self.states.len();
        let length = observations.len();

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

        (alpha, beta, scale)
    }

    /// Build a profile HMM from a multiple alignment.
    ///
    /// A profile HMM is how a family of related sequences becomes something a
    /// new sequence can be scored against: each conserved column gets a match
    /// state, and the gaps become insertions and deletions with costs learned
    /// from the alignment rather than picked by hand. It is what Pfam and HMMER
    /// search with.
    ///
    /// A column is conserved — a match column — when the fraction of gaps in it
    /// is below `threshold`. Everything else is an insertion, which is what keeps
    /// the model's length at the family's length instead of the alignment's.
    ///
    /// States are named `S`, `I0`, then `M1 D1 I1 … Mn Dn In`, then `E`. `S`,
    /// `E` and the deletion states are silent; their emission rows stay zero.
    ///
    /// `pseudocount` of zero gives BA10E and leaves unvisited rows at zero.
    /// Anything larger gives BA10F: the counts are turned into probabilities,
    /// the pseudocount is added to each *allowed* entry, and the row is
    /// renormalised. Adding it to the raw counts instead would make its effect
    /// depend on how many sequences the alignment happens to contain.
    pub fn profile(
        alignment: &[String],
        symbols: Vec<String>,
        threshold: f64,
        pseudocount: f64,
    ) -> Self {
        let rows = alignment.len();
        let width = alignment
            .iter()
            .map(|row| row.chars().count())
            .max()
            .unwrap_or(0);

        // A column is a match column when few enough of its entries are gaps.
        let columns: Vec<Vec<Option<char>>> = (0..width)
            .map(|column| {
                alignment
                    .iter()
                    .map(|row| match row.chars().nth(column) {
                        Some('-') | None => None,
                        Some(symbol) => Some(symbol),
                    })
                    .collect()
            })
            .collect();
        let is_match: Vec<bool> = columns
            .iter()
            .map(|column| {
                let gaps = column.iter().filter(|entry| entry.is_none()).count();
                rows > 0 && (gaps as f64 / rows as f64) < threshold
            })
            .collect();
        let layers = is_match.iter().filter(|&&keep| keep).count();

        // S, I0, then three states per layer, then E.
        let count = 3 * layers + 3;
        let start = 0usize;
        let end = count - 1;
        let insert = |layer: usize| if layer == 0 { 1 } else { 3 * layer + 1 };
        let match_state = |layer: usize| 3 * layer - 1;
        let delete = |layer: usize| 3 * layer;

        let mut states = Vec::with_capacity(count);
        states.push("S".to_string());
        states.push("I0".to_string());
        for layer in 1..=layers {
            states.push(format!("M{layer}"));
            states.push(format!("D{layer}"));
            states.push(format!("I{layer}"));
        }
        states.push("E".to_string());

        let index_of = |symbol: char| symbols.iter().position(|s| s == &symbol.to_string());
        let mut transition = vec![vec![0.0f64; count]; count];
        let mut emission = vec![vec![0.0f64; symbols.len()]; count];

        // Walk each sequence through the model, counting what it does.
        for row in 0..rows {
            let mut previous = start;
            let mut layer = 0usize;
            for column in 0..width {
                let entry = columns[column][row];
                let here = if is_match[column] {
                    layer += 1;
                    match entry {
                        // A gap in a match column is a deletion, and silent.
                        None => delete(layer),
                        Some(_) => match_state(layer),
                    }
                } else {
                    match entry {
                        // A gap in an insertion column is not an event at all.
                        None => continue,
                        Some(_) => insert(layer),
                    }
                };
                if let Some(symbol) = entry {
                    if let Some(which) = index_of(symbol) {
                        emission[here][which] += 1.0;
                    }
                }
                transition[previous][here] += 1.0;
                previous = here;
            }
            transition[previous][end] += 1.0;
        }

        // Unvisited rows stay zero here, rather than becoming uniform as they do
        // in parameter estimation: a state the alignment never reaches has no
        // transitions, and inventing a uniform row would put probability on
        // moves the family never makes.
        normalise_rows_or_zero(&mut transition);
        normalise_rows_or_zero(&mut emission);

        let mut model = Self {
            states,
            symbols,
            initial: {
                let mut initial = vec![0.0; count];
                initial[start] = 1.0;
                initial
            },
            transition,
            emission,
        };
        model.apply_profile_pseudocounts(pseudocount, layers);
        model
    }

    /// The most likely path through a profile HMM that emits `observations`.
    ///
    /// [`Hmm::viterbi`] cannot do this. It advances one state per symbol, and a
    /// profile's deletion states are *silent* — they are passed through without
    /// emitting anything, so a path of nine states can emit seven symbols. That
    /// changes the recurrence: at each position the silent states have to be
    /// settled in layer order before the emitting ones can look back at them.
    ///
    /// Returns `None` for a model that is not a profile, since the layer
    /// arithmetic below would otherwise read meaning into unrelated states.
    pub fn align_to_profile(&self, observations: &[usize]) -> Option<Vec<usize>> {
        let count = self.states.len();
        if count < 3 || (count - 3) % 3 != 0 {
            return None;
        }
        if self.states.first().map(String::as_str) != Some("S")
            || self.states.last().map(String::as_str) != Some("E")
        {
            return None;
        }
        let layers = (count - 3) / 3;
        let length = observations.len();
        let start = 0usize;
        let end = count - 1;
        let insert = |layer: usize| if layer == 0 { 1 } else { 3 * layer + 1 };
        let match_state = |layer: usize| 3 * layer - 1;
        let delete = |layer: usize| 3 * layer;

        // The states that can move on from a layer: `S` and `I0` at the top,
        // otherwise the layer's match, delete and insert.
        let sources = |layer: usize| -> Vec<usize> {
            if layer == 0 {
                vec![start, insert(0)]
            } else {
                vec![match_state(layer), delete(layer), insert(layer)]
            }
        };

        let mut score = vec![vec![f64::NEG_INFINITY; count]; length + 1];
        // `came_from[i][state]` is the (position, state) this was reached from.
        let mut came_from = vec![vec![(0usize, 0usize); count]; length + 1];
        score[0][start] = 0.0;

        let mut relax = |score: &mut Vec<Vec<f64>>,
                         came_from: &mut Vec<Vec<(usize, usize)>>,
                         at: usize,
                         to: usize,
                         previous_at: usize,
                         previous: usize,
                         weight: f64| {
            let candidate = score[previous_at][previous] + weight;
            if candidate > score[at][to] {
                score[at][to] = candidate;
                came_from[at][to] = (previous_at, previous);
            }
        };

        for position in 0..=length {
            for layer in 0..=layers {
                // Emitting states first: they consume the symbol just passed.
                if position > 0 {
                    let symbol = observations[position - 1];
                    if layer > 0 {
                        let to = match_state(layer);
                        let weight = ln(self.emission[to][symbol]);
                        for previous in sources(layer - 1) {
                            relax(
                                &mut score,
                                &mut came_from,
                                position,
                                to,
                                position - 1,
                                previous,
                                ln(self.transition[previous][to]) + weight,
                            );
                        }
                    }
                    let to = insert(layer);
                    let weight = ln(self.emission[to][symbol]);
                    for previous in sources(layer) {
                        relax(
                            &mut score,
                            &mut came_from,
                            position,
                            to,
                            position - 1,
                            previous,
                            ln(self.transition[previous][to]) + weight,
                        );
                    }
                }

                // Then the deletion, which emits nothing and so stays at this
                // position — reachable only from the layer above, already final.
                if layer > 0 {
                    let to = delete(layer);
                    for previous in sources(layer - 1) {
                        relax(
                            &mut score,
                            &mut came_from,
                            position,
                            to,
                            position,
                            previous,
                            ln(self.transition[previous][to]),
                        );
                    }
                }
            }
        }

        for previous in sources(layers) {
            relax(
                &mut score,
                &mut came_from,
                length,
                end,
                length,
                previous,
                ln(self.transition[previous][end]),
            );
        }
        if !score[length][end].is_finite() {
            return None;
        }

        let mut path = Vec::new();
        let mut here = (length, end);
        while here != (0, start) {
            let previous = came_from[here.0][here.1];
            path.push(here.1);
            here = previous;
        }
        path.reverse();
        // `S` and `E` bracket the path rather than belonging to it.
        path.pop();
        Some(path)
    }

    /// Add `pseudocount` to every transition and emission the topology permits,
    /// then renormalise.
    ///
    /// Only permitted entries: a profile HMM cannot jump backwards or skip a
    /// layer, and smoothing those to non-zero would invent paths the model does
    /// not have. Silent states get no emission pseudocount for the same reason.
    fn apply_profile_pseudocounts(&mut self, pseudocount: f64, layers: usize) {
        if pseudocount <= 0.0 {
            return;
        }
        let count = self.states.len();
        let end = count - 1;
        let insert = |layer: usize| if layer == 0 { 1 } else { 3 * layer + 1 };
        let match_state = |layer: usize| 3 * layer - 1;
        let delete = |layer: usize| 3 * layer;

        for layer in 0..=layers {
            // The states that sit in this layer and can move on from it.
            let sources: Vec<usize> = if layer == 0 {
                vec![0, insert(0)]
            } else {
                vec![match_state(layer), delete(layer), insert(layer)]
            };
            let targets: Vec<usize> = if layer == layers {
                vec![insert(layers), end]
            } else {
                vec![insert(layer), match_state(layer + 1), delete(layer + 1)]
            };
            for &from in &sources {
                for &to in &targets {
                    self.transition[from][to] += pseudocount;
                }
                let row = &mut self.transition[from];
                let total: f64 = row.iter().sum();
                if total > 0.0 {
                    for value in row.iter_mut() {
                        *value /= total;
                    }
                }
            }
        }

        // Only the emitting states — S, E and the deletions stay silent.
        let emitting: Vec<usize> = (0..=layers)
            .flat_map(|layer| {
                if layer == 0 {
                    vec![insert(0)]
                } else {
                    vec![match_state(layer), insert(layer)]
                }
            })
            .collect();
        for state in emitting {
            for value in self.emission[state].iter_mut() {
                *value += pseudocount;
            }
            let total: f64 = self.emission[state].iter().sum();
            if total > 0.0 {
                for value in self.emission[state].iter_mut() {
                    *value /= total;
                }
            }
        }
    }

    /// The model that best explains an observation *and* the path that produced
    /// it, by counting.
    ///
    /// With the path given there is nothing to infer: the maximum-likelihood
    /// estimate of each probability is the fraction of the time it was taken.
    /// This is the step both learning algorithms below repeat.
    ///
    /// A state the path never visits leaves its row with nothing to divide by.
    /// Those rows come back uniform, which is the convention Rosalind uses and
    /// the only one that keeps every row a distribution.
    pub fn estimate(
        states: Vec<String>,
        symbols: Vec<String>,
        observations: &[usize],
        path: &[usize],
    ) -> Self {
        let count = states.len();
        let alphabet = symbols.len();
        let mut transition = vec![vec![0.0f64; count]; count];
        let mut emission = vec![vec![0.0f64; alphabet]; count];

        for pair in path.windows(2) {
            transition[pair[0]][pair[1]] += 1.0;
        }
        for (&state, &symbol) in path.iter().zip(observations) {
            emission[state][symbol] += 1.0;
        }

        normalise_rows(&mut transition);
        normalise_rows(&mut emission);

        Self::with_uniform_start(states, symbols, transition, emission)
    }

    /// Learn a model from an observation alone, by alternating decoding and
    /// counting.
    ///
    /// Decode the most likely path under the current model, then re-estimate the
    /// model as if that path were the truth, and repeat. Each round can only
    /// increase `Pr(x, pi)`, so it converges — but to a local optimum that
    /// depends on where it started, which is why the initial matrices are part
    /// of the problem rather than an implementation detail.
    ///
    /// The harder assumption is the one inside: committing to a single path
    /// throws away every other path's contribution. [`Hmm::baum_welch`] keeps
    /// them, weighted, and is the better estimator for it.
    pub fn viterbi_learning(&self, observations: &[usize], iterations: usize) -> Self {
        let mut model = self.clone();
        for _ in 0..iterations {
            let path = model.viterbi(observations);
            let counted = Self::estimate(
                model.states.clone(),
                model.symbols.clone(),
                observations,
                &path,
            );
            model = Self {
                initial: model.initial,
                ..counted
            };
        }
        model
    }

    /// Learn a model from an observation alone, weighting every path by how
    /// likely it is.
    ///
    /// Baum-Welch: expectation-maximisation for an HMM. Where Viterbi learning
    /// counts transitions along one path, this counts the *expected* number of
    /// times each transition was taken, over all paths at once — which
    /// forward-backward gives without enumerating them.
    ///
    /// Each round can only increase `Pr(x)`, and again the fixed point it
    /// reaches depends on where it started.
    pub fn baum_welch(&self, observations: &[usize], iterations: usize) -> Self {
        let count = self.states.len();
        let alphabet = self.symbols.len();
        let length = observations.len();
        let mut model = self.clone();
        if count == 0 || length < 2 {
            return model;
        }

        for _ in 0..iterations {
            let (alpha, beta, scale) = model.scaled_passes(observations);

            // How much of each position's probability mass sat in each state.
            let gamma: Vec<Vec<f64>> = (0..length)
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
                .collect();

            // And how much crossed each edge between one position and the next.
            let mut transition = vec![vec![0.0f64; count]; count];
            for step in 0..length - 1 {
                let mut edge = vec![vec![0.0f64; count]; count];
                for from in 0..count {
                    for to in 0..count {
                        edge[from][to] = alpha[step][from]
                            * model.transition[from][to]
                            * model.emission[to][observations[step + 1]]
                            * beta[step + 1][to]
                            / scale[step + 1];
                    }
                }
                let total: f64 = edge.iter().flatten().sum();
                if total > 0.0 {
                    for from in 0..count {
                        for to in 0..count {
                            transition[from][to] += edge[from][to] / total;
                        }
                    }
                }
            }

            let mut emission = vec![vec![0.0f64; alphabet]; count];
            for (step, &symbol) in observations.iter().enumerate() {
                for state in 0..count {
                    emission[state][symbol] += gamma[step][state];
                }
            }

            normalise_rows(&mut transition);
            normalise_rows(&mut emission);
            model.transition = transition;
            model.emission = emission;
        }

        model
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

    /// Rounded the way Rosalind prints its matrices, so a published table can be
    /// compared entry by entry.
    fn rounded(rows: &[Vec<f64>], places: i32) -> Vec<Vec<f64>> {
        let factor = 10f64.powi(places);
        rows.iter()
            .map(|row| row.iter().map(|v| (v * factor).round() / factor).collect())
            .collect()
    }

    fn letters(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| (*s).to_string()).collect()
    }

    /// Look a value up by state and column name, so a test reads like the
    /// published table instead of like an index calculation.
    fn at(model: &Hmm, rows: &[Vec<f64>], state: &str, column: &[String], name: &str) -> f64 {
        let row = model.states.iter().position(|s| s == state).expect("state");
        let index = column.iter().position(|s| s == name).expect("column");
        (rows[row][index] * 1000.0).round() / 1000.0
    }

    #[test]
    fn ba10e_sample() {
        let alignment = letters(&["EBA", "EBD", "EB-", "EED", "EBD", "EBE", "E-D", "EBD"]);
        let symbols = letters(&["A", "B", "C", "D", "E"]);
        let model = Hmm::profile(&alignment, symbols.clone(), 0.289, 0.0);

        // No column is gappy enough to be an insertion, so the model is three
        // layers long.
        assert_eq!(
            model.states,
            letters(&["S", "I0", "M1", "D1", "I1", "M2", "D2", "I2", "M3", "D3", "I3", "E"])
        );

        let states = model.states.clone();
        let transition = |from: &str, to: &str| at(&model, &model.transition, from, &states, to);
        assert_eq!(transition("S", "M1"), 1.0);
        assert_eq!(transition("M1", "M2"), 0.875);
        assert_eq!(transition("M1", "D2"), 0.125);
        assert_eq!(transition("M2", "M3"), 0.857);
        assert_eq!(transition("M2", "D3"), 0.143);
        assert_eq!(transition("D2", "M3"), 1.0);
        assert_eq!(transition("M3", "E"), 1.0);
        assert_eq!(transition("D3", "E"), 1.0);

        let emission =
            |state: &str, symbol: &str| at(&model, &model.emission, state, &symbols, symbol);
        assert_eq!(emission("M1", "E"), 1.0);
        assert_eq!(emission("M2", "B"), 0.857);
        assert_eq!(emission("M2", "E"), 0.143);
        assert_eq!(emission("M3", "A"), 0.143);
        assert_eq!(emission("M3", "D"), 0.714);
        assert_eq!(emission("M3", "E"), 0.143);

        // Without pseudocounts, a state the alignment never reaches keeps an
        // empty row rather than a uniform one.
        for state in ["I0", "D1", "I1", "I2", "I3", "E"] {
            let row = model.states.iter().position(|s| s == state).expect("state");
            assert!(
                model.transition[row].iter().all(|&v| v == 0.0),
                "{state} should have no transitions"
            );
        }
        // Silent states never emit.
        for state in ["S", "D1", "D2", "D3", "E"] {
            let row = model.states.iter().position(|s| s == state).expect("state");
            assert!(
                model.emission[row].iter().all(|&v| v == 0.0),
                "{state} is silent"
            );
        }
    }

    #[test]
    fn ba10f_sample() {
        let alignment = letters(&["ADA", "ADA", "AAA", "ADC", "-DA", "D-A"]);
        let symbols = letters(&["A", "B", "C", "D", "E"]);
        let model = Hmm::profile(&alignment, symbols.clone(), 0.358, 0.01);

        let states = model.states.clone();
        let transition = |from: &str, to: &str| at(&model, &model.transition, from, &states, to);
        assert_eq!(transition("S", "I0"), 0.01);
        assert_eq!(transition("S", "M1"), 0.819);
        assert_eq!(transition("S", "D1"), 0.172);
        // A row with no counts smooths to uniform over what the topology allows,
        // not over every state.
        assert_eq!(transition("I0", "I0"), 0.333);
        assert_eq!(transition("I0", "M1"), 0.333);
        assert_eq!(transition("I0", "D1"), 0.333);
        assert_eq!(transition("M1", "I1"), 0.01);
        assert_eq!(transition("M1", "M2"), 0.786);
        assert_eq!(transition("D1", "M2"), 0.981);

        let emission =
            |state: &str, symbol: &str| at(&model, &model.emission, state, &symbols, symbol);
        assert_eq!(emission("I0", "A"), 0.2);
        assert_eq!(emission("M1", "A"), 0.771);
        assert_eq!(emission("M1", "B"), 0.01);
        assert_eq!(emission("M1", "D"), 0.2);
        assert_eq!(emission("M2", "A"), 0.2);
        assert_eq!(emission("M2", "D"), 0.771);
        // Pseudocounts do not make a silent state emit.
        let d1 = model.states.iter().position(|s| s == "D1").expect("D1");
        assert!(model.emission[d1].iter().all(|&v| v == 0.0), "D1 is silent");
    }

    #[test]
    fn a_gappy_column_becomes_an_insertion() {
        // Above the threshold the column stops being part of the family's
        // length, which is the whole point of the threshold.
        let alignment = letters(&["A-A", "A-A", "ABA", "A-A"]);
        let symbols = letters(&["A", "B"]);

        let strict = Hmm::profile(&alignment, symbols.clone(), 0.9, 0.0);
        assert_eq!(strict.states.len(), 3 * 3 + 3, "all three columns kept");

        let loose = Hmm::profile(&alignment, symbols, 0.5, 0.0);
        assert_eq!(
            loose.states.len(),
            3 * 2 + 3,
            "the gappy column is an insertion"
        );
        assert!(loose.states.contains(&"I1".to_string()));
    }

    #[test]
    fn every_profile_row_is_a_distribution_or_empty() {
        let alignment = letters(&["ACD-", "A-DE", "ACDE", "-CDE"]);
        let symbols = letters(&["A", "C", "D", "E"]);
        for pseudocount in [0.0, 0.01, 0.1] {
            let model = Hmm::profile(&alignment, symbols.clone(), 0.4, pseudocount);
            for row in model.transition.iter().chain(model.emission.iter()) {
                let total: f64 = row.iter().sum();
                assert!(
                    total == 0.0 || (total - 1.0).abs() < 1e-9,
                    "a row sums to {total} at pseudocount {pseudocount}"
                );
            }
        }
    }

    #[test]
    fn ba10g_sample() {
        let alignment = letters(&[
            "ACDEFACADF",
            "AFDA---CCF",
            "A--EFD-FDC",
            "ACAEF--A-C",
            "ADDEFAAADF",
        ]);
        let symbols = letters(&["A", "B", "C", "D", "E", "F"]);
        let model = Hmm::profile(&alignment, symbols.clone(), 0.4, 0.01);
        let observations = encode("AEFDFDC", &symbols);

        let path: Vec<String> = model
            .align_to_profile(&observations)
            .expect("a profile and a path")
            .iter()
            .map(|&state| model.states[state].clone())
            .collect();

        assert_eq!(path.join(" "), "M1 D2 D3 M4 M5 I5 M6 M7 M8");
        // Nine states emitting seven symbols: the two deletions are silent,
        // which is the whole reason plain Viterbi cannot solve this.
        let emitting = path
            .iter()
            .filter(|state| state.starts_with('M') || state.starts_with('I'))
            .count();
        assert_eq!(emitting, observations.len());
    }

    #[test]
    fn align_to_profile_refuses_a_model_that_is_not_a_profile() {
        // The layer arithmetic would otherwise read meaning into ordinary states.
        assert!(ba10c_model().align_to_profile(&[0, 1]).is_none());
    }

    #[test]
    fn ba10h_sample() {
        let states: Vec<String> = ["A", "B", "C"].iter().map(|s| (*s).into()).collect();
        let symbols: Vec<String> = ["x", "y", "z"].iter().map(|s| (*s).into()).collect();
        let observations = encode("yzzzyxzxxx", &symbols);
        let path = encode("BBABABABAB", &states);

        let model = Hmm::estimate(states, symbols, &observations, &path);

        assert_eq!(
            rounded(&model.transition, 3),
            vec![
                vec![0.0, 1.0, 0.0],
                vec![0.8, 0.2, 0.0],
                // C is never visited, so its row has nothing to count.
                vec![0.333, 0.333, 0.333],
            ]
        );
        assert_eq!(
            rounded(&model.emission, 3),
            vec![
                vec![0.25, 0.25, 0.5],
                vec![0.5, 0.167, 0.333],
                vec![0.333, 0.333, 0.333],
            ]
        );
    }

    #[test]
    fn ba10i_sample() {
        let model = Hmm::with_uniform_start(
            vec!["A".into(), "B".into()],
            vec!["x".into(), "y".into(), "z".into()],
            vec![vec![0.582, 0.418], vec![0.272, 0.728]],
            vec![vec![0.129, 0.35, 0.52], vec![0.422, 0.151, 0.426]],
        );
        let observations = encode(
            "xxxzyzzxxzxyzxzxyxxzyzyzyyyyzzxxxzzxzyzzzxyxzzzxyzzxxxxzzzxyyxzzzzzyzzzxxzzxxxyxyzzyxzxxxyxzyxxyzyxz",
            &model.symbols,
        );

        let learned = model.viterbi_learning(&observations, 100);

        assert_eq!(
            rounded(&learned.transition, 3),
            vec![vec![0.875, 0.125], vec![0.011, 0.989]]
        );
        assert_eq!(
            rounded(&learned.emission, 3),
            vec![vec![0.0, 0.75, 0.25], vec![0.402, 0.174, 0.424]]
        );
    }

    #[test]
    fn ba10k_sample() {
        let model = Hmm::with_uniform_start(
            vec!["A".into(), "B".into()],
            vec!["x".into(), "y".into(), "z".into()],
            vec![vec![0.019, 0.981], vec![0.668, 0.332]],
            vec![vec![0.175, 0.003, 0.821], vec![0.196, 0.512, 0.293]],
        );
        let observations = encode("xzyyzyzyxy", &model.symbols);

        let learned = model.baum_welch(&observations, 10);

        assert_eq!(
            rounded(&learned.transition, 3),
            vec![vec![0.0, 1.0], vec![0.786, 0.214]]
        );
        assert_eq!(
            rounded(&learned.emission, 3),
            vec![vec![0.242, 0.0, 0.758], vec![0.172, 0.828, 0.0]]
        );
    }

    #[test]
    fn learning_never_makes_the_observation_less_likely() {
        // The property both algorithms are built on. If a round ever lowers the
        // likelihood, the update is wrong however plausible the numbers look.
        let model = ba10c_model();
        let observations = encode("xyxzzxyxyyzzxyxzy", &model.symbols);
        let mut previous = model.log_likelihood(&observations);
        let mut current = model.clone();
        for round in 1..=8 {
            current = current.baum_welch(&observations, 1);
            let now = current.log_likelihood(&observations);
            assert!(
                now >= previous - 1e-9,
                "round {round} lowered the likelihood: {previous} -> {now}"
            );
            previous = now;
        }
    }

    #[test]
    fn every_learned_row_is_still_a_distribution() {
        let model = ba10c_model();
        let observations = encode("xyzzyxyxzzyx", &model.symbols);
        for learned in [
            model.viterbi_learning(&observations, 20),
            model.baum_welch(&observations, 20),
        ] {
            for row in learned.transition.iter().chain(learned.emission.iter()) {
                let total: f64 = row.iter().sum();
                assert!((total - 1.0).abs() < 1e-9, "a row sums to {total}");
                assert!(row.iter().all(|v| v.is_finite() && *v >= 0.0), "{row:?}");
            }
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
