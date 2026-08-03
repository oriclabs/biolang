use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::analysis::{
    self, DocumentAnalysis, FileModuleLoader, FunctionInfo, Symbol, SymbolKind, TypeInfo,
};
use crate::diagnostics;
use crate::occurrences;

pub struct BioLangBackend {
    client: Client,
    documents: Mutex<HashMap<Url, String>>,
    root: Mutex<Option<PathBuf>>,
}

impl BioLangBackend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Mutex::new(HashMap::new()),
            root: Mutex::new(None),
        }
    }

    async fn publish_diagnostics(&self, uri: Url, source: &str) {
        let diags = diagnostics::diagnose(source);
        self.client.publish_diagnostics(uri, diags, None).await;
    }

    fn source(&self, uri: &Url) -> Option<String> {
        self.documents.lock().ok()?.get(uri).cloned()
    }

    fn source_path(&self, uri: &Url) -> Option<PathBuf> {
        let root = self.root.lock().ok()?.clone()?;
        let path = uri.path().replace('\\', "/");
        let relative = path
            .strip_prefix("/workspace/")
            .or_else(|| path.strip_prefix("workspace/"))?;
        Some(root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR)))
    }

    fn analyze(&self, uri: &Url, source: &str) -> DocumentAnalysis {
        let root = self.root.lock().ok().and_then(|value| value.clone());
        let source_path = self.source_path(uri);
        let mut loader = FileModuleLoader::new(root);
        analysis::analyze_source(source, source_path.as_deref(), &mut loader)
    }

    fn completion_item(symbol: &Symbol) -> CompletionItem {
        let detail = symbol
            .signature
            .clone()
            .unwrap_or_else(|| symbol.type_info.display());
        CompletionItem {
            label: symbol.name.clone(),
            kind: Some(completion_kind(&symbol.kind)),
            detail: Some(detail),
            documentation: symbol.documentation.clone().map(markdown),
            insert_text: Some(symbol.name.clone()),
            sort_text: Some(format!("0_{}", symbol.name)),
            ..Default::default()
        }
    }

    fn completions_for(&self, uri: &Url, position: Position) -> Vec<CompletionItem> {
        let Some(source) = self.source(uri) else {
            return Vec::new();
        };
        let offset = position_offset(&source, position);
        let member_target = analysis::member_target(&source, offset);
        let analysis_source =
            if member_target.is_some() && source[..offset].trim_end().ends_with('.') {
                let mut parseable = source.clone();
                parseable.insert_str(offset, "__completion");
                parseable
            } else {
                source.clone()
            };
        let analysis = self.analyze(uri, &analysis_source);

        if let Some(target) = member_target {
            return analysis
                .symbols
                .get(&target)
                .map(|symbol| {
                    symbol
                        .type_info
                        .members()
                        .iter()
                        .map(Self::completion_item)
                        .collect()
                })
                .unwrap_or_default();
        }

        let mut items = Vec::new();
        for keyword in analysis::keywords() {
            items.push(CompletionItem {
                label: (*keyword).to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("BioLang keyword".into()),
                sort_text: Some(format!("2_{keyword}")),
                ..Default::default()
            });
        }
        for info in analysis::builtins() {
            items.push(CompletionItem {
                label: info.name.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(info.signature.clone()),
                documentation: Some(markdown(format!(
                    "{}\n\n_Category: {}_",
                    info.summary, info.category
                ))),
                insert_text: Some(info.name.clone()),
                sort_text: Some(format!("1_{}", info.name)),
                ..Default::default()
            });
        }
        items.extend(analysis.symbols.values().map(Self::completion_item));
        items
    }

    fn lookup_at(
        &self,
        uri: &Url,
        position: Position,
    ) -> Option<(DocumentAnalysis, String, Option<String>)> {
        let source = self.source(uri)?;
        let offset = position_offset(&source, position);
        let (word, qualifier) = analysis::word_at(&source, offset)?;
        let document = self.analyze(uri, &source);
        Some((document, word, qualifier))
    }

    fn function_at_call(&self, uri: &Url, position: Position) -> Option<(FunctionInfo, u32)> {
        let source = self.source(uri)?;
        let offset = position_offset(&source, position);
        let (callee, active_parameter) = analysis::call_at(&source, offset)?;
        let document = self.analyze(uri, &source);
        let (qualifier, name) = callee
            .rsplit_once('.')
            .map(|(qualifier, name)| (Some(qualifier), name))
            .unwrap_or((None, callee.as_str()));
        let lookup = analysis::lookup(&document, name, qualifier)?;
        match lookup.type_info() {
            TypeInfo::Function(function) => Some((*function, active_parameter)),
            _ => None,
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for BioLangBackend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let root = params
            .root_uri
            .and_then(|uri| uri.to_file_path().ok())
            .or_else(|| params.root_path.map(PathBuf::from));
        if let Ok(mut current) = self.root.lock() {
            *current = root;
        }
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".into(), ":".into(), "|".into()]),
                    resolve_provider: Some(false),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".into(), ",".into()]),
                    retrigger_characters: Some(vec![",".into()]),
                    ..Default::default()
                }),
                definition_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                document_formatting_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "BioLang Language Server".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "BioLang LSP initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        self.documents
            .lock()
            .unwrap()
            .insert(uri.clone(), text.clone());
        self.publish_diagnostics(uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if let Some(change) = params.content_changes.into_iter().last() {
            let text = change.text.clone();
            self.documents
                .lock()
                .unwrap()
                .insert(uri.clone(), text.clone());
            self.publish_diagnostics(uri, &text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents
            .lock()
            .unwrap()
            .remove(&params.text_document.uri);
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let position = params.text_document_position.position;
        let uri = params.text_document_position.text_document.uri;
        Ok(Some(CompletionResponse::Array(
            self.completions_for(&uri, position),
        )))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let Some((document, word, qualifier)) = self.lookup_at(&uri, position) else {
            return Ok(None);
        };
        let Some(lookup) = analysis::lookup(&document, &word, qualifier.as_deref()) else {
            return Ok(None);
        };
        let mut value = String::new();
        if let Some(signature) = lookup.signature() {
            value.push_str("```biolang\n");
            value.push_str(&signature);
            value.push_str("\n```");
        } else {
            value.push_str(&format!("**{word}**: `{}`", lookup.type_info().display()));
        }
        if let Some(documentation) = lookup.documentation() {
            value.push_str("\n\n");
            value.push_str(&documentation);
        }
        if let Some(preview) = lookup
            .symbol()
            .and_then(|symbol| symbol.value_preview.as_deref())
        {
            value.push_str(&format!("\n\nCurrent literal value: `{preview}`"));
        }
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value,
            }),
            range: None,
        }))
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let Some((function, active_parameter)) = self.function_at_call(&uri, position) else {
            return Ok(None);
        };
        let parameters = function
            .parameters
            .iter()
            .map(|parameter| ParameterInformation {
                label: ParameterLabel::Simple(parameter.name.clone()),
                documentation: Some(Documentation::String(format!(
                    "{}{}",
                    parameter.type_info.display(),
                    if parameter.optional {
                        " (optional)"
                    } else {
                        ""
                    }
                ))),
            })
            .collect::<Vec<_>>();
        Ok(Some(SignatureHelp {
            signatures: vec![SignatureInformation {
                label: function.signature,
                documentation: function.documentation.map(Documentation::String),
                parameters: Some(parameters),
                active_parameter: Some(
                    active_parameter.min(function.parameters.len().saturating_sub(1) as u32),
                ),
            }],
            active_signature: Some(0),
            active_parameter: Some(active_parameter),
        }))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let Some((document, word, qualifier)) = self.lookup_at(&uri, position) else {
            return Ok(None);
        };
        let Some(lookup) = analysis::lookup(&document, &word, qualifier.as_deref()) else {
            return Ok(None);
        };
        let Some(symbol) = lookup.symbol() else {
            return Ok(None);
        };
        let current_source = self.source_path(&uri);
        let target_uri = if symbol.source.as_ref() == current_source.as_ref() {
            uri.clone()
        } else {
            symbol
                .source
                .as_ref()
                .and_then(|path| Url::from_file_path(path).ok())
                .unwrap_or_else(|| uri.clone())
        };
        let source = symbol
            .source
            .as_ref()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .or_else(|| self.source(&target_uri))
            .unwrap_or_default();
        Ok(Some(GotoDefinitionResponse::Scalar(Location {
            uri: target_uri,
            range: span_range(&source, symbol.span),
        })))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let Some(source) = self.source(&uri) else {
            return Ok(None);
        };
        let analysis = self.analyze(&uri, &source);
        let symbols = analysis
            .symbols
            .values()
            .filter(|symbol| symbol.kind != SymbolKind::Parameter)
            .map(|symbol| {
                #[allow(deprecated)]
                DocumentSymbol {
                    name: symbol.name.clone(),
                    detail: Some(
                        symbol
                            .signature
                            .clone()
                            .unwrap_or_else(|| symbol.type_info.display()),
                    ),
                    kind: symbol_kind(&symbol.kind),
                    tags: None,
                    deprecated: None,
                    range: span_range(&source, symbol.span),
                    selection_range: span_range(&source, symbol.span),
                    children: None,
                }
            })
            .collect();
        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let Some(source) = self.source(&uri) else {
            return Ok(None);
        };
        let offset = position_offset(&source, position);
        let Some((word, _)) = analysis::word_at(&source, offset) else {
            return Ok(None);
        };
        let locations = occurrences::identifier_occurrences(&source, &word)
            .into_iter()
            .map(|found| Location {
                uri: uri.clone(),
                range: Range {
                    start: offset_position(&source, found.start),
                    end: offset_position(&source, found.end),
                },
            })
            .collect();
        Ok(Some(locations))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let Some(source) = self.source(&uri) else {
            return Ok(None);
        };
        let offset = position_offset(&source, params.position);
        let Some((word, qualifier)) = analysis::word_at(&source, offset) else {
            return Ok(None);
        };
        // Only rename things this document defines. Builtins and package
        // exports live in files this edit could not reach, so offering to
        // rename them would produce a document that no longer compiles.
        let document = self.analyze(&uri, &source);
        if qualifier.is_some() || !document.symbols.contains_key(&word) {
            return Ok(None);
        }
        let Some(found) = occurrences::identifier_occurrences(&source, &word)
            .into_iter()
            .find(|found| found.start <= offset && offset <= found.end)
        else {
            return Ok(None);
        };
        Ok(Some(PrepareRenameResponse::Range(Range {
            start: offset_position(&source, found.start),
            end: offset_position(&source, found.end),
        })))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let Some(source) = self.source(&uri) else {
            return Ok(None);
        };
        let offset = position_offset(&source, position);
        let Some((word, qualifier)) = analysis::word_at(&source, offset) else {
            return Ok(None);
        };
        let document = self.analyze(&uri, &source);
        if qualifier.is_some() || !document.symbols.contains_key(&word) {
            return Ok(None);
        }
        let edits: Vec<TextEdit> = occurrences::identifier_occurrences(&source, &word)
            .into_iter()
            .map(|found| TextEdit {
                range: Range {
                    start: offset_position(&source, found.start),
                    end: offset_position(&source, found.end),
                },
                new_text: params.new_name.clone(),
            })
            .collect();
        if edits.is_empty() {
            return Ok(None);
        }
        Ok(Some(WorkspaceEdit {
            changes: Some(HashMap::from([(uri, edits)])),
            ..Default::default()
        }))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let Some(source) = self.source(&uri) else {
            return Ok(None);
        };
        let options = bl_fmt::FormatOptions {
            indent_width: (params.options.tab_size as usize).clamp(1, 16),
            ..bl_fmt::FormatOptions::default()
        };
        let formatted = bl_fmt::format_source(&source, options);
        if formatted == source {
            return Ok(Some(Vec::new()));
        }
        // One edit replacing the whole document: the formatter reflows layout
        // globally, so a minimal diff would be more code for the same result.
        Ok(Some(vec![TextEdit {
            range: Range {
                start: Position::new(0, 0),
                end: offset_position(&source, source.len()),
            },
            new_text: formatted,
        }]))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let Some(source) = self.source(&uri) else {
            return Ok(None);
        };
        let offset = position_offset(&source, params.range.start);
        let Some((word, qualifier)) = analysis::word_at(&source, offset) else {
            return Ok(None);
        };
        let document = self.analyze(&uri, &source);
        if analysis::lookup(&document, &word, qualifier.as_deref()).is_some() {
            return Ok(None);
        }

        let mut actions: Vec<CodeActionOrCommand> = Vec::new();
        let Some(found) = occurrences::identifier_occurrences(&source, &word)
            .into_iter()
            .find(|found| found.start <= offset && offset <= found.end)
        else {
            return Ok(Some(actions));
        };
        let replacement = Range {
            start: offset_position(&source, found.start),
            end: offset_position(&source, found.end),
        };

        for suggestion in spelling_suggestions(&document, &word) {
            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: format!("Change to `{suggestion}`"),
                kind: Some(CodeActionKind::QUICKFIX),
                edit: Some(WorkspaceEdit {
                    changes: Some(HashMap::from([(
                        uri.clone(),
                        vec![TextEdit {
                            range: replacement,
                            new_text: suggestion,
                        }],
                    )])),
                    ..Default::default()
                }),
                ..Default::default()
            }));
        }
        Ok(Some(actions))
    }
}

