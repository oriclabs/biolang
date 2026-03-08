use crate::ast::{Param, Stmt};
use crate::sparse_matrix::SparseMatrix;
use crate::span::Spanned;
use crate::types::Type;
pub use bio_core::{BioSequence, GenomicInterval, Kmer, Strand};
use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Runtime values in BioLang.
#[derive(Debug, Clone)]
pub enum Value {
    Nil,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<Value>),
    Map(HashMap<String, Value>),
    Record(HashMap<String, Value>),

    /// User-defined function / closure
    Function {
        name: Option<String>,
        params: Vec<Param>,
        body: Vec<Spanned<Stmt>>,
        /// Captured environment (lexical scope at definition time)
        closure_env: Option<usize>,
        /// Documentation string from `## ...` comments
        doc: Option<String>,
        /// Whether this is a generator function (fn*)
        is_generator: bool,
    },

    /// Built-in native function
    NativeFunction {
        name: String,
        arity: Arity,
    },

    /// Formula expression (~expr) stored as unevaluated AST
    Formula(Box<Spanned<crate::ast::Expr>>),

    /// DNA sequence — compact 2-bit encoded storage
    DNA(BioSequence),
    /// RNA sequence
    RNA(BioSequence),
    /// Protein sequence (amino acid single-letter codes)
    Protein(BioSequence),

    /// Lazy stream — yields Values one at a time without loading all into memory.
    Stream(StreamValue),

    /// Tabular data with named columns (R tibble-like).
    Table(Table),

    /// A single genomic interval (chrom:start-end).
    Interval(GenomicInterval),

    /// Dense numeric matrix.
    Matrix(crate::matrix::Matrix),

    /// Range value: `1..10` or `1..=10`
    Range {
        start: i64,
        end: i64,
        inclusive: bool,
    },

    /// Enum variant value
    EnumValue {
        enum_name: String,
        variant: String,
        fields: Vec<Value>,
    },

    /// A callable function backed by an external plugin subprocess.
    PluginFunction {
        plugin_name: String,
        operation: String,
        plugin_dir: PathBuf,
        kind: String,
        entrypoint: String,
    },

    /// Set type: ordered Vec with dedup
    Set(Vec<Value>),

    /// Tuple: fixed-size, position-indexed, immutable collection
    Tuple(Vec<Value>),

    /// Regex value: pattern + flags
    Regex {
        pattern: String,
        flags: String,
    },

    /// Future (lazy thunk) — result of calling an async fn
    Future(Arc<Mutex<FutureState>>),

    /// 2-bit encoded k-mer for efficient sequence analysis
    Kmer(Kmer),

    /// Compressed sparse row matrix (for single-cell RNA-seq, etc.)
    SparseMatrix(SparseMatrix),

    /// Compiled bytecode closure (opaque, used by bl-jit VM).
    CompiledClosure(Arc<dyn Any + Send + Sync>),

    /// Gene annotation
    Gene {
        symbol: String,
        gene_id: String,
        chrom: String,
        start: i64,
        end: i64,
        strand: String,
        biotype: String,
        description: String,
    },

    /// Genetic variant (VCF-like)
    Variant {
        chrom: String,
        pos: i64,
        id: String,
        ref_allele: String,
        alt_allele: String,
        quality: f64,
        filter: String,
        info: HashMap<String, Value>,
    },

    /// Genome assembly reference
    Genome {
        name: String,
        species: String,
        assembly: String,
        chromosomes: Vec<(String, i64)>,
    },

    /// Quality scores (Phred+33 encoded)
    Quality(Vec<u8>),

    /// Aligned sequencing read (SAM/BAM record)
    AlignedRead(bio_core::AlignedRead),
}

/// State for a Future value (lazy thunk from async fn).
#[derive(Debug, Clone)]
pub enum FutureState {
    /// Not yet evaluated — holds the function's params, body, and closure env
    Pending {
        params: Vec<Param>,
        body: Vec<Spanned<Stmt>>,
        closure_env: Option<usize>,
        args: Vec<Value>,
    },
    /// Already evaluated
    Resolved(Value),
}

