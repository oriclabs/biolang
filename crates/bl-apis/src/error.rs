use std::fmt;

/// Errors from bioinformatics API calls.
#[derive(Debug)]
pub enum ApiError {
    /// HTTP error with status code.
    Http {
        status: u16,
        url: String,
        body: String,
    },
    /// Network / connection error.
    Network(String),
    /// JSON or text parse error.
    Parse {
        context: String,
        source: String,
    },
    /// Authentication failure (missing or invalid key).
    Auth(String),
    /// Rate limit exceeded.
    RateLimit {
        retry_after: Option<u64>,
    },
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Http { status, url, body } => {
                write!(f, "HTTP {status} from {url}: {body}")
            }
            ApiError::Network(msg) => write!(f, "network error: {msg}"),
            ApiError::Parse { context, source } => {
                write!(f, "parse error in {context}: {source}")
            }
            ApiError::Auth(msg) => write!(f, "auth error: {msg}"),
            ApiError::RateLimit { retry_after } => {
                if let Some(secs) = retry_after {
                    write!(f, "rate limited, retry after {secs}s")
                } else {
                    write!(f, "rate limited")
                }
            }
        }
    }
}

impl std::error::Error for ApiError {}

/// Convenience alias.
pub type Result<T> = std::result::Result<T, ApiError>;