/// Names close enough to `word` to be a plausible typo, best match first.
///
/// Only offered for identifiers that resolve to nothing, so the worst case is a
/// suggestion the author ignores rather than a rewrite of working code.
fn spelling_suggestions(document: &DocumentAnalysis, word: &str) -> Vec<String> {
    let allowance = match word.len() {
        0..=3 => 1,
        4..=7 => 2,
        _ => 3,
    };
    let candidates = document
        .symbols
        .keys()
        .cloned()
        .chain(analysis::builtins().map(|builtin| builtin.name.to_string()));

    let mut scored: Vec<(usize, String)> = candidates
        .filter(|candidate| candidate != word)
        .filter_map(|candidate| {
            let distance = edit_distance(word, &candidate);
            (distance <= allowance).then_some((distance, candidate))
        })
        .collect();
    scored.sort();
    scored.dedup_by(|left, right| left.1 == right.1);
    scored.into_iter().take(3).map(|(_, name)| name).collect()
}

/// Levenshtein distance over two ASCII-ish identifiers.
fn edit_distance(left: &str, right: &str) -> usize {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    if left.is_empty() {
        return right.len();
    }
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    let mut current = vec![0usize; right.len() + 1];
    for (i, left_char) in left.iter().enumerate() {
        current[0] = i + 1;
        for (j, right_char) in right.iter().enumerate() {
            let substitution = previous[j] + usize::from(left_char != right_char);
            current[j + 1] = substitution.min(previous[j + 1] + 1).min(current[j] + 1);
        }
        std::mem::swap(&mut previous, &mut current);
    }
    previous[right.len()]
}