/// Row-major table with named columns.
#[derive(Debug, Clone)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    /// Max display width per column (None = default 40).
    pub max_col_width: Option<usize>,
}

impl Table {
    pub fn new(columns: Vec<String>, rows: Vec<Vec<Value>>) -> Self {
        Self { columns, rows, max_col_width: None }
    }

    pub fn empty() -> Self {
        Self {
            columns: Vec::new(),
            rows: Vec::new(),
            max_col_width: None,
        }
    }

    /// Set the max display width per column.
    pub fn with_col_width(mut self, width: usize) -> Self {
        self.max_col_width = Some(width);
        self
    }

    pub fn num_rows(&self) -> usize {
        self.rows.len()
    }

    pub fn num_cols(&self) -> usize {
        self.columns.len()
    }

    pub fn col_index(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c == name)
    }

    /// Convert row i to a Record (HashMap).
    pub fn row_to_record(&self, i: usize) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        if let Some(row) = self.rows.get(i) {
            for (ci, col) in self.columns.iter().enumerate() {
                if let Some(val) = row.get(ci) {
                    map.insert(col.clone(), val.clone());
                }
            }
        }
        map
    }

    /// Build a Table from a list of Records. Column order from first record.
    pub fn from_records(records: &[HashMap<String, Value>]) -> Self {
        if records.is_empty() {
            return Self::empty();
        }
        // Collect columns in a stable order from first record
        let mut columns: Vec<String> = records[0].keys().cloned().collect();
        columns.sort();

        let rows: Vec<Vec<Value>> = records
            .iter()
            .map(|rec| {
                columns
                    .iter()
                    .map(|c| rec.get(c).cloned().unwrap_or(Value::Nil))
                    .collect()
            })
            .collect();

        Self { columns, rows, max_col_width: None }
    }
}

impl PartialEq for Table {
    fn eq(&self, other: &Self) -> bool {
        self.columns == other.columns && self.rows == other.rows
    }
}

/// A lazy stream backed by a shared iterator.
/// Streams are consumed once — calling next() advances the iterator.
/// After exhaustion, `is_exhausted()` returns true and subsequent iteration
/// attempts can be detected (use `check_not_exhausted()` for error reporting).
#[derive(Clone)]
pub struct StreamValue {
    pub label: String,
    inner: Arc<Mutex<Box<dyn Iterator<Item = Value> + Send>>>,
    exhausted: Arc<std::sync::atomic::AtomicBool>,
    started: Arc<std::sync::atomic::AtomicBool>,
}

