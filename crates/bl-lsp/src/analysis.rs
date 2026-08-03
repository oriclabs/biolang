use bl_core::ast::{Expr, Param, RecordEntry, Stmt, TypeAnnotation};
use bl_core::span::{Span, Spanned};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SymbolKind {
    Variable,
    Function,
    Parameter,
    Import,
    Enum,
    Struct,
    Field,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeInfo {
    Unknown,
    Named(String),
    List(Box<TypeInfo>),
    Record(BTreeMap<String, TypeInfo>),
    Function(Box<FunctionInfo>),
    Module(Box<ModuleInfo>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParameterInfo {
    pub name: String,
    pub type_info: TypeInfo,
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionInfo {
    pub name: String,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: TypeInfo,
    pub signature: String,
    pub documentation: Option<String>,
    pub span: Option<Span>,
    pub source: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub span: Span,
    pub type_info: TypeInfo,
    pub signature: Option<String>,
    pub documentation: Option<String>,
    pub value_preview: Option<String>,
    pub source: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleInfo {
    pub name: String,
    pub path: Option<PathBuf>,
    pub exports: BTreeMap<String, Symbol>,
}

#[derive(Debug, Clone, Default)]
pub struct DocumentAnalysis {
    pub symbols: BTreeMap<String, Symbol>,
}

pub trait ModuleLoader {
    fn load_module(&mut self, import_path: &str, from: Option<&Path>) -> Option<ModuleInfo>;
}

pub struct NoModules;

impl ModuleLoader for NoModules {
    fn load_module(&mut self, _import_path: &str, _from: Option<&Path>) -> Option<ModuleInfo> {
        None
    }
}

#[derive(Debug, Clone)]
pub struct FileModuleLoader {
    root: Option<PathBuf>,
    cache: HashMap<PathBuf, ModuleInfo>,
    loading: HashSet<PathBuf>,
}

impl FileModuleLoader {
    pub fn new(root: Option<PathBuf>) -> Self {
        Self {
            root,
            cache: HashMap::new(),
            loading: HashSet::new(),
        }
    }

    fn resolve(&self, import_path: &str, from: Option<&Path>) -> Option<PathBuf> {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()
            .map(PathBuf::from);
        let mut roots = Vec::new();
        if let Some(parent) = from.and_then(Path::parent) {
            roots.push(parent.to_path_buf());
        }
        if let Some(root) = &self.root {
            roots.push(root.clone());
        }
        if let Ok(cwd) = std::env::current_dir() {
            roots.push(cwd);
        }
        if let Ok(value) = std::env::var("BIOLANG_PATH") {
            roots.extend(std::env::split_paths(&value).filter(|path| path.is_dir()));
        }
        if let Some(home) = &home {
            roots.push(home.join(".biolang").join("stdlib"));
            roots.push(home.join(".biolang").join("packages"));
        }

        if let Some(rest) = import_path.strip_prefix("std/") {
            return home
                .as_ref()
                .and_then(|dir| resolve_from_root(&dir.join(".biolang").join("stdlib"), rest));
        }
        if let Some(rest) = import_path.strip_prefix("pkg/") {
            return home
                .as_ref()
                .and_then(|dir| resolve_from_root(&dir.join(".biolang").join("packages"), rest));
        }
        roots
            .into_iter()
            .find_map(|root| resolve_from_root(&root, import_path))
    }
}

impl ModuleLoader for FileModuleLoader {
    fn load_module(&mut self, import_path: &str, from: Option<&Path>) -> Option<ModuleInfo> {
        let path = self.resolve(import_path, from)?;
        if let Some(module) = self.cache.get(&path) {
            return Some(module.clone());
        }
        if !self.loading.insert(path.clone()) {
            return None;
        }
        let source = std::fs::read_to_string(&path).ok()?;
        let analysis = analyze_source(&source, Some(&path), self);
        self.loading.remove(&path);
        let name = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or(import_path)
            .to_string();
        let module = ModuleInfo {
            name,
            path: Some(path.clone()),
            exports: analysis
                .symbols
                .into_iter()
                .filter(|(_, symbol)| symbol.kind != SymbolKind::Parameter)
                .collect(),
        };
        self.cache.insert(path, module.clone());
        Some(module)
    }
}

fn resolve_from_root(root: &Path, import_path: &str) -> Option<PathBuf> {
    let direct = root.join(import_path);
    let mut candidates = Vec::new();
    if direct.extension().is_some() {
        candidates.push(direct.clone());
    } else {
        candidates.push(root.join(format!("{import_path}.bl")));
    }
    if direct.is_dir() {
        if let Ok(Some(entry)) = bl_runtime::package::resolve_library_entry(&direct) {
            candidates.insert(0, entry);
        }
        candidates.push(direct.join("main.bl"));
        candidates.push(direct.join("lib.bl"));
    }
    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .and_then(|candidate| candidate.canonicalize().ok().or(Some(candidate)))
}

pub fn analyze_source(
    source: &str,
    source_path: Option<&Path>,
    loader: &mut dyn ModuleLoader,
) -> DocumentAnalysis {
    let Ok(tokens) = bl_lexer::Lexer::new(source).tokenize() else {
        return DocumentAnalysis::default();
    };
    let Ok(parsed) = bl_parser::Parser::new(tokens).parse() else {
        return DocumentAnalysis::default();
    };
    let statements = parsed.program.stmts;
    let mut symbols = BTreeMap::new();

    for statement in &statements {
        match &statement.node {
            Stmt::Import {
                path,
                alias: Some(alias),
            } => {
                let module = loader.load_module(path, source_path);
                let type_info = module
                    .map(|value| TypeInfo::Module(Box::new(value)))
                    .unwrap_or_else(|| TypeInfo::Named("Module".into()));
                symbols.insert(
                    alias.clone(),
                    Symbol {
                        name: alias.clone(),
                        kind: SymbolKind::Import,
                        span: statement.span,
                        type_info,
                        signature: None,
                        documentation: Some(format!("Imported module `{path}`.")),
                        value_preview: None,
                        source: source_path.map(Path::to_path_buf),
                    },
                );
            }
            Stmt::FromImport { path, names } => {
                if let Some(module) = loader.load_module(path, source_path) {
                    for name in names {
                        if let Some(export) = module.exports.get(name) {
                            symbols.insert(name.clone(), export.clone());
                        }
                    }
                }
            }
            Stmt::Struct { name, fields } => {
                let record = fields
                    .iter()
                    .map(|field| {
                        (
                            field.name.clone(),
                            field
                                .type_ann
                                .as_ref()
                                .map(type_from_annotation)
                                .unwrap_or(TypeInfo::Unknown),
                        )
                    })
                    .collect();
                symbols.insert(
                    name.clone(),
                    Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Struct,
                        span: statement.span,
                        type_info: TypeInfo::Record(record),
                        signature: None,
                        documentation: None,
                        value_preview: None,
                        source: source_path.map(Path::to_path_buf),
                    },
                );
            }
            Stmt::Enum { name, .. } => {
                symbols.insert(
                    name.clone(),
                    simple_symbol(
                        name,
                        SymbolKind::Enum,
                        statement.span,
                        TypeInfo::Named(name.clone()),
                        source_path,
                    ),
                );
            }
            Stmt::Fn {
                name,
                params,
                return_type,
                doc,
                ..
            } => {
                let parameters = parameter_info(params);
                let declared = return_type
                    .as_ref()
                    .map(type_from_annotation)
                    .unwrap_or(TypeInfo::Unknown);
                let info = FunctionInfo {
                    name: name.clone(),
                    parameters,
                    return_type: declared.clone(),
                    signature: function_signature(name, params, &declared),
                    documentation: doc.clone(),
                    span: Some(statement.span),
                    source: source_path.map(Path::to_path_buf),
                };
                symbols.insert(
                    name.clone(),
                    Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Function,
                        span: statement.span,
                        type_info: TypeInfo::Function(Box::new(info.clone())),
                        signature: Some(info.signature),
                        documentation: doc.clone(),
                        value_preview: None,
                        source: source_path.map(Path::to_path_buf),
                    },
                );
            }
            _ => {}
        }
    }

    // Resolve function return shapes iteratively so forwarding wrappers inherit
    // the structural type of the function they call.
    for _ in 0..4 {
        let snapshot = symbols.clone();
        let mut changed = false;
        for statement in &statements {
            let Stmt::Fn {
                name,
                params,
                return_type,
                body,
                ..
            } = &statement.node
            else {
                continue;
            };
            let mut env = snapshot.clone();
            for parameter in parameter_info(params) {
                env.insert(
                    parameter.name.clone(),
                    Symbol {
                        name: parameter.name.clone(),
                        kind: SymbolKind::Parameter,
                        span: statement.span,
                        type_info: parameter.type_info,
                        signature: None,
                        documentation: None,
                        value_preview: None,
                        source: source_path.map(Path::to_path_buf),
                    },
                );
            }
            let inferred = return_type
                .as_ref()
                .map(type_from_annotation)
                .unwrap_or_else(|| infer_block(body, &env));
            if let Some(symbol) = symbols.get_mut(name) {
                if let TypeInfo::Function(function) = &mut symbol.type_info {
                    if function.return_type != inferred {
                        function.return_type = inferred.clone();
                        function.signature = function_signature(name, params, &inferred);
                        symbol.signature = Some(function.signature.clone());
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }

    for statement in &statements {
        match &statement.node {
            Stmt::Let {
                name,
                type_ann,
                value,
            }
            | Stmt::Const {
                name,
                type_ann,
                value,
            } => {
                let type_info = type_ann
                    .as_ref()
                    .map(type_from_annotation)
                    .unwrap_or_else(|| infer_expr(value, &symbols));
                symbols.insert(
                    name.clone(),
                    Symbol {
                        name: name.clone(),
                        kind: SymbolKind::Variable,
                        span: statement.span,
                        type_info,
                        signature: None,
                        documentation: None,
                        value_preview: primitive_preview(&value.node),
                        source: source_path.map(Path::to_path_buf),
                    },
                );
            }
            Stmt::Assign { name, value } | Stmt::NilAssign { name, value } => {
                let inferred = infer_expr(value, &symbols);
                if let Some(symbol) = symbols.get_mut(name) {
                    symbol.type_info = merge_types(symbol.type_info.clone(), inferred);
                    symbol.value_preview = primitive_preview(&value.node);
                }
            }
            _ => {}
        }
    }

    DocumentAnalysis { symbols }
}

fn simple_symbol(
    name: &str,
    kind: SymbolKind,
    span: Span,
    type_info: TypeInfo,
    source: Option<&Path>,
) -> Symbol {
    Symbol {
        name: name.to_string(),
        kind,
        span,
        type_info,
        signature: None,
        documentation: None,
        value_preview: None,
        source: source.map(Path::to_path_buf),
    }
}

fn parameter_info(params: &[Param]) -> Vec<ParameterInfo> {
    params
        .iter()
        .map(|parameter| ParameterInfo {
            name: parameter.name.clone(),
            type_info: parameter
                .type_ann
                .as_ref()
                .map(type_from_annotation)
                .unwrap_or(TypeInfo::Unknown),
            optional: parameter.default.is_some() || parameter.rest,
        })
        .collect()
}

fn type_from_annotation(annotation: &TypeAnnotation) -> TypeInfo {
    if annotation.name == "List" && annotation.params.len() == 1 {
        return TypeInfo::List(Box::new(type_from_annotation(&annotation.params[0])));
    }
    TypeInfo::Named(format_annotation(annotation))
}

fn format_annotation(annotation: &TypeAnnotation) -> String {
    if annotation.params.is_empty() {
        annotation.name.clone()
    } else {
        format!(
            "{}<{}>",
            annotation.name,
            annotation
                .params
                .iter()
                .map(format_annotation)
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

fn function_signature(name: &str, params: &[Param], return_type: &TypeInfo) -> String {
    let parameters = params
        .iter()
        .map(|parameter| {
            let mut text = if parameter.rest {
                format!("...{}", parameter.name)
            } else {
                parameter.name.clone()
            };
            if let Some(annotation) = &parameter.type_ann {
                text.push_str(": ");
                text.push_str(&format_annotation(annotation));
            }
            if parameter.default.is_some() {
                text.push_str(" = …");
            }
            text
        })
        .collect::<Vec<_>>()
        .join(", ");
    let result = return_type.display();
    if result == "Any" {
        format!("{name}({parameters})")
    } else {
        format!("{name}({parameters}) -> {result}")
    }
}

fn infer_block(body: &[Spanned<Stmt>], env: &BTreeMap<String, Symbol>) -> TypeInfo {
    let mut local = env.clone();
    let mut result = TypeInfo::Unknown;
    for statement in body {
        match &statement.node {
            Stmt::Let {
                name,
                type_ann,
                value,
            }
            | Stmt::Const {
                name,
                type_ann,
                value,
            } => {
                let type_info = type_ann
                    .as_ref()
                    .map(type_from_annotation)
                    .unwrap_or_else(|| infer_expr(value, &local));
                local.insert(
                    name.clone(),
                    simple_symbol(name, SymbolKind::Variable, statement.span, type_info, None),
                );
            }
            Stmt::Assign { name, value } | Stmt::NilAssign { name, value } => {
                let inferred = infer_expr(value, &local);
                if let Some(symbol) = local.get_mut(name) {
                    symbol.type_info = merge_types(symbol.type_info.clone(), inferred);
                }
            }
            Stmt::Return(Some(value)) => {
                result = merge_types(result, infer_expr(value, &local));
            }
            Stmt::Expr(value) => result = merge_types(result, infer_expr(value, &local)),
            _ => {}
        }
    }
    result
}

fn infer_expr(expr: &Spanned<Expr>, env: &BTreeMap<String, Symbol>) -> TypeInfo {
    match &expr.node {
        Expr::Nil => TypeInfo::Named("Nil".into()),
        Expr::Bool(_) => TypeInfo::Named("Bool".into()),
        Expr::Int(_) => TypeInfo::Named("Int".into()),
        Expr::Float(_) => TypeInfo::Named("Float".into()),
        Expr::Str(_) | Expr::StringInterp(_) => TypeInfo::Named("Str".into()),
        Expr::DnaLit(_) => TypeInfo::Named("DNA".into()),
        Expr::RnaLit(_) => TypeInfo::Named("RNA".into()),
        Expr::ProteinLit(_) => TypeInfo::Named("Protein".into()),
        Expr::QualLit(_) => TypeInfo::Named("Quality".into()),
        Expr::Ident(name) => env
            .get(name)
            .map(|symbol| symbol.type_info.clone())
            .or_else(|| builtin(name).map(|info| TypeInfo::Function(Box::new(info.function()))))
            .unwrap_or(TypeInfo::Unknown),
        Expr::Record(entries) => {
            let mut fields = BTreeMap::new();
            for entry in entries {
                match entry {
                    RecordEntry::Field(name, value) => {
                        fields.insert(name.clone(), infer_expr(value, env));
                    }
                    RecordEntry::Spread(value) => {
                        if let TypeInfo::Record(spread) = infer_expr(value, env) {
                            fields.extend(spread);
                        }
                    }
                }
            }
            TypeInfo::Record(fields)
        }
        Expr::StructLit { name, fields } => TypeInfo::Record(
            fields
                .iter()
                .map(|(field, value)| (field.clone(), infer_expr(value, env)))
                .chain(std::iter::once((
                    "__type".into(),
                    TypeInfo::Named(name.clone()),
                )))
                .collect(),
        ),
        Expr::List(values) | Expr::TupleLit(values) => {
            let inner = values
                .iter()
                .map(|value| infer_expr(value, env))
                .reduce(merge_types)
                .unwrap_or(TypeInfo::Unknown);
            TypeInfo::List(Box::new(inner))
        }
        Expr::Field { object, field, .. } => match infer_expr(object, env) {
            TypeInfo::Record(fields) => fields.get(field).cloned().unwrap_or(TypeInfo::Unknown),
            TypeInfo::Module(module) => module
                .exports
                .get(field)
                .map(|symbol| symbol.type_info.clone())
                .unwrap_or(TypeInfo::Unknown),
            _ => TypeInfo::Unknown,
        },
        Expr::Call { callee, .. } => match infer_expr(callee, env) {
            TypeInfo::Function(function) => function.return_type.clone(),
            _ => TypeInfo::Unknown,
        },
        Expr::Index { object, .. } => match infer_expr(object, env) {
            TypeInfo::List(inner) => *inner,
            _ => TypeInfo::Unknown,
        },
        Expr::Block(body) => infer_block(body, env),
        Expr::If {
            then_body,
            else_body,
            ..
        } => merge_types(
            infer_block(then_body, env),
            else_body
                .as_ref()
                .map(|body| infer_block(body, env))
                .unwrap_or(TypeInfo::Named("Nil".into())),
        ),
        Expr::TryCatch {
            body, catch_body, ..
        } => merge_types(infer_block(body, env), infer_block(catch_body, env)),
        Expr::NullCoalesce { left, right } => {
            merge_types(infer_expr(left, env), infer_expr(right, env))
        }
        Expr::Ternary {
            value, else_value, ..
        } => merge_types(infer_expr(value, env), infer_expr(else_value, env)),
        Expr::Pipe { right, .. } | Expr::TapPipe { right, .. } => infer_expr(right, env),
        Expr::TypeCast { target, .. } => TypeInfo::Named(target.clone()),
        Expr::Binary { op, left, right } => {
            use bl_core::ast::BinaryOp;
            match op {
                BinaryOp::Eq
                | BinaryOp::Neq
                | BinaryOp::Lt
                | BinaryOp::Gt
                | BinaryOp::Le
                | BinaryOp::Ge
                | BinaryOp::And
                | BinaryOp::Or => TypeInfo::Named("Bool".into()),
                _ => merge_types(infer_expr(left, env), infer_expr(right, env)),
            }
        }
        Expr::Range { .. } => TypeInfo::List(Box::new(TypeInfo::Named("Int".into()))),
        _ => TypeInfo::Unknown,
    }
}

fn merge_types(left: TypeInfo, right: TypeInfo) -> TypeInfo {
    match (left, right) {
        (TypeInfo::Unknown, value) | (value, TypeInfo::Unknown) => value,
        (TypeInfo::Record(mut left), TypeInfo::Record(right)) => {
            for (name, value) in right {
                left.entry(name)
                    .and_modify(|current| {
                        *current = merge_types(current.clone(), value.clone());
                    })
                    .or_insert(value);
            }
            TypeInfo::Record(left)
        }
        (TypeInfo::List(left), TypeInfo::List(right)) => {
            TypeInfo::List(Box::new(merge_types(*left, *right)))
        }
        (left, right) if left == right => left,
        _ => TypeInfo::Unknown,
    }
}

impl TypeInfo {
    pub fn display(&self) -> String {
        match self {
            Self::Unknown => "Any".into(),
            Self::Named(name) => name.clone(),
            Self::List(inner) => format!("List<{}>", inner.display()),
            Self::Record(fields) => {
                let names = fields
                    .iter()
                    .filter(|(name, _)| name.as_str() != "__type")
                    .map(|(name, value)| format!("{name}: {}", value.display()))
                    .collect::<Vec<_>>();
                if names.is_empty() {
                    "Record".into()
                } else {
                    format!("Record{{{}}}", names.join(", "))
                }
            }
            Self::Function(function) => function.signature.clone(),
            Self::Module(module) => format!("module {}", module.name),
        }
    }

    pub fn members(&self) -> Vec<Symbol> {
        match self {
            Self::Record(fields) => fields
                .iter()
                .filter(|(name, _)| !name.starts_with('_'))
                .map(|(name, type_info)| Symbol {
                    name: name.clone(),
                    kind: SymbolKind::Field,
                    span: Span::default(),
                    type_info: type_info.clone(),
                    signature: None,
                    documentation: None,
                    value_preview: None,
                    source: None,
                })
                .collect(),
            Self::Module(module) => module
                .exports
                .values()
                .filter(|symbol| !symbol.name.starts_with('_'))
                .cloned()
                .collect(),
            _ => Vec::new(),
        }
    }
}

fn primitive_preview(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Nil => Some("nil".into()),
        Expr::Bool(value) => Some(value.to_string()),
        Expr::Int(value) => Some(value.to_string()),
        Expr::Float(value) => Some(value.to_string()),
        Expr::Str(value) => Some(format!("\"{value}\"")),
        Expr::DnaLit(value) => Some(format!("dna\"{value}\"")),
        Expr::RnaLit(value) => Some(format!("rna\"{value}\"")),
        Expr::ProteinLit(value) => Some(format!("protein\"{value}\"")),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct BuiltinInfo {
    pub name: String,
    pub signature: String,
    pub summary: String,
    pub category: String,
    pub return_type: TypeInfo,
}

impl BuiltinInfo {
    pub fn function(&self) -> FunctionInfo {
        let parameters = signature_parameters(&self.signature)
            .into_iter()
            .map(|name| ParameterInfo {
                optional: name.ends_with('?') || name.contains('='),
                name,
                type_info: TypeInfo::Unknown,
            })
            .collect();
        FunctionInfo {
            name: self.name.clone(),
            parameters,
            return_type: self.return_type.clone(),
            signature: self.signature.clone(),
            documentation: Some(self.summary.clone()),
            span: None,
            source: None,
        }
    }
}

fn builtin_catalog() -> &'static BTreeMap<String, BuiltinInfo> {
    static CATALOG: OnceLock<BTreeMap<String, BuiltinInfo>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        bl_repl::biolang_metadata()
            .builtins
            .into_iter()
            .map(|entry| {
                let return_text = entry
                    .return_type
                    .clone()
                    .or_else(|| signature_return(&entry.signature));
                let return_type = return_text
                    .as_deref()
                    .map(parse_type)
                    .unwrap_or(TypeInfo::Unknown);
                (
                    entry.name.clone(),
                    BuiltinInfo {
                        name: entry.name,
                        signature: normalize_arrow(&entry.signature),
                        summary: entry.summary,
                        category: entry.category,
                        return_type,
                    },
                )
            })
            .collect()
    })
}

pub fn builtins() -> impl Iterator<Item = &'static BuiltinInfo> {
    builtin_catalog().values()
}

pub fn builtin(name: &str) -> Option<&'static BuiltinInfo> {
    builtin_catalog().get(name)
}

fn normalize_arrow(value: &str) -> String {
    value.replace('→', "->")
}

fn signature_return(signature: &str) -> Option<String> {
    signature
        .split_once("->")
        .or_else(|| signature.split_once('→'))
        .map(|(_, value)| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn signature_parameters(signature: &str) -> Vec<String> {
    let Some(open) = signature.find('(') else {
        return Vec::new();
    };
    let mut depth = 0usize;
    let mut close = None;
    for (index, character) in signature[open + 1..].char_indices() {
        match character {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' if depth == 0 => {
                close = Some(open + 1 + index);
                break;
            }
            ')' | ']' | '}' | '>' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    let Some(close) = close else {
        return Vec::new();
    };
    signature[open + 1..close]
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_type(value: &str) -> TypeInfo {
    let value = value.trim();
    if let Some(fields) = value
        .strip_prefix("Record{")
        .and_then(|rest| rest.split_once('}').map(|(fields, _)| fields))
    {
        return TypeInfo::Record(
            fields
                .split(',')
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(|name| {
                    let (name, type_info) = name
                        .split_once(':')
                        .map(|(name, kind)| (name.trim(), parse_type(kind)))
                        .unwrap_or((name, TypeInfo::Unknown));
                    (name.to_string(), type_info)
                })
                .collect(),
        );
    }
    if let Some(inner) = value
        .strip_prefix("List<")
        .and_then(|rest| rest.strip_suffix('>'))
    {
        return TypeInfo::List(Box::new(parse_type(inner)));
    }
    let name = value
        .split_whitespace()
        .next()
        .unwrap_or("Any")
        .trim_matches(|character: char| character == '(' || character == ')');
    if name.eq_ignore_ascii_case("any") || name.is_empty() {
        TypeInfo::Unknown
    } else {
        TypeInfo::Named(name.to_string())
    }
}

pub fn keywords() -> &'static [&'static str] {
    &[
        "let", "const", "fn", "if", "else", "for", "in", "while", "break", "continue", "return",
        "match", "import", "as", "true", "false", "nil", "and", "or", "not", "try", "catch",
        "pipeline", "stage", "assert", "yield", "enum", "struct", "trait", "impl",
    ]
}

pub fn member_target(source: &str, offset: usize) -> Option<String> {
    let before = source.get(..offset)?;
    let trimmed = before.trim_end();
    let dot = trimmed.rfind('.')?;
    let target = trimmed[..dot].trim_end();
    let start = target
        .rfind(|character: char| !character.is_alphanumeric() && character != '_')
        .map(|index| index + 1)
        .unwrap_or(0);
    let value = &target[start..];
    (!value.is_empty()).then(|| value.to_string())
}

pub fn word_at(source: &str, offset: usize) -> Option<(String, Option<String>)> {
    let offset = offset.min(source.len());
    let bytes = source.as_bytes();
    let mut start = offset;
    while start > 0 {
        let character = bytes[start - 1] as char;
        if !character.is_ascii_alphanumeric() && character != '_' {
            break;
        }
        start -= 1;
    }
    let mut end = offset;
    while end < bytes.len() {
        let character = bytes[end] as char;
        if !character.is_ascii_alphanumeric() && character != '_' {
            break;
        }
        end += 1;
    }
    let word = source.get(start..end)?.to_string();
    if word.is_empty() {
        return None;
    }
    let qualifier = source.get(..start)?.strip_suffix('.').and_then(|before| {
        let begin = before
            .rfind(|character: char| !character.is_alphanumeric() && character != '_')
            .map(|index| index + 1)
            .unwrap_or(0);
        let value = &before[begin..];
        (!value.is_empty()).then(|| value.to_string())
    });
    Some((word, qualifier))
}

pub fn call_at(source: &str, offset: usize) -> Option<(String, u32)> {
    let before = source.get(..offset)?;
    let mut depth = 0usize;
    let mut open = None;
    for (index, character) in before.char_indices().rev() {
        match character {
            ')' | ']' | '}' => depth += 1,
            '(' if depth == 0 => {
                open = Some(index);
                break;
            }
            '(' | '[' | '{' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    let open = open?;
    let callee_end = before[..open].trim_end().len();
    let callee_start = before[..callee_end]
        .rfind(|character: char| {
            !character.is_ascii_alphanumeric() && character != '_' && character != '.'
        })
        .map(|index| index + 1)
        .unwrap_or(0);
    let callee = before[callee_start..callee_end].to_string();
    if callee.is_empty() {
        return None;
    }
    let mut nesting = 0usize;
    let mut active = 0u32;
    for character in before[open + 1..].chars() {
        match character {
            '(' | '[' | '{' => nesting += 1,
            ')' | ']' | '}' => nesting = nesting.saturating_sub(1),
            ',' if nesting == 0 => active += 1,
            _ => {}
        }
    }
    Some((callee, active))
}

pub fn lookup<'a>(
    analysis: &'a DocumentAnalysis,
    name: &str,
    qualifier: Option<&str>,
) -> Option<SymbolLookup<'a>> {
    if let Some(qualifier) = qualifier {
        let target = analysis.symbols.get(qualifier)?;
        let member = target
            .type_info
            .members()
            .into_iter()
            .find(|symbol| symbol.name == name)?;
        return Some(SymbolLookup::Owned(member));
    }
    if let Some(symbol) = analysis.symbols.get(name) {
        return Some(SymbolLookup::Borrowed(symbol));
    }
    builtin(name).map(SymbolLookup::Builtin)
}

pub enum SymbolLookup<'a> {
    Borrowed(&'a Symbol),
    Owned(Symbol),
    Builtin(&'static BuiltinInfo),
}

impl SymbolLookup<'_> {
    pub fn type_info(&self) -> TypeInfo {
        match self {
            Self::Borrowed(symbol) => symbol.type_info.clone(),
            Self::Owned(symbol) => symbol.type_info.clone(),
            Self::Builtin(info) => TypeInfo::Function(Box::new(info.function())),
        }
    }

    pub fn signature(&self) -> Option<String> {
        match self {
            Self::Borrowed(symbol) => symbol.signature.clone(),
            Self::Owned(symbol) => symbol.signature.clone(),
            Self::Builtin(info) => Some(info.signature.clone()),
        }
    }

    pub fn documentation(&self) -> Option<String> {
        match self {
            Self::Borrowed(symbol) => symbol.documentation.clone(),
            Self::Owned(symbol) => symbol.documentation.clone(),
            Self::Builtin(info) => Some(info.summary.clone()),
        }
    }

    pub fn symbol(&self) -> Option<&Symbol> {
        match self {
            Self::Borrowed(symbol) => Some(symbol),
            Self::Owned(symbol) => Some(symbol),
            Self::Builtin(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analyze(source: &str) -> DocumentAnalysis {
        analyze_source(source, None, &mut NoModules)
    }

    #[test]
    fn infers_record_fields_and_forwarded_function_shapes() {
        let result = analyze(
            r#"
fn summary(x) { {count: len(x), ready: true} }
let stats = summary([1, 2, 3])
"#,
        );
        let fields = result
            .symbols
            .get("stats")
            .unwrap()
            .type_info
            .members()
            .into_iter()
            .map(|field| field.name)
            .collect::<Vec<_>>();
        assert_eq!(fields, vec!["count", "ready"]);
    }

    #[test]
    fn detects_member_and_signature_context() {
        assert_eq!(member_target("sc.sum", 6).as_deref(), Some("sc"));
        assert_eq!(
            call_at("sc.summary(cells, ", 18),
            Some(("sc.summary".into(), 1))
        );
    }

    #[test]
    fn builtin_metadata_includes_signatures() {
        let pretty = builtin("json_pretty").unwrap();
        assert!(pretty.signature.starts_with("json_pretty("));
    }

    #[test]
    fn resolves_singlecell_exports_and_summary_shape() {
        let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let source_path = repository.join("examples/intellisense-check.bl");
        let mut loader = FileModuleLoader::new(Some(repository.join("packages")));
        let result = analyze_source(
            "import \"singlecell\" as sc\nlet stats = sc.summary(cells)\n",
            Some(&source_path),
            &mut loader,
        );
        let module_members = result
            .symbols
            .get("sc")
            .unwrap()
            .type_info
            .members()
            .into_iter()
            .map(|member| member.name)
            .collect::<Vec<_>>();
        assert!(module_members.contains(&"load".to_string()));
        assert!(module_members.contains(&"summary".to_string()));
        assert!(!module_members.iter().any(|name| name.starts_with('_')));

        let summary_fields = result
            .symbols
            .get("stats")
            .unwrap()
            .type_info
            .members()
            .into_iter()
            .map(|member| member.name)
            .collect::<Vec<_>>();
        assert!(summary_fields.contains(&"n_cells".to_string()));
        assert!(summary_fields.contains(&"has_clusters".to_string()));
    }
}