fn completion_kind(kind: &SymbolKind) -> CompletionItemKind {
    match kind {
        SymbolKind::Function => CompletionItemKind::FUNCTION,
        SymbolKind::Variable | SymbolKind::Parameter => CompletionItemKind::VARIABLE,
        SymbolKind::Import => CompletionItemKind::MODULE,
        SymbolKind::Enum => CompletionItemKind::ENUM,
        SymbolKind::Struct => CompletionItemKind::STRUCT,
        SymbolKind::Field => CompletionItemKind::FIELD,
    }
}

fn symbol_kind(kind: &SymbolKind) -> tower_lsp::lsp_types::SymbolKind {
    match kind {
        SymbolKind::Function => tower_lsp::lsp_types::SymbolKind::FUNCTION,
        SymbolKind::Variable | SymbolKind::Parameter => tower_lsp::lsp_types::SymbolKind::VARIABLE,
        SymbolKind::Import => tower_lsp::lsp_types::SymbolKind::MODULE,
        SymbolKind::Enum => tower_lsp::lsp_types::SymbolKind::ENUM,
        SymbolKind::Struct => tower_lsp::lsp_types::SymbolKind::STRUCT,
        SymbolKind::Field => tower_lsp::lsp_types::SymbolKind::FIELD,
    }
}

