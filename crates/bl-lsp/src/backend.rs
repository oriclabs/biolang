use std::collections::HashMap;
use std::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::analysis;
use crate::diagnostics;

pub struct BioLangBackend {
    client: Client,
    documents: Mutex<HashMap<Url, String>>,
}

impl BioLangBackend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            documents: Mutex::new(HashMap::new()),
        }
    }

    async fn publish_diagnostics(&self, uri: Url, source: &str) {
        let diags = diagnostics::diagnose(source);
        self.client
            .publish_diagnostics(uri, diags, None)
            .await;
    }

    fn completions_for(&self, uri: &Url) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Keywords
        for kw in analysis::keywords() {
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            });
        }

        // Builtins
        for name in analysis::builtin_names() {
            items.push(CompletionItem {
                label: name,
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some("builtin".into()),
                ..Default::default()
            });
        }

        // Document symbols
        let docs = self.documents.lock().unwrap();
        if let Some(source) = docs.get(uri) {
            if let Ok(tokens) = bl_lexer::Lexer::new(source).tokenize() {
                if let Ok(result) = bl_parser::Parser::new(tokens).parse() {
                    let program = &result.program;
                    for sym in analysis::extract_symbols(&program.stmts) {
                        let kind = match sym.kind {
                            analysis::SymbolKind::Function => CompletionItemKind::FUNCTION,
                            analysis::SymbolKind::Variable => CompletionItemKind::VARIABLE,
                            analysis::SymbolKind::Parameter => CompletionItemKind::VARIABLE,
                            analysis::SymbolKind::Import => CompletionItemKind::MODULE,
                            analysis::SymbolKind::Enum => CompletionItemKind::ENUM,
                        };
                        items.push(CompletionItem {
                            label: sym.name,
                            kind: Some(kind),
                            documentation: sym.doc.map(|d| {
                                Documentation::MarkupContent(MarkupContent {
                                    kind: MarkupKind::Markdown,
                                    value: d,
                                })
                            }),
                            ..Default::default()
                        });
                    }
                }
            }
        }

        items
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for BioLangBackend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".into(), ":".into(), "|".into()]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
            ..Default::default()
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

    async fn completion(
        &self,
        params: CompletionParams,
    ) -> Result<Option<CompletionResponse>> {
        let items = self.completions_for(&params.text_document_position.text_document.uri);
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;

        let docs = self.documents.lock().unwrap();
        let source = match docs.get(uri) {
            Some(s) => s.clone(),
            None => return Ok(None),
        };
        drop(docs);

        // Extract word at cursor position
        let lines: Vec<&str> = source.lines().collect();
        let line = match lines.get(pos.line as usize) {
            Some(l) => *l,
            None => return Ok(None),
        };

        let col = pos.character as usize;
        if col >= line.len() {
            return Ok(None);
        }

        // Find word boundaries
        let start = line[..col]
            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);
        let end = line[col..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + col)
            .unwrap_or(line.len());

        let word = &line[start..end];
        if word.is_empty() {
            return Ok(None);
        }

        // Check if it's a builtin
        if analysis::is_builtin(word) {
            let info = format!("**{}** — builtin function", word);
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: info,
                }),
                range: None,
            }));
        }

        // Check document symbols
        if let Ok(tokens) = bl_lexer::Lexer::new(&source).tokenize() {
            if let Ok(result) = bl_parser::Parser::new(tokens).parse() {
                let program = &result.program;
                for sym in analysis::extract_symbols(&program.stmts) {
                    if sym.name == word {
                        let kind = format!("{:?}", sym.kind).to_lowercase();
                        let mut info = format!("**{}** — {}", word, kind);
                        if let Some(doc) = &sym.doc {
                            info.push_str(&format!("\n\n{}", doc));
                        }
                        return Ok(Some(Hover {
                            contents: HoverContents::Markup(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: info,
                            }),
                            range: None,
                        }));
                    }
                }
            }
        }

        Ok(None)
    }
}
