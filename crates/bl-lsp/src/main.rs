mod backend;
mod analysis;
mod diagnostics;

use backend::BioLangBackend;
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| BioLangBackend::new(client));

    Server::new(stdin, stdout, socket).serve(service).await;
}