impl StreamValue {
    pub fn new(label: impl Into<String>, iter: Box<dyn Iterator<Item = Value> + Send>) -> Self {
        Self {
            label: label.into(),
            inner: Arc::new(Mutex::new(iter)),
            exhausted: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            started: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Pull the next value from the stream. Returns None when exhausted.
    pub fn next(&self) -> Option<Value> {
        self.started.store(true, std::sync::atomic::Ordering::Relaxed);
        let result = self.inner.lock().unwrap().next();
        if result.is_none() {
            self.exhausted.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        result
    }

    /// Returns true if this stream has been fully consumed.
    pub fn is_exhausted(&self) -> bool {
        self.exhausted.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Returns true if iteration has started on this stream.
    pub fn is_started(&self) -> bool {
        self.started.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Collect all remaining items into a Vec.
    pub fn collect_all(&self) -> Vec<Value> {
        let mut items = Vec::new();
        while let Some(v) = self.next() {
            items.push(v);
        }
        items
    }

    /// Create a stream from an existing Vec (useful for testing and list-to-stream).
    pub fn from_list(label: impl Into<String>, items: Vec<Value>) -> Self {
        Self::new(label, Box::new(items.into_iter()))
    }
}

impl fmt::Debug for StreamValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Stream({})", self.label)
    }
}

impl PartialEq for StreamValue {
    fn eq(&self, other: &Self) -> bool {
        // Streams are identity-compared (same Arc)
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

/// Describes how many arguments a native function accepts.
#[derive(Debug, Clone, PartialEq)]
pub enum Arity {
    Exact(usize),
    AtLeast(usize),
    Range(usize, usize),
}

impl Value {
    pub fn type_of(&self) -> Type {
        match self {
            Value::Nil => Type::Nil,
            Value::Bool(_) => Type::Bool,
            Value::Int(_) => Type::Int,
            Value::Float(_) => Type::Float,
            Value::Str(_) => Type::Str,
            Value::List(_) => Type::List,
            Value::Map(_) => Type::Map,
            Value::Record(_) => Type::Record,
            Value::Function { .. } => Type::Function,
            Value::NativeFunction { .. } => Type::Function,
            Value::Formula(_) => Type::Formula,
            Value::DNA(_) => Type::DNA,
            Value::RNA(_) => Type::RNA,
            Value::Protein(_) => Type::Protein,
            Value::Stream(_) => Type::Stream,
            Value::Table(_) => Type::Table,
            Value::Interval(_) => Type::Interval,
            Value::Matrix(_) => Type::Matrix,
            Value::Range { .. } => Type::Range,
            Value::EnumValue { .. } => Type::Enum,
            Value::PluginFunction { .. } => Type::Function,
            Value::Set(_) => Type::Set,
            Value::Tuple(_) => Type::Tuple,
            Value::Regex { .. } => Type::Regex,
            Value::Future(_) => Type::Future,
            Value::Kmer(_) => Type::Kmer,
            Value::SparseMatrix(_) => Type::SparseMatrix,
            Value::CompiledClosure(_) => Type::Function,
            Value::Gene { .. } => Type::Gene,
            Value::Variant { .. } => Type::Variant,
            Value::Genome { .. } => Type::Genome,
            Value::Quality(_) => Type::Quality,
            Value::AlignedRead(_) => Type::AlignedRead,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::List(l) => !l.is_empty(),
            Value::Map(m) => !m.is_empty(),
            Value::Record(r) => !r.is_empty(),
            Value::Table(t) => !t.rows.is_empty(),
            Value::Matrix(m) => m.nrow > 0 && m.ncol > 0,
            Value::Range { start, end, inclusive } => {
                if *inclusive { end >= start } else { end > start }
            }
            Value::Set(items) => !items.is_empty(),
            Value::Tuple(items) => !items.is_empty(),
            Value::Kmer(_) => true,
            Value::SparseMatrix(sm) => sm.nnz() > 0,
            Value::Quality(q) => !q.is_empty(),
            _ => true,
        }
    }

    pub fn is_callable(&self) -> bool {
        matches!(self, Value::Function { .. } | Value::NativeFunction { .. })
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Int(n) => Some(*n as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

/// If `items` is a uniform list of Records (all same keys), convert to Table for display.
fn list_of_records_as_table(items: &[Value]) -> Option<Table> {
    // Extract keys from first record
    let first_keys: Vec<String> = match &items[0] {
        Value::Record(m) => {
            let mut keys: Vec<String> = m.keys().cloned().collect();
            keys.sort();
            keys
        }
        _ => return None,
    };
    // Verify all items are Records with the same keys
    for item in &items[1..] {
        match item {
            Value::Record(m) => {
                let mut keys: Vec<String> = m.keys().cloned().collect();
                keys.sort();
                if keys != first_keys {
                    return None;
                }
            }
            _ => return None,
        }
    }
    let rows: Vec<Vec<Value>> = items
        .iter()
        .map(|item| {
            if let Value::Record(m) = item {
                first_keys
                    .iter()
                    .map(|k| m.get(k).cloned().unwrap_or(Value::Nil))
                    .collect()
            } else {
                vec![]
            }
        })
        .collect();
    Some(Table::new(first_keys, rows))
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Nil => write!(f, "nil"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(n) => write!(f, "{n}"),
            Value::Float(v) => {
                if v.fract() == 0.0 {
                    write!(f, "{v:.1}")
                } else {
                    write!(f, "{v}")
                }
            }
            Value::Str(s) => write!(f, "{s}"),
            Value::List(items) => {
                // Auto-table: if all items are Records with the same keys, display as table
                if items.len() >= 2 {
                    if let Some(table) = list_of_records_as_table(items) {
                        return write!(f, "{}", Value::Table(table));
                    }
                }
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
            Value::Map(map) | Value::Record(map) => {
                write!(f, "{{")?;
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, "}}")
            }
            Value::Function { name, .. } => {
                write!(f, "<fn {}>", name.as_deref().unwrap_or("anonymous"))
            }
            Value::NativeFunction { name, .. } => write!(f, "<builtin {name}>"),
            Value::Formula(_) => write!(f, "<formula>"),
            Value::DNA(seq) => write!(f, "DNA({})", seq.data),
            Value::RNA(seq) => write!(f, "RNA({})", seq.data),
            Value::Protein(seq) => write!(f, "Protein({})", seq.data),
            Value::Stream(s) => write!(f, "<stream {}>", s.label),
            Value::Table(t) => {
                writeln!(f, "Table: {} rows x {} cols", t.num_rows(), t.num_cols())?;
                if t.columns.is_empty() {
                    return write!(f, "(empty)");
                }
                // Compute column widths (header vs data), cap per column
                let show = t.rows.len().min(20);
                let mut widths: Vec<usize> = t.columns.iter().map(|c| c.chars().count()).collect();
                for row in &t.rows[..show] {
                    for (i, val) in row.iter().enumerate() {
                        if i < widths.len() {
                            let s = format!("{val}");
                            widths[i] = widths[i].max(s.chars().count());
                        }
                    }
                }
                // Cap each column width
                let cap = t.max_col_width.unwrap_or(40);
                for w in &mut widths {
                    *w = (*w).min(cap);
                }
                // Header
                let hdr: Vec<String> = t
                    .columns
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let clen = c.chars().count();
                        if clen < widths[i] {
                            format!("{}{}", c, " ".repeat(widths[i] - clen))
                        } else {
                            c.clone()
                        }
                    })
                    .collect();
                writeln!(f, " {}", hdr.join(" | "))?;
                // Separator
                let sep: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
                writeln!(f, " {}", sep.join("-+-"))?;
                // Data rows
                for row in &t.rows[..show] {
                    let vals: Vec<String> = row
                        .iter()
                        .enumerate()
                        .map(|(i, v)| {
                            let s = format!("{v}");
                            let w = widths.get(i).copied().unwrap_or(0);
                            let slen = s.chars().count();
                            if slen > w {
                                let truncated: String = s.chars().take(w.saturating_sub(1)).collect();
                                format!("{}~", truncated)
                            } else if slen < w {
                                format!("{}{}", s, " ".repeat(w - slen))
                            } else {
                                s
                            }
                        })
                        .collect();
                    writeln!(f, " {}", vals.join(" | "))?;
                }
                if t.rows.len() > show {
                    write!(f, " # {} more rows", t.rows.len() - show)?;
                }
                Ok(())
            }
            Value::Interval(iv) => write!(f, "{iv}"),
            Value::Matrix(m) => write!(f, "{m}"),
            Value::Range { start, end, inclusive } => {
                if *inclusive {
                    write!(f, "{start}..={end}")
                } else {
                    write!(f, "{start}..{end}")
                }
            }
            Value::EnumValue { enum_name, variant, fields } => {
                if fields.is_empty() {
                    write!(f, "{enum_name}::{variant}")
                } else {
                    let args: Vec<String> = fields.iter().map(|v| format!("{v}")).collect();
                    write!(f, "{enum_name}::{variant}({})", args.join(", "))
                }
            }
            Value::PluginFunction {
                plugin_name,
                operation,
                ..
            } => write!(f, "<plugin:{plugin_name}.{operation}>"),
            Value::Set(items) => {
                write!(f, "#{{")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "}}")
            }
            Value::Regex { pattern, flags } => write!(f, "/{pattern}/{flags}"),
            Value::Future(_) => write!(f, "<future>"),
            Value::Kmer(km) => write!(f, "{km}"),
            Value::SparseMatrix(sm) => write!(f, "{sm}"),
            Value::Tuple(items) => {
                write!(f, "(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                if items.len() == 1 {
                    write!(f, ",")?;
                }
                write!(f, ")")
            }
            Value::CompiledClosure(_) => write!(f, "<compiled fn>"),
            Value::Gene { symbol, chrom, start, end, strand, biotype, .. } => {
                write!(f, "Gene({symbol} {chrom}:{start}-{end}:{strand} [{biotype}])")
            }
            Value::Variant { chrom, pos, ref_allele, alt_allele, quality, .. } => {
                write!(f, "Variant({chrom}:{pos} {ref_allele}>{alt_allele} Q={quality:.0})")
            }
            Value::Genome { name, assembly, .. } => write!(f, "Genome({name} {assembly})"),
            Value::Quality(scores) => {
                let ascii: String = scores.iter().map(|&b| (b + 33) as char).collect();
                write!(f, "Quality({ascii})")
            }
            Value::AlignedRead(r) => {
                write!(f, "AlignedRead({} {}:{} {})", r.qname, r.rname, r.pos, r.cigar)
            }
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Nil, Value::Nil) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Int(a), Value::Float(b)) => (*a as f64) == *b,
            (Value::Float(a), Value::Int(b)) => *a == (*b as f64),
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::DNA(a), Value::DNA(b)) => a == b,
            (Value::RNA(a), Value::RNA(b)) => a == b,
            (Value::Protein(a), Value::Protein(b)) => a == b,
            (Value::Table(a), Value::Table(b)) => a == b,
            (Value::Interval(a), Value::Interval(b)) => a == b,
            (Value::Matrix(a), Value::Matrix(b)) => a == b,
            (
                Value::Range { start: s1, end: e1, inclusive: i1 },
                Value::Range { start: s2, end: e2, inclusive: i2 },
            ) => s1 == s2 && e1 == e2 && i1 == i2,
            (
                Value::EnumValue { enum_name: a, variant: av, fields: af },
                Value::EnumValue { enum_name: b, variant: bv, fields: bf },
            ) => a == b && av == bv && af == bf,
            (
                Value::PluginFunction {
                    plugin_name: a,
                    operation: ao,
                    ..
                },
                Value::PluginFunction {
                    plugin_name: b,
                    operation: bo,
                    ..
                },
            ) => a == b && ao == bo,
            (Value::Set(a), Value::Set(b)) => a == b,
            (Value::Tuple(a), Value::Tuple(b)) => a == b,
            (
                Value::Regex { pattern: a, flags: af },
                Value::Regex { pattern: b, flags: bf },
            ) => a == b && af == bf,
            (Value::Kmer(a), Value::Kmer(b)) => a == b,
            (Value::SparseMatrix(a), Value::SparseMatrix(b)) => a == b,
            (Value::CompiledClosure(a), Value::CompiledClosure(b)) => Arc::ptr_eq(a, b),
            (
                Value::Gene { symbol: a, gene_id: ai, .. },
                Value::Gene { symbol: b, gene_id: bi, .. },
            ) => a == b && ai == bi,
            (
                Value::Variant { chrom: ac, pos: ap, ref_allele: ar, alt_allele: aa, .. },
                Value::Variant { chrom: bc, pos: bp, ref_allele: br, alt_allele: ba, .. },
            ) => ac == bc && ap == bp && ar == br && aa == ba,
            (
                Value::Genome { name: an, assembly: aa, .. },
                Value::Genome { name: bn, assembly: ba, .. },
            ) => an == bn && aa == ba,
            (Value::Quality(a), Value::Quality(b)) => a == b,
            (Value::AlignedRead(a), Value::AlignedRead(b)) => a == b,
            _ => false,
        }
    }
}
