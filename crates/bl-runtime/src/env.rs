use bl_core::error::{BioLangError, Result};
use bl_core::span::Span;
use bl_core::value::Value;
// Variable lookup is the interpreter's hottest path — every read and every
// write of a name hashes it. The standard hasher is SipHash, which is built to
// resist collision attacks on untrusted keys; these keys are identifiers out of
// the program's own source, so that protection buys nothing and the extra work
// is paid on every access. FxHash is the one rustc itself uses for exactly this
// reason.
use rustc_hash::FxHashMap as HashMap;

/// A scope in the environment chain.
#[derive(Debug, Clone)]
struct Scope {
    vars: HashMap<String, Value>,
    parent: Option<usize>,
    /// Value of `Environment::handed_out` when this scope was created. Compared
    /// on the way out to decide whether the scope can be reclaimed — see
    /// `pop_scope`.
    handed_out_at_birth: u64,
}

/// Environment with a scope chain for lexical scoping.
#[derive(Debug, Clone)]
pub struct Environment {
    scopes: Vec<Scope>,
    current: usize,
    /// How many times a scope id has been handed out to be stored elsewhere.
    /// A closure keeps the id of the scope it was defined in, so once an id has
    /// escaped, the scope it names has to stay alive.
    handed_out: u64,
}

impl Environment {
    pub fn new() -> Self {
        let global = Scope {
            vars: HashMap::default(),
            parent: None,
            handed_out_at_birth: 0,
        };
        Self {
            scopes: vec![global],
            current: 0,
            handed_out: 0,
        }
    }

    /// The current scope id, for a closure to capture.
    ///
    /// Every caller stores the id somewhere that outlives the scope, so this
    /// counts as the scope escaping and pins everything created since — see
    /// `pop_scope`. Callers that only want to read the id and drop it would
    /// pin scopes unnecessarily, which costs memory but is never wrong.
    pub fn current_scope_id(&mut self) -> usize {
        self.handed_out += 1;
        self.current
    }

    /// Push a new child scope, returning the previous scope id.
    pub fn push_scope(&mut self) -> usize {
        let prev = self.current;
        let new_scope = Scope {
            vars: HashMap::default(),
            parent: Some(self.current),
            handed_out_at_birth: self.handed_out,
        };
        self.current = self.scopes.len();
        self.scopes.push(new_scope);
        prev
    }

    /// Push a child scope under a specific parent (for closures).
    pub fn push_scope_under(&mut self, parent: usize) -> usize {
        let prev = self.current;
        let new_scope = Scope {
            vars: HashMap::default(),
            parent: Some(parent),
            handed_out_at_birth: self.handed_out,
        };
        self.current = self.scopes.len();
        self.scopes.push(new_scope);
        prev
    }

    /// Pop back to a previous scope, reclaiming the one being left when it can
    /// be shown that nothing else refers to it.
    ///
    /// Scopes live in a `Vec` and are named by index, so a scope cannot simply
    /// be dropped: a closure may hold the index of the scope it was defined in.
    /// The test is therefore whether any id was handed out during this scope's
    /// lifetime. If none was, no closure can name this scope or any scope
    /// created inside it, and every one of them is unreachable once we leave.
    ///
    /// Indices only grow, so those scopes are exactly the tail of the `Vec`.
    /// The length check is belt and braces: if push and pop were ever unbalanced
    /// the tail would not be what we think, and the right answer is to reclaim
    /// nothing and keep the old behaviour rather than to free a live scope.
    ///
    /// Without this, a loop kept every scope it ever entered — around 900 bytes
    /// an iteration, so a loop of a million left most of a gigabyte behind.
    pub fn pop_scope(&mut self, prev: usize) {
        let leaving = self.current;
        self.current = prev;

        let nothing_escaped = self
            .scopes
            .get(leaving)
            .is_some_and(|scope| scope.handed_out_at_birth == self.handed_out);
        let is_tail = leaving + 1 == self.scopes.len();

        if nothing_escaped && is_tail && leaving > prev {
            self.scopes.truncate(leaving);
        }
    }

    /// How many scopes are currently held.
    ///
    /// Exposed so the reclamation in `pop_scope` can be asserted directly. A
    /// loop that creates no closures should leave this flat however many times
    /// it goes round.
    pub fn scope_count(&self) -> usize {
        self.scopes.len()
    }

    /// Define a variable in the current scope.
    pub fn define(&mut self, name: String, value: Value) {
        self.scopes[self.current].vars.insert(name, value);
    }

