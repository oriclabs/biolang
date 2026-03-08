use std::collections::HashMap;

use crate::error::{ApiError, Result};

/// Shared HTTP client with timeout, user-agent, and API key management.
pub struct BaseClient {
    agent: ureq::Agent,
    api_keys: HashMap<String, String>,
}

impl BaseClient {
    /// Create a new client, auto-reading known API keys from env vars.
    pub fn new() -> Self {
        let mut builder = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("bio-apis/0.1");
        if let Some(proxy) = proxy_from_env() {
            builder = builder.proxy(proxy);
        }
        let agent = builder.build();

        let mut api_keys = HashMap::new();
        for var in &["NCBI_API_KEY", "COSMIC_API_KEY"] {
            if let Ok(val) = std::env::var(var) {
                if !val.is_empty() {
                    api_keys.insert(var.to_string(), val);
                }
            }
        }

        BaseClient { agent, api_keys }
    }

    /// Store an API key by name.
    pub fn set_api_key(&mut self, name: &str, value: String) {
        self.api_keys.insert(name.to_string(), value);
    }

    /// Retrieve an API key by name.
    pub fn get_api_key(&self, name: &str) -> Option<&str> {
        self.api_keys.get(name).map(|s| s.as_str())
    }

    /// HTTP GET returning the response body as a string.
    pub fn get_text(&self, url: &str) -> Result<String> {
        let resp = self
            .agent
            .get(url)
            .set("Accept-Encoding", "identity")
            .call()
            .map_err(|e| map_ureq_error(e, url))?;
        resp.into_string()
            .map_err(|e| ApiError::Parse {
                context: url.to_string(),
                source: e.to_string(),
            })
    }

    /// HTTP GET returning parsed JSON.
    pub fn get_json(&self, url: &str) -> Result<serde_json::Value> {
        let text = self.get_text(url)?;
        serde_json::from_str(&text).map_err(|e| ApiError::Parse {
            context: url.to_string(),
            source: e.to_string(),
        })
    }

    /// HTTP GET with custom headers, returning parsed JSON.
    pub fn get_json_with_headers(
        &self,
        url: &str,
        headers: &[(&str, &str)],
    ) -> Result<serde_json::Value> {
        let mut req = self.agent.get(url).set("Accept-Encoding", "identity");
        for (k, v) in headers {
            req = req.set(k, v);
        }
        let resp = req.call().map_err(|e| map_ureq_error(e, url))?;
        let text = resp.into_string().map_err(|e| ApiError::Parse {
            context: url.to_string(),
            source: e.to_string(),
        })?;
        serde_json::from_str(&text).map_err(|e| ApiError::Parse {
            context: url.to_string(),
            source: e.to_string(),
        })
    }

    /// HTTP GET with custom headers, returning raw text.
    pub fn get_text_with_headers(
        &self,
        url: &str,
        headers: &[(&str, &str)],
    ) -> Result<String> {
        let mut req = self.agent.get(url).set("Accept-Encoding", "identity");
        for (k, v) in headers {
            req = req.set(k, v);
        }
        let resp = req.call().map_err(|e| map_ureq_error(e, url))?;
        resp.into_string().map_err(|e| ApiError::Parse {
            context: url.to_string(),
            source: e.to_string(),
        })
    }

    /// HTTP POST with JSON body, returning parsed JSON.
    pub fn post_json(
        &self,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let resp = self
            .agent
            .post(url)
            .set("Content-Type", "application/json")
            .send_string(&body.to_string())
            .map_err(|e| map_ureq_error(e, url))?;
        let text = resp.into_string().map_err(|e| ApiError::Parse {
            context: url.to_string(),
            source: e.to_string(),
        })?;
        serde_json::from_str(&text).map_err(|e| ApiError::Parse {
            context: url.to_string(),
            source: e.to_string(),
        })
    }

    /// HTTP POST with form-encoded body, returning raw text.
    pub fn post_form(&self, url: &str, params: &[(&str, &str)]) -> Result<String> {
        let mut req = self.agent.post(url);
        // Build form-encoded body
        let body: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", urlencod(k), urlencod(v)))
            .collect::<Vec<_>>()
            .join("&");
        req = req.set("Content-Type", "application/x-www-form-urlencoded");
        let resp = req
            .send_string(&body)
            .map_err(|e| map_ureq_error(e, url))?;
        resp.into_string().map_err(|e| ApiError::Parse {
            context: url.to_string(),
            source: e.to_string(),
        })
    }
}

impl Default for BaseClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Read proxy settings from environment variables.
/// Checks ALL_PROXY, HTTPS_PROXY, HTTP_PROXY in order (case-insensitive).
fn proxy_from_env() -> Option<ureq::Proxy> {
    for var in &[
        "ALL_PROXY", "all_proxy",
        "HTTPS_PROXY", "https_proxy",
        "HTTP_PROXY", "http_proxy",
    ] {
        if let Ok(val) = std::env::var(var) {
            if !val.is_empty() {
                if let Ok(proxy) = ureq::Proxy::new(&val) {
                    return Some(proxy);
                }
            }
        }
    }
    None
}

/// Minimal percent-encoding for form values.
fn urlencod(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            b' ' => out.push('+'),
            _ => {
                out.push('%');
                out.push_str(&format!("{b:02X}"));
            }
        }
    }
    out
}

/// Map ureq errors to our ApiError.
fn map_ureq_error(err: ureq::Error, url: &str) -> ApiError {
    match err {
        ureq::Error::Status(code, resp) => {
            let body = resp.into_string().unwrap_or_default();
            if code == 429 {
                ApiError::RateLimit { retry_after: None }
            } else if code == 401 || code == 403 {
                ApiError::Auth(format!("HTTP {code} from {url}: {body}"))
            } else {
                ApiError::Http {
                    status: code,
                    url: url.to_string(),
                    body,
                }
            }
        }
        ureq::Error::Transport(t) => ApiError::Network(t.to_string()),
    }
}
