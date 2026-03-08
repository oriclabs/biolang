use std::fmt;

/// Runtime type tags for dynamic dispatch and error messages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Nil,
    Bool,
    Int,
    Float,
    Str,
    DNA,
    RNA,
    Protein,
    Interval,
    Table,
    List,
    Map,
    File,
    Formula,
    Function,
    Stream,
    SomerJobHandle,
    Record,
    Matrix,
    Range,
    Enum,
    Set,
    Tuple,
    Regex,
    Future,
    Kmer,
    SparseMatrix,
    Gene,
    Variant,
    Genome,
    Quality,
    AlignedRead,
    /// Gradual typing: matches everything (unannotated = Any).
    Any,
    /// Union type: `Int | Str` — value may be any of the listed types.
    Union(Vec<Type>),
    /// Optional type: `Int?` — value may be the inner type or Nil.
    Optional(Box<Type>),
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Nil => write!(f, "Nil"),
            Type::Bool => write!(f, "Bool"),
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::Str => write!(f, "Str"),
            Type::DNA => write!(f, "DNA"),
            Type::RNA => write!(f, "RNA"),
            Type::Protein => write!(f, "Protein"),
            Type::Interval => write!(f, "Interval"),
            Type::Table => write!(f, "Table"),
            Type::List => write!(f, "List"),
            Type::Map => write!(f, "Map"),
            Type::File => write!(f, "File"),
            Type::Formula => write!(f, "Formula"),
            Type::Function => write!(f, "Function"),
            Type::Stream => write!(f, "Stream"),
            Type::SomerJobHandle => write!(f, "SomerJobHandle"),
            Type::Record => write!(f, "Record"),
            Type::Matrix => write!(f, "Matrix"),
            Type::Range => write!(f, "Range"),
            Type::Enum => write!(f, "Enum"),
            Type::Set => write!(f, "Set"),
            Type::Tuple => write!(f, "Tuple"),
            Type::Regex => write!(f, "Regex"),
            Type::Future => write!(f, "Future"),
            Type::Kmer => write!(f, "Kmer"),
            Type::SparseMatrix => write!(f, "SparseMatrix"),
            Type::Gene => write!(f, "Gene"),
            Type::Variant => write!(f, "Variant"),
            Type::Genome => write!(f, "Genome"),
            Type::Quality => write!(f, "Quality"),
            Type::AlignedRead => write!(f, "AlignedRead"),
            Type::Any => write!(f, "Any"),
            Type::Union(types) => {
                let parts: Vec<String> = types.iter().map(|t| t.to_string()).collect();
                write!(f, "{}", parts.join(" | "))
            }
            Type::Optional(inner) => write!(f, "{}?", inner),
        }
    }
}