    /// Look up a variable, walking the scope chain. Returns None if it is not
    /// bound anywhere.
    ///
    /// Use this rather than `get` wherever a missing name is an ordinary answer
    /// instead of an error. `get` pays for the "did you mean?" search on the way
    /// out, which is worth it for a real mistake and ruinous in a loop: the
    /// interpreter probes for marker bindings like `__const_x` on every
    /// assignment, and those probes are expected to miss.
    pub fn lookup(&self, name: &str) -> Option<&Value> {
        let mut scope_id = self.current;
        loop {
            if let Some(val) = self.scopes[scope_id].vars.get(name) {
                return Some(val);
            }
            match self.scopes[scope_id].parent {
                Some(parent) => scope_id = parent,
                None => return None,
            }
        }
    }

    /// Whether a name is bound in the current scope chain.
    pub fn has(&self, name: &str) -> bool {
        self.lookup(name).is_some()
    }

    /// Borrow a bound variable mutably, walking the scope chain.
    ///
    /// Updating a container through this leaves the binding as its only owner,
    /// so `Arc::make_mut` writes in place. Reading the value out, editing the
    /// copy and storing it back instead gives the Arc a second owner and copies
    /// the whole container on every element write, which turns a loop that
    /// assigns into an array from quadratic into cubic.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        let mut scope_id = self.current;
        loop {
            if self.scopes[scope_id].vars.contains_key(name) {
                return self.scopes[scope_id].vars.get_mut(name);
            }
            match self.scopes[scope_id].parent {
                Some(parent) => scope_id = parent,
                None => return None,
            }
        }
    }

    /// Look up a variable, walking the scope chain.
    pub fn get(&self, name: &str, span: Option<Span>) -> Result<&Value> {
        // Borrow-checker note: the lookup is repeated rather than reusing
        // `lookup`, because returning the borrow from it would hold `&self` for
        // the whole function and `find_similar` needs it again on the error path.
        if self.lookup(name).is_some() {
            return Ok(self.lookup(name).expect("just checked"));
        }
        let mut err = BioLangError::name_error(format!("undefined variable '{name}'"), span);
        // "Did you mean?" — find closest variable name. This walks every name in
        // scope and is only reached when the lookup has already failed.
        if let Some(suggestion) = self.find_similar(name) {
            err = err.with_suggestion(format!("did you mean '{suggestion}'?"));
        }
        Err(err)
    }

    /// Find the most similar variable name using Levenshtein distance.
    fn find_similar(&self, name: &str) -> Option<String> {
        let mut best: Option<(String, usize)> = None;
        let max_dist = (name.len() / 3).max(2); // Allow ~33% edit distance
        let mut scope_id = self.current;
        loop {
            for key in self.scopes[scope_id].vars.keys() {
                let dist = levenshtein(name, key);
                if dist > 0 && dist <= max_dist && best.as_ref().is_none_or(|(_, d)| dist < *d) {
                    best = Some((key.clone(), dist));
                }
            }
            match self.scopes[scope_id].parent {
                Some(parent) => scope_id = parent,
                None => break,
            }
        }
        best.map(|(s, _)| s)
    }

    /// Set a variable in the nearest scope that contains it.
    pub fn set(&mut self, name: &str, value: Value, span: Option<Span>) -> Result<()> {
        let mut scope_id = self.current;
        loop {
            // One lookup, and no new key. This used to test with contains_key
            // and then insert, which hashed the name twice and allocated a fresh
            // String for a key already sitting in the map.
            if let Some(slot) = self.scopes[scope_id].vars.get_mut(name) {
                *slot = value;
                return Ok(());
            }
            match self.scopes[scope_id].parent {
                Some(parent) => scope_id = parent,
                None => {
                    return Err(BioLangError::name_error(
                        format!("undefined variable '{name}'"),
                        span,
                    ))
                }
            }
        }
    }

    /// Return all variables in the current scope only (not walking parents).
    pub fn list_current_scope_vars(&self) -> Vec<(String, Value)> {
        self.scopes[self.current]
            .vars
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Return all variables in the global scope (scope 0).
    pub fn list_global_vars(&self) -> Vec<(&str, &Value)> {
        self.scopes[0]
            .vars
            .iter()
            .map(|(k, v)| (k.as_str(), v))
            .collect()
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

/// Levenshtein edit distance between two strings.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut prev = (0..=n).collect::<Vec<_>>();
    let mut curr = vec![0; n + 1];
    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}
