use bl_core::error::{BioLangError, Result};
use bl_core::span::Span;
use bl_core::value::Value;
use std::collections::HashMap;

/// A scope in the environment chain.
#[derive(Debug, Clone)]
struct Scope {
    vars: HashMap<String, Value>,
    parent: Option<usize>,
}

/// Environment with a scope chain for lexical scoping.
#[derive(Debug, Clone)]
pub struct Environment {
    scopes: Vec<Scope>,
    current: usize,
}

impl Environment {
    pub fn new() -> Self {
        let global = Scope {
            vars: HashMap::new(),
            parent: None,
        };
        Self {
            scopes: vec![global],
            current: 0,
        }
    }

    pub fn current_scope_id(&self) -> usize {
        self.current
    }

    /// Push a new child scope, returning the previous scope id.
    pub fn push_scope(&mut self) -> usize {
        let prev = self.current;
        let new_scope = Scope {
            vars: HashMap::new(),
            parent: Some(self.current),
        };
        self.current = self.scopes.len();
        self.scopes.push(new_scope);
        prev
    }

    /// Push a child scope under a specific parent (for closures).
    pub fn push_scope_under(&mut self, parent: usize) -> usize {
        let prev = self.current;
        let new_scope = Scope {
            vars: HashMap::new(),
            parent: Some(parent),
        };
        self.current = self.scopes.len();
        self.scopes.push(new_scope);
        prev
    }

    /// Pop back to a previous scope.
    pub fn pop_scope(&mut self, prev: usize) {
        self.current = prev;
    }

    /// Define a variable in the current scope.
    pub fn define(&mut self, name: String, value: Value) {
        self.scopes[self.current].vars.insert(name, value);
    }

    /// Look up a variable, walking the scope chain.
    pub fn get(&self, name: &str, span: Option<Span>) -> Result<&Value> {
        let mut scope_id = self.current;
        loop {
            if let Some(val) = self.scopes[scope_id].vars.get(name) {
                return Ok(val);
            }
            match self.scopes[scope_id].parent {
                Some(parent) => scope_id = parent,
                None => {
                    let mut err = BioLangError::name_error(
                        format!("undefined variable '{name}'"),
                        span,
                    );
                    // "Did you mean?" — find closest variable name
                    if let Some(suggestion) = self.find_similar(name) {
                        err = err.with_suggestion(format!("did you mean '{suggestion}'?"));
                    }
                    return Err(err);
                }
            }
        }
    }

    /// Find the most similar variable name using Levenshtein distance.
    fn find_similar(&self, name: &str) -> Option<String> {
        let mut best: Option<(String, usize)> = None;
        let max_dist = (name.len() / 3).max(2); // Allow ~33% edit distance
        let mut scope_id = self.current;
        loop {
            for key in self.scopes[scope_id].vars.keys() {
                let dist = levenshtein(name, key);
                if dist > 0 && dist <= max_dist {
                    if best.as_ref().map_or(true, |(_, d)| dist < *d) {
                        best = Some((key.clone(), dist));
                    }
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
            if self.scopes[scope_id].vars.contains_key(name) {
                self.scopes[scope_id].vars.insert(name.to_string(), value);
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