fn markdown(value: impl Into<String>) -> Documentation {
    Documentation::MarkupContent(MarkupContent {
        kind: MarkupKind::Markdown,
        value: value.into(),
    })
}

fn position_offset(source: &str, position: Position) -> usize {
    let mut offset = 0usize;
    for (line_index, line) in source.split_inclusive('\n').enumerate() {
        if line_index == position.line as usize {
            let mut units = 0u32;
            for (index, character) in line.char_indices() {
                if units >= position.character {
                    return offset + index;
                }
                units += character.len_utf16() as u32;
            }
            return offset + line.trim_end_matches(['\r', '\n']).len();
        }
        offset += line.len();
    }
    source.len()
}

fn span_range(source: &str, span: bl_core::span::Span) -> Range {
    Range {
        start: offset_position(source, span.start),
        end: offset_position(source, span.end),
    }
}

fn offset_position(source: &str, offset: usize) -> Position {
    let offset = offset.min(source.len());
    let before = &source[..offset];
    let line = before.bytes().filter(|byte| *byte == b'\n').count() as u32;
    let current = before
        .rsplit_once('\n')
        .map(|(_, value)| value)
        .unwrap_or(before);
    Position {
        line,
        character: current.encode_utf16().count() as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf16_position_offsets_round_trip() {
        let source = "let gene = \"β\"\nprintln(gene)\n";
        let offset = position_offset(
            source,
            Position {
                line: 1,
                character: 8,
            },
        );
        assert_eq!(&source[offset..offset + 4], "gene");
        assert_eq!(offset_position(source, offset).line, 1);
    }
}
